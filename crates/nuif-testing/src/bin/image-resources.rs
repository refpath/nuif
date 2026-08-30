use nuif_codec::canonical_hash;
use nuif_core::{
    AffineTransform, Asset, AssetId, AssetKind, AssetPortability, CURRENT_SCHEMA_VERSION, Document,
    Entity, EntityId, EntityKind, ImageAsset, ImageCrop, ImageFit, ImagePaint, ImageSampling,
    ResourceDigest, ResourceRole, SizeIntent,
};
use nuif_layout::EvaluationContext;
use nuif_media::{
    MAX_PNG_BYTES, MAX_PNG_CHUNKS, MAX_PNG_PIXELS, MAX_PNG_WIDTH, PNG_RGBA8_PROFILE, Rgba8Image,
    decode_png_rgba8, inspect_png_rgba8,
};
use nuif_package::{NuifPackage, PackageMode};
use nuif_render::{DrawCommand, RenderTarget, build_scene, build_scene_with_resources, render_cpu};
use png::{BitDepth, ColorType, Filter, SrgbRenderingIntent};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::env;
use std::fs;
use std::io::Cursor;
use std::path::PathBuf;
use std::process::Command;
use std::time::Instant;
use zune_png::PngDecoder;
use zune_png::zune_core::bit_depth::BitDepth as ZuneBitDepth;
use zune_png::zune_core::bytestream::ZCursor;
use zune_png::zune_core::colorspace::ColorSpace as ZuneColorSpace;
use zune_png::zune_core::options::DecoderOptions;

fn main() {
    if let Err(error) = run() {
        eprintln!("image-resources: {error}");
        std::process::exit(1);
    }
}

#[expect(
    clippy::too_many_lines,
    reason = "the evidence executable keeps accepted, negative, package and render trials auditable in one flow"
)]
fn run() -> Result<(), String> {
    let output = output_path()?;
    let started = Instant::now();
    let pixels = fixture_pixels(8, 6);
    let filters = [
        ("none", Filter::NoFilter),
        ("sub", Filter::Sub),
        ("up", Filter::Up),
        ("average", Filter::Avg),
        ("paeth", Filter::Paeth),
        ("adaptive", Filter::Adaptive),
    ];
    let mut accepted = Vec::new();
    let mut primary_micros = 0_u128;
    let mut independent_micros = 0_u128;
    for (name, filter) in filters {
        for srgb in [false, true] {
            let bytes = encode_png(
                8,
                6,
                &pixels,
                ColorType::Rgba,
                BitDepth::Eight,
                filter,
                srgb,
            )?;
            let primary_started = Instant::now();
            let primary = decode_png_rgba8(&bytes).map_err(|error| error.to_string())?;
            primary_micros += primary_started.elapsed().as_micros();
            let independent_started = Instant::now();
            let independent = decode_independent(&bytes)?;
            independent_micros += independent_started.elapsed().as_micros();
            let header = inspect_png_rgba8(&bytes).map_err(|error| error.to_string())?;
            accepted.push(json!({
                "name": format!("{name}-{}", if srgb { "srgb" } else { "profile-assumed-srgb" }),
                "bytes": bytes.len(),
                "chunks": header.chunks,
                "has_srgb": header.has_srgb,
                "rgba_sha256": sha256(&primary.rgba),
                "passed": primary == independent && primary.rgba == pixels,
            }));
        }
    }

    let canonical = encode_png(
        2,
        2,
        &fixture_pixels(2, 2),
        ColorType::Rgba,
        BitDepth::Eight,
        Filter::Paeth,
        true,
    )?;
    let negative = negative_trials(&canonical)?;
    let package = package_trial(&canonical)?;
    let render = render_trials(&canonical)?;
    let passed = accepted.iter().all(passed_trial)
        && negative.iter().all(passed_trial)
        && package.iter().all(passed_trial)
        && render.iter().all(passed_trial);
    let report = json!({
        "schema_version": 1,
        "experiment": "nuif:experiment:image-resource-rgba8-baseline",
        "status": if passed { "passed" } else { "failed" },
        "profile": {
            "name": PNG_RGBA8_PROFILE,
            "primary_decoder": "png 0.18.1",
            "independent_decoder": "zune-png 0.5.2 safe options, CRC and Adler enabled",
            "encoded_color": "RGBA8 samples interpreted as encoded sRGB by the NUIF profile; optional valid sRGB intent retained",
            "alpha": "straight alpha in source; opacity applied to alpha; encoded-sRGB integer source-over at raster",
            "orientation": "identity only; Exif and all orientation metadata rejected",
            "animation": "rejected",
            "sampling": "nearest or fixed 16-bit-weight bilinear",
            "transform": "identity only in this baseline",
        },
        "limits": {
            "encoded_bytes": MAX_PNG_BYTES,
            "width": nuif_media::MAX_PNG_WIDTH,
            "height": nuif_media::MAX_PNG_HEIGHT,
            "pixels": MAX_PNG_PIXELS,
            "chunks": MAX_PNG_CHUNKS,
        },
        "measurements": {
            "accepted_decodes": accepted.len(),
            "primary_total_microseconds": primary_micros,
            "independent_total_microseconds": independent_micros,
            "total_microseconds": started.elapsed().as_micros(),
        },
        "accepted_trials": accepted,
        "negative_trials": negative,
        "package_trials": package,
        "render_trials": render,
        "source": source_identity(),
        "non_claims": [
            "no palette grayscale RGB 16-bit interlaced ICC gamma CICP Exif animation or arbitrary ancillary PNG support",
            "no non-identity image transform",
            "no GPU image sampling or cross-platform image-render reproduction yet",
            "the two decoders are independent libraries but the fixture author and harness are in this repository",
        ],
        "summary": {
            "accepted": accepted.len(),
            "negative": negative.len(),
            "package": package.len(),
            "render": render.len(),
            "blocking_failures": accepted.iter().chain(&negative).chain(&package).chain(&render).filter(|trial| !passed_trial(trial)).count(),
        }
    });
    if let Some(parent) = output.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    fs::write(
        &output,
        serde_json::to_vec_pretty(&report).map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())?;
    println!(
        "image resources: {} accepted, {} negative, {} package/render, status {}",
        accepted.len(),
        negative.len(),
        package.len() + render.len(),
        if passed { "passed" } else { "failed" }
    );
    if passed {
        Ok(())
    } else {
        Err(format!("report failed; inspect {}", output.display()))
    }
}

