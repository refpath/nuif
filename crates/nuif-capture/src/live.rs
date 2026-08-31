//! Bounded live Chromium capture through the browser debugging protocol.

use crate::{
    BROWSER_CAPTURE_PROFILE, BrowserCapture, BrowserFontUse, BrowserNode, BrowserResource,
    MAX_CAPTURE_NODES, MAX_CAPTURE_RESOURCES, Viewport,
};
use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use nuif_reconstruct::provider::ProviderManifest;
use nuif_reconstruct::{Bounds, CaptureContext};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fs;
use std::io::{Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};
use tempfile::TempDir;
use thiserror::Error;
use tungstenite::client::connect_with_config;
use tungstenite::protocol::{Message, WebSocket, WebSocketConfig};
use tungstenite::stream::MaybeTlsStream;

pub const LIVE_CAPTURE_ADAPTER_VERSION: &str = "nuif-cdp-live-0";
pub const MAX_CDP_MESSAGE_BYTES: usize = 32 * 1024 * 1024;
pub const MAX_LIVE_RESOURCE_BYTES: usize = 16 * 1024 * 1024;
pub const MAX_LIVE_TOTAL_RESOURCE_BYTES: usize = 64 * 1024 * 1024;
pub const MAX_CDP_EVENTS: usize = 65_536;
pub const MAX_CDP_EVENT_BYTES: usize = 64 * 1024 * 1024;
pub const MAX_CDP_COMMANDS: u64 = 100_000;
pub const MAX_FONT_USES_PER_NODE: usize = 64;
pub const MAX_TOTAL_FONT_USES: usize = 32_768;
pub const LIVE_CONTEXT_PROFILE: &str = "nuif-browser-runtime-context-0";
pub const LIVE_CONTEXT_LOCALE: &str = "en-US";
pub const LIVE_CONTEXT_TIMEZONE: &str = "UTC";
const BROWSER_START_TIMEOUT: Duration = Duration::from_secs(10);
const CDP_IO_TIMEOUT: Duration = Duration::from_secs(10);
const LIVE_CAPTURE_TIMEOUT: Duration = Duration::from_secs(30);
const HTTP_DISCOVERY_LIMIT: usize = 1024 * 1024;
const CDP_QUIET_PERIOD: Duration = Duration::from_millis(50);
const MAX_SCREENSHOT_STABILITY_ATTEMPTS: usize = 4;
const BROWSER_STOP_GRACE: Duration = Duration::from_secs(2);

#[derive(Clone, Copy, Debug)]
pub struct LiveBrowserCanaries<'a> {
    pub cookie: &'a str,
    pub storage: &'a str,
    pub authorization: &'a str,
    pub header: &'a str,
    pub probe_path: &'a str,
}

#[derive(Clone, Copy, Debug)]
pub struct LiveBrowserOptions<'a> {
    pub chrome: &'a Path,
    pub source_url: &'a str,
    pub capture_id: &'a str,
    pub provider_manifest: &'a ProviderManifest,
    pub viewport: Viewport,
    pub canaries: Option<LiveBrowserCanaries<'a>>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LiveBrowserEvidence {
    pub capture: BrowserCapture,
    pub screenshot_png: Vec<u8>,
    pub browser_product: String,
    pub protocol_version: String,
    pub font_status: String,
    pub canary_probe_completed: bool,
}

/// Captures one source URL in an isolated pinned Chromium process.
///
/// The adapter reads DOM snapshot, resolved geometry/style, platform font use,
/// accessibility, response bodies and a viewport screenshot. It never requests
/// cookie, storage or request-header values from CDP.
///
/// # Errors
///
/// Returns a typed error when the browser cannot start, CDP violates the
/// bounded contract, navigation fails, or captured data exceeds its limits.
pub fn capture_chromium(
    options: &LiveBrowserOptions<'_>,
) -> Result<LiveBrowserEvidence, LiveCaptureError> {
    validate_options(options)?;
    let provider = options
        .provider_manifest
        .identity()
        .map_err(|_| LiveCaptureError::InvalidOptions)?;
    let (mut browser, port) = BrowserProcess::launch(options)?;
    let target = discover_page_target(port, &mut browser)?;
    let mut cdp = CdpClient::connect(&target.web_socket_debugger_url)?;
    enable_domains(&mut cdp, options.viewport)?;
    let navigation = cdp.command("Page.navigate", &json!({"url": options.source_url}))?;
    if let Some(error) = navigation.get("errorText").and_then(Value::as_str) {
        return Err(LiveCaptureError::Protocol(format!(
            "navigation failed: {error}"
        )));
    }
    let loader_id = required_string(&navigation, "loaderId")?.to_owned();
    cdp.wait_event_matching("Page.lifecycleEvent", |event| {
        event.pointer("/params/name").and_then(Value::as_str) == Some("load")
            && event.pointer("/params/loaderId").and_then(Value::as_str) == Some(loader_id.as_str())
    })?;
    settle_page(&mut cdp)?;
    let font_status = await_assets(&mut cdp)?;
    settle_after_fonts(&mut cdp)?;
    let canary_probe_completed = if let Some(canaries) = options.canaries {
        let probe = inject_canaries(&mut cdp, canaries)?;
        let probe_url = same_origin_url(options.source_url, canaries.probe_path)?;
        cdp.retain_response(probe_url, "text/plain".to_owned(), probe.body)?;
        probe.completed
    } else {
        false
    };
    cdp.drain_events_until_quiet()?;

    let snapshot = cdp.command(
        "DOMSnapshot.captureSnapshot",
        &json!({
            "computedStyles": ["background-color", "font-family", "font-size", "line-height"],
            "includePaintOrder": true,
            "includeDOMRects": true,
        }),
    )?;
    let accessibility = cdp.command("Accessibility.getFullAXTree", &json!({}))?;
    let mut raw_nodes = parse_nodes(&snapshot, &accessibility)?;
    let mut omitted_runtime = vec![
        "event-listeners-timers-workers-worklets-and-live-media-state".to_owned(),
        "cross-origin-or-opaque-response-bodies-may-be-unavailable".to_owned(),
        "canvas-webgl-and-video-semantics-are-unavailable-pixels-remain-in-the-screenshot"
            .to_owned(),
        "source-spans-unavailable-without-source-map-correlation".to_owned(),
        "local-font-bytes-are-never-read-from-the-host".to_owned(),
        "captured-script-bytes-are-inert-package-resources".to_owned(),
    ];
    attach_platform_fonts(&mut cdp, &mut raw_nodes, &mut omitted_runtime)?;
    let nodes = normalize_nodes(raw_nodes);
    let resources = collect_resources(&mut cdp, &mut omitted_runtime)?;
    let screenshot_png = capture_stable_screenshot(&mut cdp)?;
    let version = cdp.command("Browser.getVersion", &json!({}))?;
    let browser_product = required_string(&version, "product")?.to_owned();
    let protocol_version = required_string(&version, "protocolVersion")?.to_owned();
    let context = live_context(&browser_product, &protocol_version, options.viewport);
    let _ = cdp.command("Browser.close", &json!({}));
    cdp.close();
    browser.stop()?;

    Ok(LiveBrowserEvidence {
        capture: BrowserCapture {
            schema_version: 1,
            profile: BROWSER_CAPTURE_PROFILE.to_owned(),
            capture_id: options.capture_id.to_owned(),
            adapter_version: format!(
                "{LIVE_CAPTURE_ADAPTER_VERSION};{browser_product};cdp-{protocol_version}"
            ),
            provider,
            provider_manifests: vec![options.provider_manifest.clone()],
            source_url: strip_url_secrets(options.source_url)?.to_owned(),
            viewport: options.viewport,
            context: Some(context),
            nodes,
            resources,
            omitted_runtime,
        },
        screenshot_png,
        browser_product,
        protocol_version,
        font_status,
        canary_probe_completed,
    })
}