fn decode_independent(bytes: &[u8]) -> Result<Rgba8Image, String> {
    let profile = inspect_png_rgba8(bytes).map_err(|error| error.to_string())?;
    let options =
        DecoderOptions::new_safe()
            .set_max_width(usize::try_from(MAX_PNG_WIDTH).map_err(|error| error.to_string())?)
            .set_max_height(
                usize::try_from(nuif_media::MAX_PNG_HEIGHT).map_err(|error| error.to_string())?,
            )
            .set_use_unsafe(false)
            .inflate_set_limit(profile.decoded_bytes.saturating_add(
                usize::try_from(profile.height).map_err(|error| error.to_string())?,
            ))
            .inflate_set_confirm_adler(true)
            .png_set_confirm_crc(true)
            .png_set_decode_animated(false);
    let mut decoder = PngDecoder::new_with_options(ZCursor::new(bytes), options);
    let result = decoder.decode().map_err(|error| error.to_string())?;
    if decoder.dimensions()
        != Some((
            usize::try_from(profile.width).map_err(|error| error.to_string())?,
            usize::try_from(profile.height).map_err(|error| error.to_string())?,
        ))
        || decoder.colorspace() != Some(ZuneColorSpace::RGBA)
        || decoder.depth() != Some(ZuneBitDepth::Eight)
        || decoder.is_animated()
    {
        return Err("independent decoder changed the accepted profile".to_owned());
    }
    let rgba = result
        .u8()
        .ok_or_else(|| "independent decoder did not return u8 samples".to_owned())?;
    if rgba.len() != profile.decoded_bytes {
        return Err("independent decoder emitted the wrong byte count".to_owned());
    }
    Ok(Rgba8Image {
        width: profile.width,
        height: profile.height,
        rgba,
    })
}

fn package_trial(bytes: &[u8]) -> Result<Vec<Value>, String> {
    let mut package = NuifPackage::new(Document::empty(EntityId::new(1)), PackageMode::Portable);
    let digest = package
        .add_embedded(bytes.to_vec(), "image/png", ResourceRole::Authoring, None)
        .map_err(|error| error.to_string())?;
    let (asset, entity) = image_model(digest.clone());
    package.document.assets.insert(asset.id, asset);
    package.document.roots.push(entity.id);
    package.document.entities.insert(entity.id, entity);
    let before_hash = canonical_hash(&package.document).map_err(|error| error.to_string())?;
    let encoded = package.encode().map_err(|error| error.to_string())?;
    let mut decoded = NuifPackage::decode(&encoded).map_err(|error| error.to_string())?;
    let exact_before_edit = decoded.embedded(&digest) == Some(bytes);
    decoded
        .document
        .entities
        .get_mut(&EntityId::new(2))
        .unwrap()
        .name = Some("unrelated edit".to_owned());
    let edited = NuifPackage::decode(&decoded.encode().map_err(|error| error.to_string())?)
        .map_err(|error| error.to_string())?;
    let after_hash = canonical_hash(&edited.document).map_err(|error| error.to_string())?;
    Ok(vec![
        trial(
            "encoded_resource_survives_package_fixpoint",
            exact_before_edit && edited.embedded(&digest) == Some(bytes),
        ),
        trial(
            "unrelated_edit_changes_document_not_resource",
            before_hash != after_hash && digest.sha256_hex() == Some(sha256(bytes).as_str()),
        ),
    ])
}

fn render_trials(bytes: &[u8]) -> Result<Vec<Value>, String> {
    let digest = ResourceDigest::from_sha256_hex(sha256(bytes));
    let (asset, entity) = image_model(digest.clone());
    let mut document = Document::empty(EntityId::new(1));
    document.assets.insert(asset.id, asset);
    document.roots.push(entity.id);
    document.entities.insert(entity.id, entity);
    let context = EvaluationContext::viewport(4.0, 4.0);
    let layout = nuif_layout::evaluate(&document, &context);
    let unresolved =
        build_scene(&document, &layout, &context).map_err(|error| error.to_string())?;
    let first = build_scene_with_resources(&document, &layout, &context, |_| Some(bytes))
        .map_err(|error| error.to_string())?;
    let second = build_scene_with_resources(&document, &layout, &context, |_| Some(bytes))
        .map_err(|error| error.to_string())?;
    let first_raster = render_cpu(
        &first,
        RenderTarget {
            width: 4,
            height: 4,
            scale_factor: 1.0,
        },
    )
    .map_err(|error| error.to_string())?;
    let second_raster = render_cpu(
        &second,
        RenderTarget {
            width: 4,
            height: 4,
            scale_factor: 1.0,
        },
    )
    .map_err(|error| error.to_string())?;
    let image_command = matches!(first.commands.as_slice(), [DrawCommand::Image { .. }]);
    let repeatable = first == second && first_raster == second_raster;
    document
        .entities
        .get_mut(&EntityId::new(2))
        .and_then(|entity| entity.authored.image.as_mut())
        .unwrap()
        .transform
        .tx = 1.0;
    let unsupported_transform =
        build_scene_with_resources(&document, &layout, &context, |_| Some(bytes))
            .map_err(|error| error.to_string())?;
    Ok(vec![
        trial(
            "resolver_required",
            unresolved.commands.is_empty() && !unresolved.fidelity.is_empty(),
        ),
        trial("resolved_image_is_one_typed_command", image_command),
        trial("scene_and_raster_repeat", repeatable),
        trial(
            "non_identity_transform_fails_closed",
            unsupported_transform.commands.is_empty() && !unsupported_transform.fidelity.is_empty(),
        ),
    ])
}

fn image_model(resource: ResourceDigest) -> (Asset, Entity) {
    let id = AssetId::new(1);
    let asset = Asset {
        schema_version: CURRENT_SCHEMA_VERSION,
        id,
        name: Some("rgba8 fixture".to_owned()),
        resource: Some(resource),
        portability: AssetPortability::Portable,
        kind: AssetKind::Image(ImageAsset {
            width: 2,
            height: 2,
            decoder_profile: PNG_RGBA8_PROFILE.to_owned(),
        }),
    };
    let mut entity = Entity::new(EntityId::new(2), EntityKind::Image);
    entity.authored.width = SizeIntent::Fixed(4.0);
    entity.authored.height = SizeIntent::Fixed(4.0);
    entity.authored.image = Some(ImagePaint {
        asset: id,
        fit: ImageFit::Fill,
        crop: ImageCrop {
            x: 0.0,
            y: 0.0,
            width: 1.0,
            height: 1.0,
        },
        transform: AffineTransform {
            a: 1.0,
            b: 0.0,
            c: 0.0,
            d: 1.0,
            tx: 0.0,
            ty: 0.0,
        },
        sampling: ImageSampling::Linear,
        opacity: 0.75,
        color_conversion: "srgb".to_owned(),
    });
    (asset, entity)
}