fn validate_options(options: &LiveBrowserOptions<'_>) -> Result<(), LiveCaptureError> {
    if !options.chrome.is_file()
        || !nuif_core::is_identifier(options.capture_id)
        || options.provider_manifest.validate().is_err()
        || !(options.source_url.starts_with("http://")
            || options.source_url.starts_with("https://"))
        || strip_url_secrets(options.source_url).is_err()
        || options.source_url.len() > 4_096
        || options
            .source_url
            .bytes()
            .any(|byte| byte.is_ascii_control())
        || ![
            options.viewport.width,
            options.viewport.height,
            options.viewport.device_scale_factor,
        ]
        .into_iter()
        .all(|value| value.is_finite() && value > 0.0)
        || options.viewport.width > 8_192.0
        || options.viewport.height > 8_192.0
        || options.viewport.device_scale_factor > 4.0
    {
        return Err(LiveCaptureError::InvalidOptions);
    }
    if let Some(canaries) = options.canaries
        && ([
            canaries.cookie,
            canaries.storage,
            canaries.authorization,
            canaries.header,
        ]
        .into_iter()
        .any(|value| {
            value.is_empty()
                || value.len() > 256
                || !value.is_ascii()
                || value.bytes().any(|byte| byte.is_ascii_control())
        }) || !canaries.probe_path.starts_with('/')
            || canaries.probe_path.starts_with("//")
            || canaries.probe_path.contains(['?', '#', '\\'])
            || !canaries.probe_path.is_ascii()
            || canaries
                .probe_path
                .bytes()
                .any(|byte| byte.is_ascii_control()))
    {
        return Err(LiveCaptureError::InvalidOptions);
    }
    Ok(())
}

struct BrowserProcess {
    child: Option<Child>,
    _profile: TempDir,
}

impl BrowserProcess {
    fn launch(options: &LiveBrowserOptions<'_>) -> Result<(Self, u16), LiveCaptureError> {
        let profile = tempfile::tempdir().map_err(LiveCaptureError::Io)?;
        let profile_path = profile
            .path()
            .to_str()
            .ok_or(LiveCaptureError::InvalidOptions)?;
        let width = format!("{:.0}", options.viewport.width.ceil());
        let height = format!("{:.0}", options.viewport.height.ceil());
        let mut child = Command::new(options.chrome)
            .args([
                "--headless=new",
                "--disable-background-networking",
                "--disable-component-update",
                "--disable-default-apps",
                "--disable-extensions",
                "--disable-gpu",
                "--disable-sync",
                "--force-color-profile=srgb",
                "--hide-scrollbars",
                "--metrics-recording-only",
                "--mute-audio",
                "--no-default-browser-check",
                "--no-first-run",
                "--no-sandbox",
                "--remote-debugging-port=0",
            ])
            .arg(format!("--user-data-dir={profile_path}"))
            .arg(format!("--window-size={width},{height}"))
            .arg("about:blank")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(LiveCaptureError::Io)?;
        let active_port = profile.path().join("DevToolsActivePort");
        let deadline = Instant::now() + BROWSER_START_TIMEOUT;
        let port = loop {
            if let Some(status) = child.try_wait().map_err(LiveCaptureError::Io)? {
                return Err(LiveCaptureError::BrowserExited(status.to_string()));
            }
            if let Ok(value) = fs::read_to_string(&active_port)
                && let Some(port) = value.lines().next().and_then(|line| line.parse().ok())
            {
                break port;
            }
            if Instant::now() >= deadline {
                let _ = child.kill();
                let _ = child.wait();
                return Err(LiveCaptureError::BrowserStartTimeout);
            }
            thread::sleep(Duration::from_millis(20));
        };
        Ok((
            Self {
                child: Some(child),
                _profile: profile,
            },
            port,
        ))
    }

    fn stop(&mut self) -> Result<(), LiveCaptureError> {
        let Some(mut child) = self.child.take() else {
            return Ok(());
        };
        let deadline = Instant::now() + BROWSER_STOP_GRACE;
        loop {
            if child.try_wait().map_err(LiveCaptureError::Io)?.is_some() {
                return Ok(());
            }
            if Instant::now() >= deadline {
                child.kill().map_err(LiveCaptureError::Io)?;
                child.wait().map_err(LiveCaptureError::Io)?;
                return Ok(());
            }
            thread::sleep(Duration::from_millis(10));
        }
    }
}

impl Drop for BrowserProcess {
    fn drop(&mut self) {
        if let Some(child) = &mut self.child {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

#[derive(serde::Deserialize)]
struct DebugTarget {
    #[serde(rename = "type")]
    kind: String,
    #[serde(rename = "webSocketDebuggerUrl")]
    web_socket_debugger_url: String,
}

fn discover_page_target(
    port: u16,
    browser: &mut BrowserProcess,
) -> Result<DebugTarget, LiveCaptureError> {
    let deadline = Instant::now() + BROWSER_START_TIMEOUT;
    loop {
        if let Some(child) = &mut browser.child
            && let Some(status) = child.try_wait().map_err(LiveCaptureError::Io)?
        {
            return Err(LiveCaptureError::BrowserExited(status.to_string()));
        }
        if let Ok(body) = http_get(port, "/json/list")
            && let Ok(targets) = serde_json::from_slice::<Vec<DebugTarget>>(&body)
            && let Some(target) = targets.into_iter().find(|target| target.kind == "page")
        {
            return Ok(target);
        }
        if Instant::now() >= deadline {
            return Err(LiveCaptureError::BrowserStartTimeout);
        }
        thread::sleep(Duration::from_millis(20));
    }
}

fn http_get(port: u16, path: &str) -> Result<Vec<u8>, LiveCaptureError> {
    let address = ("127.0.0.1", port)
        .to_socket_addrs()
        .map_err(LiveCaptureError::Io)?
        .next()
        .ok_or_else(|| LiveCaptureError::Protocol("debug address did not resolve".to_owned()))?;
    let mut stream =
        TcpStream::connect_timeout(&address, CDP_IO_TIMEOUT).map_err(LiveCaptureError::Io)?;
    stream
        .set_read_timeout(Some(CDP_IO_TIMEOUT))
        .map_err(LiveCaptureError::Io)?;
    stream
        .write_all(
            format!("GET {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nConnection: close\r\n\r\n")
                .as_bytes(),
        )
        .map_err(LiveCaptureError::Io)?;
    let mut response = Vec::new();
    let mut buffer = [0_u8; 8 * 1_024];
    loop {
        let read = stream.read(&mut buffer).map_err(LiveCaptureError::Io)?;
        if read == 0 {
            return Err(LiveCaptureError::Protocol(
                "debug HTTP response ended before its declared body".to_owned(),
            ));
        }
        response.extend_from_slice(&buffer[..read]);
        if response.len() > HTTP_DISCOVERY_LIMIT {
            return Err(LiveCaptureError::ResourceLimit("debug HTTP response"));
        }
        let Some(separator) = response.windows(4).position(|window| window == b"\r\n\r\n") else {
            continue;
        };
        let head = std::str::from_utf8(&response[..separator])
            .map_err(|_| LiveCaptureError::Protocol("debug HTTP header is not ASCII".to_owned()))?;
        if !head.starts_with("HTTP/1.1 200") && !head.starts_with("HTTP/1.0 200") {
            return Err(LiveCaptureError::Protocol(
                "debug HTTP request failed".to_owned(),
            ));
        }
        let content_length = head
            .lines()
            .filter_map(|line| line.split_once(':'))
            .find(|(name, _)| name.eq_ignore_ascii_case("content-length"))
            .and_then(|(_, value)| value.trim().parse::<usize>().ok())
            .ok_or_else(|| {
                LiveCaptureError::Protocol(
                    "debug HTTP response has no valid content length".to_owned(),
                )
            })?;
        if content_length > HTTP_DISCOVERY_LIMIT.saturating_sub(separator + 4) {
            return Err(LiveCaptureError::ResourceLimit("debug HTTP response"));
        }
        let body_start = separator + 4;
        let body_end = body_start + content_length;
        if response.len() >= body_end {
            return Ok(response[body_start..body_end].to_vec());
        }
    }
}

type CdpSocket = WebSocket<MaybeTlsStream<TcpStream>>;

struct RetainedResponse {
    media_type: String,
    body: Vec<u8>,
}

struct CdpClient {
    socket: CdpSocket,
    next_id: u64,
    events: VecDeque<Value>,
    responses: BTreeMap<u64, Value>,
    retained_responses: BTreeMap<String, RetainedResponse>,
    retained_response_bytes: usize,
    event_bytes: usize,
    deadline: Instant,
}

impl CdpClient {
    fn connect(url: &str) -> Result<Self, LiveCaptureError> {
        if !url.starts_with("ws://127.0.0.1:") {
            return Err(LiveCaptureError::Protocol(
                "debug WebSocket is not loopback".to_owned(),
            ));
        }
        let config = WebSocketConfig::default()
            .read_buffer_size(32 * 1024)
            .write_buffer_size(0)
            .max_write_buffer_size(1024 * 1024)
            .max_message_size(Some(MAX_CDP_MESSAGE_BYTES))
            .max_frame_size(Some(MAX_CDP_MESSAGE_BYTES));
        let (mut socket, _) = connect_with_config(url, Some(config), 0)
            .map_err(|error| LiveCaptureError::WebSocket(error.to_string()))?;
        if let MaybeTlsStream::Plain(stream) = socket.get_mut() {
            stream
                .set_read_timeout(Some(CDP_IO_TIMEOUT))
                .map_err(LiveCaptureError::Io)?;
            stream
                .set_write_timeout(Some(CDP_IO_TIMEOUT))
                .map_err(LiveCaptureError::Io)?;
        }
        Ok(Self {
            socket,
            next_id: 1,
            events: VecDeque::new(),
            responses: BTreeMap::new(),
            retained_responses: BTreeMap::new(),
            retained_response_bytes: 0,
            event_bytes: 0,
            deadline: Instant::now() + LIVE_CAPTURE_TIMEOUT,
        })
    }

    fn command(&mut self, method: &str, params: &Value) -> Result<Value, LiveCaptureError> {
        self.ensure_deadline()?;
        if self.next_id > MAX_CDP_COMMANDS {
            return Err(LiveCaptureError::ResourceLimit("CDP commands"));
        }
        let id = self.next_id;
        self.next_id = self.next_id.saturating_add(1);
        let payload = serde_json::to_string(&json!({
            "id": id,
            "method": method,
            "params": params,
        }))
        .map_err(LiveCaptureError::Json)?;
        self.socket
            .send(Message::Text(payload.into()))
            .map_err(|error| LiveCaptureError::WebSocket(error.to_string()))?;
        loop {
            if let Some(value) = self.responses.remove(&id) {
                return Self::command_result(method, &value);
            }
            let (value, encoded_bytes) = self.read_value()?;
            if let Some(response_id) = value.get("id").and_then(Value::as_u64) {
                if response_id == id {
                    return Self::command_result(method, &value);
                }
                self.store_response(response_id, value)?;
                continue;
            }
            self.push_event(value, encoded_bytes)?;
        }
    }

    fn command_result(method: &str, value: &Value) -> Result<Value, LiveCaptureError> {
        if let Some(error) = value.get("error") {
            return Err(LiveCaptureError::Protocol(format!("{method}: {error}")));
        }
        Ok(value.get("result").cloned().unwrap_or_else(|| json!({})))
    }

    fn store_response(&mut self, id: u64, value: Value) -> Result<(), LiveCaptureError> {
        if self.responses.len() >= 1_024 {
            return Err(LiveCaptureError::ResourceLimit("pending CDP responses"));
        }
        self.responses.insert(id, value);
        Ok(())
    }

    fn wait_event_matching(
        &mut self,
        method: &str,
        matches: impl Fn(&Value) -> bool,
    ) -> Result<Value, LiveCaptureError> {
        if let Some(index) = self.events.iter().position(|event| {
            event.get("method").and_then(Value::as_str) == Some(method) && matches(event)
        }) {
            return self
                .events
                .remove(index)
                .ok_or_else(|| LiveCaptureError::Protocol("event queue changed".to_owned()));
        }
        loop {
            let (value, encoded_bytes) = self.read_value()?;
            if let Some(response_id) = value.get("id").and_then(Value::as_u64) {
                self.store_response(response_id, value)?;
                continue;
            }
            if value.get("method").and_then(Value::as_str) == Some(method) && matches(&value) {
                self.account_event_bytes(encoded_bytes)?;
                return Ok(value);
            }
            self.push_event(value, encoded_bytes)?;
        }
    }

    fn read_value(&mut self) -> Result<(Value, usize), LiveCaptureError> {
        loop {
            self.ensure_deadline()?;
            let message = self
                .socket
                .read()
                .map_err(|error| LiveCaptureError::WebSocket(error.to_string()))?;
            match message {
                Message::Text(text) => {
                    let encoded_bytes = text.len();
                    let value =
                        serde_json::from_str(text.as_str()).map_err(LiveCaptureError::Json)?;
                    return Ok((value, encoded_bytes));
                }
                Message::Ping(_) | Message::Pong(_) | Message::Frame(_) => {}
                Message::Close(_) => return Err(LiveCaptureError::Closed),
                Message::Binary(_) => {
                    return Err(LiveCaptureError::Protocol(
                        "CDP sent an unexpected binary message".to_owned(),
                    ));
                }
            }
        }
    }

    fn push_event(&mut self, value: Value, encoded_bytes: usize) -> Result<(), LiveCaptureError> {
        if value.get("method").is_some() {
            self.account_event_bytes(encoded_bytes)?;
            if self.events.len() >= MAX_CDP_EVENTS {
                return Err(LiveCaptureError::ResourceLimit("CDP events"));
            }
            self.events.push_back(value);
        }
        Ok(())
    }

    fn retain_response(
        &mut self,
        url: String,
        media_type: String,
        body: Vec<u8>,
    ) -> Result<(), LiveCaptureError> {
        validate_resource_metadata(&url, &media_type)?;
        if body.len() > MAX_LIVE_RESOURCE_BYTES {
            return Err(LiveCaptureError::ResourceLimit("one retained response"));
        }
        let previous_bytes = self
            .retained_responses
            .get(&url)
            .map_or(0, |response| response.body.len());
        let retained = self
            .retained_response_bytes
            .checked_sub(previous_bytes)
            .and_then(|value| value.checked_add(body.len()))
            .ok_or(LiveCaptureError::ResourceLimit("retained response total"))?;
        if retained > MAX_LIVE_TOTAL_RESOURCE_BYTES {
            return Err(LiveCaptureError::ResourceLimit("retained response total"));
        }
        if !self.retained_responses.contains_key(&url)
            && self.retained_responses.len() >= MAX_CAPTURE_RESOURCES
        {
            return Err(LiveCaptureError::ResourceLimit("retained responses"));
        }
        self.retained_response_bytes = retained;
        self.retained_responses
            .insert(url, RetainedResponse { media_type, body });
        Ok(())
    }

    fn account_event_bytes(&mut self, encoded_bytes: usize) -> Result<(), LiveCaptureError> {
        self.event_bytes = self
            .event_bytes
            .checked_add(encoded_bytes)
            .ok_or(LiveCaptureError::ResourceLimit("CDP event bytes"))?;
        if self.event_bytes > MAX_CDP_EVENT_BYTES {
            return Err(LiveCaptureError::ResourceLimit("CDP event bytes"));
        }
        Ok(())
    }

    fn ensure_deadline(&self) -> Result<(), LiveCaptureError> {
        if Instant::now() > self.deadline {
            return Err(LiveCaptureError::CaptureTimeout);
        }
        Ok(())
    }

    fn drain_events_until_quiet(&mut self) -> Result<(), LiveCaptureError> {
        if let MaybeTlsStream::Plain(stream) = self.socket.get_mut() {
            stream
                .set_read_timeout(Some(CDP_QUIET_PERIOD))
                .map_err(LiveCaptureError::Io)?;
        }
        let outcome = loop {
            if let Err(error) = self.ensure_deadline() {
                break Err(error);
            }
            match self.socket.read() {
                Ok(Message::Text(text)) => {
                    let encoded_bytes = text.len();
                    match serde_json::from_str::<Value>(text.as_str()) {
                        Ok(value) => {
                            let result = if let Some(response_id) =
                                value.get("id").and_then(Value::as_u64)
                            {
                                self.store_response(response_id, value)
                            } else {
                                self.push_event(value, encoded_bytes)
                            };
                            if let Err(error) = result {
                                break Err(error);
                            }
                        }
                        Err(error) => break Err(LiveCaptureError::Json(error)),
                    }
                }
                Ok(Message::Ping(_) | Message::Pong(_) | Message::Frame(_)) => {}
                Ok(Message::Close(_)) => break Err(LiveCaptureError::Closed),
                Ok(Message::Binary(_)) => {
                    break Err(LiveCaptureError::Protocol(
                        "CDP sent an unexpected binary message".to_owned(),
                    ));
                }
                Err(tungstenite::Error::Io(error))
                    if matches!(
                        error.kind(),
                        std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                    ) =>
                {
                    break Ok(());
                }
                Err(error) => break Err(LiveCaptureError::WebSocket(error.to_string())),
            }
        };
        if let MaybeTlsStream::Plain(stream) = self.socket.get_mut() {
            stream
                .set_read_timeout(Some(CDP_IO_TIMEOUT))
                .map_err(LiveCaptureError::Io)?;
        }
        outcome
    }

    fn close(&mut self) {
        let _ = self.socket.close(None);
    }
}

fn enable_domains(cdp: &mut CdpClient, viewport: Viewport) -> Result<(), LiveCaptureError> {
    cdp.command("Page.enable", &json!({}))?;
    cdp.command("Runtime.enable", &json!({}))?;
    cdp.command(
        "Network.enable",
        &json!({
            "maxTotalBufferSize": MAX_LIVE_TOTAL_RESOURCE_BYTES,
            "maxResourceBufferSize": MAX_LIVE_RESOURCE_BYTES,
            "maxPostDataSize": 0,
        }),
    )?;
    cdp.command("DOM.enable", &json!({}))?;
    cdp.command("DOMSnapshot.enable", &json!({}))?;
    cdp.command("CSS.enable", &json!({}))?;
    cdp.command("Accessibility.enable", &json!({}))?;
    cdp.command("Animation.enable", &json!({}))?;
    cdp.command("Animation.setPlaybackRate", &json!({"playbackRate": 0}))?;
    cdp.command("Page.setLifecycleEventsEnabled", &json!({"enabled": true}))?;
    cdp.command(
        "Emulation.setLocaleOverride",
        &json!({"locale": LIVE_CONTEXT_LOCALE}),
    )?;
    cdp.command(
        "Emulation.setTimezoneOverride",
        &json!({"timezoneId": LIVE_CONTEXT_TIMEZONE}),
    )?;
    cdp.command(
        "Emulation.setEmulatedMedia",
        &json!({
            "media": "screen",
            "features": [
                {"name": "prefers-color-scheme", "value": "light"},
                {"name": "prefers-reduced-motion", "value": "reduce"}
            ]
        }),
    )?;
    cdp.command(
        "Emulation.setDeviceMetricsOverride",
        &json!({
            "width": viewport.width.ceil(),
            "height": viewport.height.ceil(),
            "deviceScaleFactor": viewport.device_scale_factor,
            "mobile": false,
        }),
    )?;
    Ok(())
}

fn settle_page(cdp: &mut CdpClient) -> Result<(), LiveCaptureError> {
    let result = cdp.command(
        "Runtime.evaluate",
        &json!({
            "expression": "(()=>{const s=document.createElement('style');s.setAttribute('data-nuif-capture-freeze','');s.textContent='*,*::before,*::after{animation:none!important;transition:none!important;caret-color:transparent!important}';document.documentElement.append(s);window.scrollTo(0,0);return new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(()=>resolve(window.scrollX===0&&window.scrollY===0))));})()",
            "awaitPromise": true,
            "returnByValue": true,
        }),
    )?;
    if result.pointer("/result/value").and_then(Value::as_bool) == Some(true) {
        Ok(())
    } else {
        Err(LiveCaptureError::Protocol(
            "page did not reach the declared static settling point".to_owned(),
        ))
    }
}

fn settle_after_fonts(cdp: &mut CdpClient) -> Result<(), LiveCaptureError> {
    let result = cdp.command(
        "Runtime.evaluate",
        &json!({
            "expression": "new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(()=>resolve(true))))",
            "awaitPromise": true,
            "returnByValue": true,
        }),
    )?;
    if result.pointer("/result/value").and_then(Value::as_bool) == Some(true) {
        Ok(())
    } else {
        Err(LiveCaptureError::Protocol(
            "page did not settle after font readiness".to_owned(),
        ))
    }
}

fn live_context(product: &str, protocol: &str, viewport: Viewport) -> CaptureContext {
    CaptureContext {
        profile: LIVE_CONTEXT_PROFILE.to_owned(),
        properties: BTreeMap::from([
            (
                "animation-policy".to_owned(),
                "playback-rate-zero-plus-freeze-stylesheet".to_owned(),
            ),
            ("architecture".to_owned(), std::env::consts::ARCH.to_owned()),
            ("browser-product".to_owned(), product.to_owned()),
            ("color-scheme".to_owned(), "light".to_owned()),
            (
                "device-scale-factor".to_owned(),
                viewport.device_scale_factor.to_string(),
            ),
            ("locale".to_owned(), LIVE_CONTEXT_LOCALE.to_owned()),
            ("media".to_owned(), "screen".to_owned()),
            (
                "operating-system".to_owned(),
                std::env::consts::OS.to_owned(),
            ),
            ("protocol-version".to_owned(), protocol.to_owned()),
            ("reduced-motion".to_owned(), "reduce".to_owned()),
            ("scroll".to_owned(), "0,0".to_owned()),
            (
                "settling-policy".to_owned(),
                "load-freeze-assets-ready-event-quiet-in-page-probe-stable-screenshot".to_owned(),
            ),
            ("timezone".to_owned(), LIVE_CONTEXT_TIMEZONE.to_owned()),
            ("viewport-height".to_owned(), viewport.height.to_string()),
            ("viewport-width".to_owned(), viewport.width.to_string()),
        ]),
    }
}

fn await_assets(cdp: &mut CdpClient) -> Result<String, LiveCaptureError> {
    let result = cdp.command(
        "Runtime.evaluate",
        &json!({
            "expression": "Promise.all(Array.from(document.images,image=>image.decode().catch(()=>false))).then(()=>document.fonts.ready).then(()=>document.fonts.status)",
            "awaitPromise": true,
            "returnByValue": true,
        }),
    )?;
    Ok(result
        .pointer("/result/value")
        .and_then(Value::as_str)
        .unwrap_or("unknown")
        .to_owned())
}

struct CanaryProbe {
    completed: bool,
    body: Vec<u8>,
}

fn inject_canaries(
    cdp: &mut CdpClient,
    canaries: LiveBrowserCanaries<'_>,
) -> Result<CanaryProbe, LiveCaptureError> {
    let cookie = serde_json::to_string(canaries.cookie).map_err(LiveCaptureError::Json)?;
    let storage = serde_json::to_string(canaries.storage).map_err(LiveCaptureError::Json)?;
    let authorization =
        serde_json::to_string(canaries.authorization).map_err(LiveCaptureError::Json)?;
    let header = serde_json::to_string(canaries.header).map_err(LiveCaptureError::Json)?;
    let probe_path = serde_json::to_string(canaries.probe_path).map_err(LiveCaptureError::Json)?;
    let expression = format!(
        "(async()=>{{document.cookie='nuif_capture_canary='+encodeURIComponent({cookie})+'; SameSite=Strict';localStorage.setItem('nuif_capture_canary',{storage});const stored=localStorage.getItem('nuif_capture_canary')==={storage};const r=await fetch({probe_path},{{credentials:'same-origin',headers:{{Authorization:{authorization},'X-Nuif-Canary':{header}}}}});const body=await r.text();return{{completed:stored&&r.ok&&body==='ok',body}};}})()"
    );
    let result = cdp.command(
        "Runtime.evaluate",
        &json!({
            "expression": expression,
            "awaitPromise": true,
            "returnByValue": true,
        }),
    )?;
    let value = result
        .pointer("/result/value")
        .ok_or_else(|| LiveCaptureError::Protocol("canary probe result is absent".to_owned()))?;
    let body = required_string(value, "body")?;
    if body.len() > MAX_LIVE_RESOURCE_BYTES {
        return Err(LiveCaptureError::ResourceLimit("canary probe body"));
    }
    if [
        canaries.cookie,
        canaries.storage,
        canaries.authorization,
        canaries.header,
    ]
    .into_iter()
    .any(|canary| body.contains(canary))
    {
        return Err(LiveCaptureError::Protocol(
            "canary probe reflected a credential into its body".to_owned(),
        ));
    }
    Ok(CanaryProbe {
        completed: value
            .get("completed")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        body: body.as_bytes().to_vec(),
    })
}

#[derive(Clone)]
struct RawNode {
    dom_index: usize,
    backend_node_id: u64,
    parent_dom_index: Option<usize>,
    tag: String,
    text: Option<String>,
    bounds: Bounds,
    background: Option<[f32; 4]>,
    accessible_role: Option<String>,
    accessible_name: Option<String>,
    font_uses: Vec<BrowserFontUse>,
    resource_url: Option<String>,
}

fn parse_nodes(snapshot: &Value, accessibility: &Value) -> Result<Vec<RawNode>, LiveCaptureError> {
    let strings = required_array(snapshot, "strings")?;
    let document = snapshot
        .get("documents")
        .and_then(Value::as_array)
        .and_then(|documents| documents.first())
        .ok_or_else(|| LiveCaptureError::Protocol("DOM snapshot has no document".to_owned()))?;
    let tree = document
        .get("nodes")
        .ok_or_else(|| LiveCaptureError::Protocol("DOM snapshot has no node table".to_owned()))?;
    let layout = document
        .get("layout")
        .ok_or_else(|| LiveCaptureError::Protocol("DOM snapshot has no layout table".to_owned()))?;
    let node_types = required_array(tree, "nodeType")?;
    let node_names = required_array(tree, "nodeName")?;
    let node_values = required_array(tree, "nodeValue")?;
    let backend_ids = required_array(tree, "backendNodeId")?;
    let parent_indexes = required_array(tree, "parentIndex")?;
    let layout_nodes = required_array(layout, "nodeIndex")?;
    let layout_bounds = required_array(layout, "bounds")?;
    let layout_styles = required_array(layout, "styles")?;
    let layout_text = required_array(layout, "text")?;
    let current_sources = rare_string_map(tree.get("currentSourceURL"), strings)?;
    let ax = accessibility_map(accessibility)?;
    if node_types.len() > MAX_CAPTURE_NODES {
        return Err(LiveCaptureError::ResourceLimit("DOM nodes"));
    }
    if layout_nodes.len() != layout_bounds.len()
        || layout_nodes.len() != layout_styles.len()
        || layout_nodes.len() != layout_text.len()
    {
        return Err(LiveCaptureError::Protocol(
            "DOM layout table has mismatched columns".to_owned(),
        ));
    }
    let mut layout_by_node = BTreeMap::new();
    for (layout_index, node) in layout_nodes.iter().enumerate() {
        let node = usize::try_from(value_u64(Some(node), "layout node index")?)
            .map_err(|_| LiveCaptureError::ResourceLimit("layout node index"))?;
        if layout_by_node.insert(node, layout_index).is_some() {
            return Err(LiveCaptureError::Protocol(
                "DOM layout table contains a duplicate node".to_owned(),
            ));
        }
    }
    let element_indexes = (0..node_types.len())
        .filter(|index| node_types[*index].as_u64() == Some(1))
        .filter(|index| layout_by_node.contains_key(index))
        .collect::<BTreeSet<_>>();
    let mut element_children = BTreeSet::new();
    for index in &element_indexes {
        if let Some(parent) = nearest_element_parent(*index, parent_indexes, &element_indexes) {
            element_children.insert(parent);
        }
    }
    let mut result = Vec::with_capacity(element_indexes.len());
    for &index in &element_indexes {
        let layout_index = layout_by_node[&index];
        let backend_node_id = value_u64(backend_ids.get(index), "backend node id")?;
        let tag = indexed_string(strings, node_names.get(index), "node name")?.to_ascii_lowercase();
        let bounds = parse_bounds(layout_bounds.get(layout_index))?;
        let text = (!element_children.contains(&index))
            .then(|| {
                indexed_string(strings, layout_text.get(layout_index), "layout text")
                    .ok()
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .map(str::to_owned)
            })
            .flatten()
            .or_else(|| leaf_text(index, node_types, node_values, parent_indexes, strings));
        let background = layout_styles
            .get(layout_index)
            .and_then(Value::as_array)
            .and_then(|styles| styles.first())
            .and_then(|index| indexed_string(strings, Some(index), "background style").ok())
            .and_then(parse_css_color);
        let (accessible_role, accessible_name) =
            ax.get(&backend_node_id).cloned().unwrap_or((None, None));
        result.push(RawNode {
            dom_index: index,
            backend_node_id,
            parent_dom_index: nearest_element_parent(index, parent_indexes, &element_indexes),
            tag,
            text,
            bounds,
            background,
            accessible_role,
            accessible_name,
            font_uses: Vec::new(),
            resource_url: current_sources
                .get(&index)
                .map(|url| strip_url_secrets(url).map(str::to_owned))
                .transpose()?,
        });
    }
    Ok(result)
}

fn attach_platform_fonts(
    cdp: &mut CdpClient,
    nodes: &mut [RawNode],
    omissions: &mut Vec<String>,
) -> Result<(), LiveCaptureError> {
    let mut total_font_uses = 0_usize;
    let candidates = nodes
        .iter()
        .enumerate()
        .filter(|(_, node)| node.text.is_some())
        .map(|(index, node)| (index, node.backend_node_id))
        .collect::<Vec<_>>();
    if candidates.len() > MAX_CAPTURE_NODES {
        return Err(LiveCaptureError::ResourceLimit("font-use query nodes"));
    }
    if candidates.is_empty() {
        return Ok(());
    }
    cdp.command("DOM.getDocument", &json!({"depth": 0, "pierce": true}))?;
    let backend = candidates
        .iter()
        .map(|(_, id)| Value::from(*id))
        .collect::<Vec<_>>();
    let frontend = cdp.command(
        "DOM.pushNodesByBackendIdsToFrontend",
        &json!({"backendNodeIds": backend}),
    )?;
    let frontend_ids = required_array(&frontend, "nodeIds")?;
    for ((index, _), frontend_id) in candidates.into_iter().zip(frontend_ids) {
        let Some(node_id) = frontend_id.as_u64().filter(|id| *id != 0) else {
            omissions.push(format!("platform-font-node-unavailable-{index}"));
            continue;
        };
        match cdp.command("CSS.getPlatformFontsForNode", &json!({"nodeId": node_id})) {
            Ok(result) => {
                let fonts = result
                    .get("fonts")
                    .and_then(Value::as_array)
                    .ok_or_else(|| {
                        LiveCaptureError::Protocol("platform font response is invalid".to_owned())
                    })?;
                if fonts.len() > MAX_FONT_USES_PER_NODE {
                    return Err(LiveCaptureError::ResourceLimit("font uses per node"));
                }
                total_font_uses = total_font_uses
                    .checked_add(fonts.len())
                    .ok_or(LiveCaptureError::ResourceLimit("total font uses"))?;
                if total_font_uses > MAX_TOTAL_FONT_USES {
                    return Err(LiveCaptureError::ResourceLimit("total font uses"));
                }
                nodes[index].font_uses =
                    fonts.iter().map(parse_font_use).collect::<Result<_, _>>()?;
            }
            Err(_) => omissions.push(format!("platform-font-query-unavailable-{index}")),
        }
    }
    Ok(())
}

fn parse_font_use(value: &Value) -> Result<BrowserFontUse, LiveCaptureError> {
    Ok(BrowserFontUse {
        family: bounded_metadata_string(value, "familyName", "font family")?,
        postscript_name: bounded_metadata_string(value, "postScriptName", "font PostScript name")?,
        glyph_count: u32::try_from(value_u64(value.get("glyphCount"), "glyph count")?)
            .map_err(|_| LiveCaptureError::ResourceLimit("font glyph count"))?,
        custom: value
            .get("isCustomFont")
            .and_then(Value::as_bool)
            .ok_or_else(|| LiveCaptureError::Protocol("font custom flag is absent".to_owned()))?,
    })
}

fn bounded_metadata_string(
    value: &Value,
    field: &str,
    resource: &'static str,
) -> Result<String, LiveCaptureError> {
    let value = required_string(value, field)?;
    if value.is_empty() || value.len() > 1_024 || value.bytes().any(|byte| byte.is_ascii_control())
    {
        return Err(LiveCaptureError::ResourceLimit(resource));
    }
    Ok(value.to_owned())
}

fn normalize_nodes(raw: Vec<RawNode>) -> Vec<BrowserNode> {
    let ids = raw
        .iter()
        .enumerate()
        .map(|(index, node)| (node.dom_index, u64::try_from(index + 1).unwrap_or(u64::MAX)))
        .collect::<BTreeMap<_, _>>();
    let mut orders = BTreeMap::new();
    raw.into_iter()
        .enumerate()
        .map(|(index, node)| {
            let parent = node
                .parent_dom_index
                .and_then(|parent| ids.get(&parent).copied());
            let order = orders.entry(parent).or_insert(0_u32);
            let observed_order = *order;
            *order = order.saturating_add(1);
            BrowserNode {
                backend_node_id: u64::try_from(index + 1).unwrap_or(u64::MAX),
                parent,
                order: observed_order,
                tag: node.tag,
                text: node.text,
                bounds: node.bounds,
                background: node.background,
                accessible_role: node.accessible_role,
                accessible_name: node.accessible_name,
                font_uses: node.font_uses,
                source_span: None,
                resource_url: node.resource_url,
            }
        })
        .collect()
}

#[derive(Clone)]
struct ResponseRecord {
    request_id: Option<String>,
    frame_id: Option<String>,
    url: String,
    media_type: String,
    body: Option<Vec<u8>>,
}

fn collect_resources(
    cdp: &mut CdpClient,
    omissions: &mut Vec<String>,
) -> Result<Vec<BrowserResource>, LiveCaptureError> {
    let mut responses = page_resource_records(cdp)?;
    for (url, retained) in std::mem::take(&mut cdp.retained_responses) {
        let record = responses
            .entry(url.clone())
            .or_insert_with(|| ResponseRecord {
                request_id: None,
                frame_id: None,
                url,
                media_type: retained.media_type.clone(),
                body: None,
            });
        record.media_type = retained.media_type;
        record.body = Some(retained.body);
    }
    for event in &cdp.events {
        if event.get("method").and_then(Value::as_str) != Some("Network.responseReceived") {
            continue;
        }
        let Some(request_id) = event.pointer("/params/requestId").and_then(Value::as_str) else {
            continue;
        };
        let Some(url) = event
            .pointer("/params/response/url")
            .and_then(Value::as_str)
        else {
            continue;
        };
        if !url.starts_with("http://") && !url.starts_with("https://") {
            continue;
        }
        let media_type = event
            .pointer("/params/response/mimeType")
            .and_then(Value::as_str)
            .unwrap_or("application/octet-stream");
        validate_resource_metadata(url, media_type)?;
        if request_id.len() > 1_024 || request_id.bytes().any(|byte| byte.is_ascii_control()) {
            return Err(LiveCaptureError::ResourceLimit("network request ID"));
        }
        let record = responses
            .entry(url.to_owned())
            .or_insert_with(|| ResponseRecord {
                request_id: None,
                frame_id: None,
                url: url.to_owned(),
                media_type: media_type.to_owned(),
                body: None,
            });
        record.request_id = Some(request_id.to_owned());
        media_type.clone_into(&mut record.media_type);
    }
    if responses.len() > MAX_CAPTURE_RESOURCES {
        return Err(LiveCaptureError::ResourceLimit("network resources"));
    }
    let mut resources = Vec::with_capacity(responses.len());
    let mut total = 0_usize;
    for (index, record) in responses.into_values().enumerate() {
        let mut body = record.body;
        if body.is_none()
            && let Some(request_id) = record.request_id.as_deref()
            && let Ok(result) =
                cdp.command("Network.getResponseBody", &json!({"requestId": request_id}))
        {
            body = Some(decode_protocol_body(&result, "body")?);
        }
        if body.is_none()
            && let Some(frame_id) = record.frame_id.as_deref()
            && let Ok(result) = cdp.command(
                "Page.getResourceContent",
                &json!({"frameId": frame_id, "url": record.url}),
            )
        {
            body = Some(decode_protocol_body(&result, "content")?);
        }
        let Some(body) = body else {
            omissions.push(format!("response-body-unavailable-{index}"));
            continue;
        };
        total = total.saturating_add(body.len());
        if total > MAX_LIVE_TOTAL_RESOURCE_BYTES {
            return Err(LiveCaptureError::ResourceLimit("network resource total"));
        }
        let dimensions = (record.media_type == "image/png")
            .then(|| nuif_media::inspect_png_basic_rgba8(&body).ok())
            .flatten()
            .map_or((None, None), |header| {
                (Some(header.width), Some(header.height))
            });
        resources.push(BrowserResource {
            url: strip_url_secrets(&record.url)?.to_owned(),
            final_url: strip_url_secrets(&record.url)?.to_owned(),
            media_type: record.media_type,
            body,
            intrinsic_width: dimensions.0,
            intrinsic_height: dimensions.1,
        });
    }
    Ok(resources)
}

fn page_resource_records(
    cdp: &mut CdpClient,
) -> Result<BTreeMap<String, ResponseRecord>, LiveCaptureError> {
    let result = cdp.command("Page.getResourceTree", &json!({}))?;
    let tree = result
        .get("frameTree")
        .ok_or_else(|| LiveCaptureError::Protocol("page resource tree is absent".to_owned()))?;
    let mut records = BTreeMap::new();
    let mut count = 0_usize;
    insert_page_resource_tree(tree, 0, &mut count, &mut records)?;
    Ok(records)
}

fn insert_page_resource_tree(
    tree: &Value,
    depth: usize,
    count: &mut usize,
    records: &mut BTreeMap<String, ResponseRecord>,
) -> Result<(), LiveCaptureError> {
    if depth > 64 {
        return Err(LiveCaptureError::ResourceLimit("page resource tree depth"));
    }
    let frame = tree
        .get("frame")
        .ok_or_else(|| LiveCaptureError::Protocol("page resource frame is absent".to_owned()))?;
    let frame_id = required_string(frame, "id")?;
    validate_protocol_id(frame_id, "page frame ID")?;
    if let (Some(url), Some(media_type)) = (
        frame.get("url").and_then(Value::as_str),
        frame.get("mimeType").and_then(Value::as_str),
    ) {
        insert_page_resource(url, media_type, frame_id, count, records)?;
    }
    if let Some(resources) = tree.get("resources").and_then(Value::as_array) {
        for resource in resources {
            if resource.get("failed").and_then(Value::as_bool) == Some(true)
                || resource.get("canceled").and_then(Value::as_bool) == Some(true)
            {
                continue;
            }
            insert_page_resource(
                required_string(resource, "url")?,
                required_string(resource, "mimeType")?,
                frame_id,
                count,
                records,
            )?;
        }
    }
    if let Some(children) = tree.get("childFrames").and_then(Value::as_array) {
        for child in children {
            insert_page_resource_tree(child, depth + 1, count, records)?;
        }
    }
    Ok(())
}

fn insert_page_resource(
    url: &str,
    media_type: &str,
    frame_id: &str,
    count: &mut usize,
    records: &mut BTreeMap<String, ResponseRecord>,
) -> Result<(), LiveCaptureError> {
    if !(url.starts_with("http://") || url.starts_with("https://")) {
        return Ok(());
    }
    *count = count
        .checked_add(1)
        .ok_or(LiveCaptureError::ResourceLimit("page resources"))?;
    if *count > MAX_CAPTURE_RESOURCES {
        return Err(LiveCaptureError::ResourceLimit("page resources"));
    }
    validate_resource_metadata(url, media_type)?;
    records
        .entry(url.to_owned())
        .or_insert_with(|| ResponseRecord {
            request_id: None,
            frame_id: Some(frame_id.to_owned()),
            url: url.to_owned(),
            media_type: media_type.to_owned(),
            body: None,
        });
    Ok(())
}

fn validate_resource_metadata(url: &str, media_type: &str) -> Result<(), LiveCaptureError> {
    if url.len() > 4_096 || url.bytes().any(|byte| byte.is_ascii_control()) {
        return Err(LiveCaptureError::ResourceLimit("network resource URL"));
    }
    strip_url_secrets(url)?;
    if media_type.is_empty()
        || media_type.len() > 256
        || media_type.bytes().any(|byte| byte.is_ascii_control())
    {
        return Err(LiveCaptureError::ResourceLimit("network media type"));
    }
    Ok(())
}

fn validate_protocol_id(value: &str, resource: &'static str) -> Result<(), LiveCaptureError> {
    if value.is_empty() || value.len() > 1_024 || value.bytes().any(|byte| byte.is_ascii_control())
    {
        return Err(LiveCaptureError::ResourceLimit(resource));
    }
    Ok(())
}

fn decode_protocol_body(result: &Value, field: &str) -> Result<Vec<u8>, LiveCaptureError> {
    let body = required_string(result, field)?;
    if result
        .get("base64Encoded")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        decode_base64_bounded(body, MAX_LIVE_RESOURCE_BYTES, "one network resource")
    } else if body.len() > MAX_LIVE_RESOURCE_BYTES {
        Err(LiveCaptureError::ResourceLimit("one network resource"))
    } else {
        Ok(body.as_bytes().to_vec())
    }
}

fn capture_screenshot(cdp: &mut CdpClient) -> Result<Vec<u8>, LiveCaptureError> {
    let result = cdp.command(
        "Page.captureScreenshot",
        &json!({"format": "png", "fromSurface": true, "captureBeyondViewport": false}),
    )?;
    let bytes = decode_base64_bounded(
        required_string(&result, "data")?,
        MAX_LIVE_RESOURCE_BYTES,
        "screenshot",
    )?;
    nuif_media::inspect_png_basic_rgba8(&bytes)
        .map_err(|error| LiveCaptureError::Protocol(error.to_string()))?;
    Ok(bytes)
}

fn capture_stable_screenshot(cdp: &mut CdpClient) -> Result<Vec<u8>, LiveCaptureError> {
    let mut previous = None;
    for _ in 0..MAX_SCREENSHOT_STABILITY_ATTEMPTS {
        settle_after_fonts(cdp)?;
        let current = capture_screenshot(cdp)?;
        if previous.as_ref() == Some(&current) {
            return Ok(current);
        }
        previous = Some(current);
    }
    Err(LiveCaptureError::Protocol(
        "screenshot did not stabilize within its bounded attempts".to_owned(),
    ))
}

fn decode_base64_bounded(
    encoded: &str,
    decoded_limit: usize,
    resource: &'static str,
) -> Result<Vec<u8>, LiveCaptureError> {
    let encoded_limit = decoded_limit
        .checked_add(2)
        .and_then(|value| value.checked_div(3))
        .and_then(|value| value.checked_mul(4))
        .ok_or(LiveCaptureError::ResourceLimit(resource))?;
    if encoded.len() > encoded_limit {
        return Err(LiveCaptureError::ResourceLimit(resource));
    }
    let decoded = BASE64
        .decode(encoded)
        .map_err(|error| LiveCaptureError::Protocol(error.to_string()))?;
    if decoded.len() > decoded_limit {
        return Err(LiveCaptureError::ResourceLimit(resource));
    }
    Ok(decoded)
}

fn strip_url_secrets(value: &str) -> Result<&str, LiveCaptureError> {
    let base = value.split(['?', '#']).next().unwrap_or(value);
    let authority_has_credentials = base
        .split_once("://")
        .and_then(|(_, rest)| rest.split('/').next())
        .is_some_and(|authority| authority.contains('@'));
    if base.is_empty()
        || base.len() > 4_096
        || !base.is_ascii()
        || base.bytes().any(|byte| byte.is_ascii_control())
        || authority_has_credentials
        || !(base.starts_with("http://") || base.starts_with("https://"))
    {
        return Err(LiveCaptureError::Protocol(
            "captured URL is unsafe".to_owned(),
        ));
    }
    Ok(base)
}

fn same_origin_url(source_url: &str, path: &str) -> Result<String, LiveCaptureError> {
    let source = strip_url_secrets(source_url)?;
    let authority_start = source
        .find("://")
        .map(|index| index + 3)
        .ok_or_else(|| LiveCaptureError::Protocol("source URL has no authority".to_owned()))?;
    let authority_end = source[authority_start..]
        .find('/')
        .map_or(source.len(), |index| authority_start + index);
    let joined = format!("{}{path}", &source[..authority_end]);
    strip_url_secrets(&joined)?;
    Ok(joined)
}

type AccessibilityEntry = (Option<String>, Option<String>);

fn accessibility_map(
    accessibility: &Value,
) -> Result<BTreeMap<u64, AccessibilityEntry>, LiveCaptureError> {
    let nodes = required_array(accessibility, "nodes")?;
    if nodes.len() > MAX_CAPTURE_NODES {
        return Err(LiveCaptureError::ResourceLimit("accessibility nodes"));
    }
    Ok(nodes
        .iter()
        .filter_map(|node| {
            let backend = node.get("backendDOMNodeId")?.as_u64()?;
            let role = node
                .pointer("/role/value")
                .and_then(Value::as_str)
                .filter(|role| *role != "none")
                .map(str::to_owned);
            let name = node
                .pointer("/name/value")
                .and_then(Value::as_str)
                .filter(|name| !name.is_empty())
                .map(str::to_owned);
            Some((backend, (role, name)))
        })
        .collect())
}

fn nearest_element_parent(
    index: usize,
    parents: &[Value],
    elements: &BTreeSet<usize>,
) -> Option<usize> {
    let mut parent = parents.get(index).and_then(Value::as_i64);
    let mut depth = 0_usize;
    while let Some(candidate) = parent.filter(|candidate| *candidate >= 0) {
        let candidate = usize::try_from(candidate).ok()?;
        if elements.contains(&candidate) {
            return Some(candidate);
        }
        depth = depth.saturating_add(1);
        if depth > 128 {
            return None;
        }
        parent = parents.get(candidate).and_then(Value::as_i64);
    }
    None
}

fn leaf_text(
    element: usize,
    node_types: &[Value],
    node_values: &[Value],
    parents: &[Value],
    strings: &[Value],
) -> Option<String> {
    let text = node_types
        .iter()
        .enumerate()
        .filter(|(index, value)| {
            value.as_u64() == Some(3) && nearest_raw_parent(*index, parents) == Some(element)
        })
        .filter_map(|(index, _)| indexed_string(strings, node_values.get(index), "text").ok())
        .collect::<String>();
    let text = text.trim();
    (!text.is_empty()).then(|| text.to_owned())
}

fn nearest_raw_parent(index: usize, parents: &[Value]) -> Option<usize> {
    parents
        .get(index)
        .and_then(Value::as_i64)
        .filter(|parent| *parent >= 0)
        .and_then(|parent| usize::try_from(parent).ok())
}

fn rare_string_map(
    value: Option<&Value>,
    strings: &[Value],
) -> Result<BTreeMap<usize, String>, LiveCaptureError> {
    let Some(value) = value else {
        return Ok(BTreeMap::new());
    };
    let indexes = required_array(value, "index")?;
    let values = required_array(value, "value")?;
    if indexes.len() != values.len() {
        return Err(LiveCaptureError::Protocol(
            "sparse string table has mismatched columns".to_owned(),
        ));
    }
    indexes
        .iter()
        .zip(values)
        .map(|(index, value)| {
            Ok((
                usize::try_from(value_u64(Some(index), "sparse index")?)
                    .map_err(|_| LiveCaptureError::ResourceLimit("sparse index"))?,
                indexed_string(strings, Some(value), "sparse string")?.to_owned(),
            ))
        })
        .collect()
}

fn parse_bounds(value: Option<&Value>) -> Result<Bounds, LiveCaptureError> {
    let values = value
        .and_then(Value::as_array)
        .filter(|values| values.len() == 4)
        .ok_or_else(|| LiveCaptureError::Protocol("layout bounds are invalid".to_owned()))?;
    let mut numbers = [0.0; 4];
    for (index, value) in values.iter().enumerate() {
        numbers[index] = value
            .as_f64()
            .filter(|number| number.is_finite())
            .ok_or_else(|| LiveCaptureError::Protocol("layout bound is not finite".to_owned()))?;
    }
    if numbers[2] < 0.0 || numbers[3] < 0.0 {
        return Err(LiveCaptureError::Protocol(
            "layout size is negative".to_owned(),
        ));
    }
    Ok(Bounds {
        x: numbers[0],
        y: numbers[1],
        width: numbers[2],
        height: numbers[3],
    })
}

fn parse_css_color(value: &str) -> Option<[f32; 4]> {
    if value == "transparent" {
        return Some([0.0, 0.0, 0.0, 0.0]);
    }
    let body = value
        .strip_prefix("rgba(")
        .or_else(|| value.strip_prefix("rgb("))?
        .strip_suffix(')')?;
    let values = body
        .replace([',', '/'], " ")
        .split_whitespace()
        .map(str::parse::<f32>)
        .collect::<Result<Vec<_>, _>>()
        .ok()?;
    if !(3..=4).contains(&values.len()) {
        return None;
    }
    Some([
        (values[0] / 255.0).clamp(0.0, 1.0),
        (values[1] / 255.0).clamp(0.0, 1.0),
        (values[2] / 255.0).clamp(0.0, 1.0),
        values.get(3).copied().unwrap_or(1.0).clamp(0.0, 1.0),
    ])
}

fn indexed_string<'a>(
    strings: &'a [Value],
    index: Option<&Value>,
    field: &str,
) -> Result<&'a str, LiveCaptureError> {
    let index = usize::try_from(value_u64(index, field)?)
        .map_err(|_| LiveCaptureError::ResourceLimit("string index"))?;
    strings
        .get(index)
        .and_then(Value::as_str)
        .ok_or_else(|| LiveCaptureError::Protocol(format!("{field} string index is invalid")))
}