fn negative_trials(canonical: &[u8]) -> Result<Vec<Value>, String> {
    let mut cases = Vec::new();
    for (name, color, depth, pixels) in [
        (
            "grayscale",
            ColorType::Grayscale,
            BitDepth::Eight,
            vec![0; 4],
        ),
        ("rgb", ColorType::Rgb, BitDepth::Eight, vec![0; 12]),
        ("rgba16", ColorType::Rgba, BitDepth::Sixteen, vec![0; 32]),
    ] {
        let bytes = encode_png(2, 2, &pixels, color, depth, Filter::NoFilter, false)?;
        cases.push((name, bytes));
    }
    cases.push((
        "gamma",
        insert_chunk_before(canonical, *b"IDAT", *b"gAMA", &45_455_u32.to_be_bytes())?,
    ));
    cases.push((
        "exif",
        insert_chunk_before(canonical, *b"IDAT", *b"eXIf", b"MM\0*")?,
    ));
    cases.push((
        "animation",
        insert_chunk_before(canonical, *b"IDAT", *b"acTL", &[0; 8])?,
    ));
    cases.push((
        "duplicate-srgb",
        insert_chunk_before(canonical, *b"IDAT", *b"sRGB", &[0])?,
    ));
    let mut corrupt = canonical.to_vec();
    let idat = find_chunk(&corrupt, *b"IDAT")?;
    corrupt[idat + 8] ^= 1;
    cases.push(("corrupt-crc-or-deflate", corrupt));
    let mut trailing = canonical.to_vec();
    trailing.push(0);
    cases.push(("trailing-data", trailing));
    let mut oversized = canonical.to_vec();
    oversized[16..20].copy_from_slice(&(MAX_PNG_WIDTH + 1).to_be_bytes());
    cases.push(("width-one-over", oversized));
    let mut too_many_pixels = canonical.to_vec();
    too_many_pixels[16..20].copy_from_slice(&4_097_u32.to_be_bytes());
    too_many_pixels[20..24].copy_from_slice(&4_096_u32.to_be_bytes());
    cases.push(("pixel-one-over", too_many_pixels));
    let mut chunks = Vec::new();
    for _ in 0..(MAX_PNG_CHUNKS - 3) {
        chunks.extend(make_chunk(*b"IDAT", &[]));
    }
    let iend = find_chunk(canonical, *b"IEND")?;
    let mut too_many_chunks = canonical[..iend].to_vec();
    too_many_chunks.extend(chunks);
    too_many_chunks.extend_from_slice(&canonical[iend..]);
    cases.push(("chunk-one-over", too_many_chunks));
    let mut encoded_over = vec![0; MAX_PNG_BYTES + 1];
    encoded_over[..8].copy_from_slice(b"\x89PNG\r\n\x1a\n");
    cases.push(("encoded-byte-one-over", encoded_over));

    Ok(cases
        .into_iter()
        .map(|(name, bytes)| trial(name, decode_png_rgba8(&bytes).is_err()))
        .collect())
}

fn encode_png(
    width: u32,
    height: u32,
    pixels: &[u8],
    color: ColorType,
    depth: BitDepth,
    filter: Filter,
    srgb: bool,
) -> Result<Vec<u8>, String> {
    let mut bytes = Vec::new();
    {
        let mut encoder = png::Encoder::new(Cursor::new(&mut bytes), width, height);
        encoder.set_color(color);
        encoder.set_depth(depth);
        encoder.set_filter(filter);
        if srgb {
            encoder.set_source_srgb(SrgbRenderingIntent::Perceptual);
        }
        let mut writer = encoder.write_header().map_err(|error| error.to_string())?;
        writer
            .write_image_data(pixels)
            .map_err(|error| error.to_string())?;
    }
    Ok(bytes)
}