fn required_array<'a>(value: &'a Value, field: &str) -> Result<&'a [Value], LiveCaptureError> {
    value
        .get(field)
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .ok_or_else(|| LiveCaptureError::Protocol(format!("{field} array is absent")))
}

fn required_string<'a>(value: &'a Value, field: &str) -> Result<&'a str, LiveCaptureError> {
    value
        .get(field)
        .and_then(Value::as_str)
        .ok_or_else(|| LiveCaptureError::Protocol(format!("{field} string is absent")))
}

fn value_u64(value: Option<&Value>, field: &str) -> Result<u64, LiveCaptureError> {
    value
        .and_then(Value::as_u64)
        .ok_or_else(|| LiveCaptureError::Protocol(format!("{field} integer is absent")))
}

#[derive(Debug, Error)]
pub enum LiveCaptureError {
    #[error("live capture options are invalid")]
    InvalidOptions,
    #[error("browser did not expose DevTools before the startup deadline")]
    BrowserStartTimeout,
    #[error("live capture exceeded its wall-clock deadline")]
    CaptureTimeout,
    #[error("browser exited before capture completed: {0}")]
    BrowserExited(String),
    #[error("browser DevTools connection closed")]
    Closed,
    #[error("live capture resource limit exceeded: {0}")]
    ResourceLimit(&'static str),
    #[error("browser protocol error: {0}")]
    Protocol(String),
    #[error("browser WebSocket error: {0}")]
    WebSocket(String),
    #[error("browser JSON error: {0}")]
    Json(serde_json::Error),
    #[error("browser I/O error: {0}")]
    Io(std::io::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn css_colors_are_bounded_and_explicit() {
        assert_eq!(
            parse_css_color("rgb(255, 0, 128)"),
            Some([1.0, 0.0, 128.0 / 255.0, 1.0])
        );
        assert_eq!(parse_css_color("rgba(0, 0, 0, 0)"), Some([0.0; 4]));
        assert_eq!(parse_css_color("transparent"), Some([0.0; 4]));
        assert_eq!(parse_css_color("color(display-p3 1 0 0)"), None);
    }

    #[test]
    fn normalized_node_ids_do_not_depend_on_cdp_backend_ids() {
        let raw = vec![
            RawNode {
                dom_index: 2,
                backend_node_id: 99,
                parent_dom_index: None,
                tag: "main".to_owned(),
                text: None,
                bounds: Bounds {
                    x: 0.0,
                    y: 0.0,
                    width: 10.0,
                    height: 10.0,
                },
                background: None,
                accessible_role: None,
                accessible_name: None,
                font_uses: Vec::new(),
                resource_url: None,
            },
            RawNode {
                dom_index: 8,
                backend_node_id: 501,
                parent_dom_index: Some(2),
                tag: "span".to_owned(),
                text: Some("probe".to_owned()),
                bounds: Bounds {
                    x: 1.0,
                    y: 1.0,
                    width: 4.0,
                    height: 2.0,
                },
                background: None,
                accessible_role: None,
                accessible_name: None,
                font_uses: Vec::new(),
                resource_url: None,
            },
        ];
        let normalized = normalize_nodes(raw);
        assert_eq!(normalized[0].backend_node_id, 1);
        assert_eq!(normalized[1].backend_node_id, 2);
        assert_eq!(normalized[1].parent, Some(1));
    }

    #[test]
    fn captured_urls_drop_queries_and_reject_credentials() {
        assert_eq!(
            strip_url_secrets("https://example.invalid/page?token=secret#fragment").unwrap(),
            "https://example.invalid/page"
        );
        assert_eq!(
            same_origin_url("https://example.invalid/nested/page", "/probe").unwrap(),
            "https://example.invalid/probe"
        );
        assert!(strip_url_secrets("https://user:secret@example.invalid/page").is_err());
        assert!(strip_url_secrets("file:///tmp/page.html").is_err());
        assert!(same_origin_url("file:///tmp/page.html", "/probe").is_err());
    }

    #[test]
    fn base64_is_rejected_before_unbounded_decode() {
        assert_eq!(decode_base64_bounded("b2s=", 2, "fixture").unwrap(), b"ok");
        assert!(matches!(
            decode_base64_bounded("b2s=", 1, "fixture"),
            Err(LiveCaptureError::ResourceLimit("fixture"))
        ));
    }

    #[test]
    fn platform_font_metadata_is_bounded_before_allocation() {
        let valid = json!({"familyName": "Ahem"});
        assert_eq!(
            bounded_metadata_string(&valid, "familyName", "font family").unwrap(),
            "Ahem"
        );
        let invalid = json!({"familyName": "bad\nfont"});
        assert!(matches!(
            bounded_metadata_string(&invalid, "familyName", "font family"),
            Err(LiveCaptureError::ResourceLimit("font family"))
        ));
    }
}