fn fixture_pixels(width: u32, height: u32) -> Vec<u8> {
    let width = usize::try_from(width).expect("fixture width fits usize");
    let height = usize::try_from(height).expect("fixture height fits usize");
    let mut pixels = Vec::with_capacity(width * height * 4);
    for y in 0..height {
        for x in 0..width {
            pixels.extend([
                u8::try_from((x * 37 + y * 11) % 256).expect("fixture channel fits u8"),
                u8::try_from((x * 13 + y * 53) % 256).expect("fixture channel fits u8"),
                u8::try_from((x * 71 + y * 7) % 256).expect("fixture channel fits u8"),
                u8::try_from(64 + (x * 19 + y * 23) % 192).expect("fixture alpha fits u8"),
            ]);
        }
    }
    pixels
}

fn insert_chunk_before(
    bytes: &[u8],
    before: [u8; 4],
    kind: [u8; 4],
    data: &[u8],
) -> Result<Vec<u8>, String> {
    let offset = find_chunk(bytes, before)?;
    let mut output = Vec::with_capacity(bytes.len() + data.len() + 12);
    output.extend_from_slice(&bytes[..offset]);
    output.extend(make_chunk(kind, data));
    output.extend_from_slice(&bytes[offset..]);
    Ok(output)
}

fn make_chunk(kind: [u8; 4], data: &[u8]) -> Vec<u8> {
    let mut chunk = Vec::with_capacity(data.len() + 12);
    chunk.extend_from_slice(&u32::try_from(data.len()).unwrap().to_be_bytes());
    chunk.extend_from_slice(&kind);
    chunk.extend_from_slice(data);
    let mut crc_input = kind.to_vec();
    crc_input.extend_from_slice(data);
    chunk.extend_from_slice(&crc32(&crc_input).to_be_bytes());
    chunk
}

fn find_chunk(bytes: &[u8], target: [u8; 4]) -> Result<usize, String> {
    let mut offset = 8_usize;
    while offset.checked_add(12).is_some_and(|end| end <= bytes.len()) {
        let length = usize::try_from(u32::from_be_bytes(
            bytes[offset..offset + 4]
                .try_into()
                .expect("four-byte PNG fixture length"),
        ))
        .map_err(|error| error.to_string())?;
        if bytes[offset + 4..offset + 8] == target {
            return Ok(offset);
        }
        offset = offset
            .checked_add(length + 12)
            .ok_or_else(|| "PNG chunk offset overflow".to_owned())?;
    }
    Err(format!(
        "PNG fixture lacks chunk {:?}",
        String::from_utf8_lossy(&target)
    ))
}

fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = u32::MAX;
    for byte in bytes {
        crc ^= u32::from(*byte);
        for _ in 0..8 {
            crc = (crc >> 1) ^ (0xedb8_8320 & 0_u32.wrapping_sub(crc & 1));
        }
    }
    !crc
}

fn trial(name: &str, passed: bool) -> Value {
    json!({"name": name, "passed": passed})
}

fn passed_trial(value: &Value) -> bool {
    value["passed"] == true
}

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn source_identity() -> Value {
    json!({
        "revision": command_text("git", &["rev-parse", "HEAD"]),
        "dirty": command_text("git", &["status", "--porcelain"]).map(|value| !value.is_empty()),
        "toolchain": command_text("rustc", &["--version"]),
        "os": env::consts::OS,
        "architecture": env::consts::ARCH,
    })
}

fn command_text(program: &str, arguments: &[&str]) -> Option<String> {
    Command::new(program)
        .args(arguments)
        .output()
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .map(|value| value.trim().to_owned())
}

fn output_path() -> Result<PathBuf, String> {
    let mut arguments = env::args().skip(1);
    let mut output = PathBuf::from("target/image-resources-report.json");
    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--output" => {
                output = arguments
                    .next()
                    .ok_or_else(|| "--output requires a path".to_owned())?
                    .into();
            }
            "--help" | "-h" => return Err("usage: image-resources [--output <json>]".to_owned()),
            unknown => return Err(format!("unknown argument {unknown:?}")),
        }
    }
    Ok(output)
}
