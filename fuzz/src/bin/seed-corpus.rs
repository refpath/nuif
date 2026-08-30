use nuif_codec::{CanonicalText, DeterministicCbor, Encoder};
use nuif_package::{NuifPackage, PackageMode};
use nuif_render::RasterImage;
use std::env;
use std::fs;
use std::path::Path;

fn main() {
    if let Err(error) = run() {
        eprintln!("seed-corpus: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let root = env::args()
        .nth(1)
        .ok_or_else(|| "usage: seed-corpus <output-directory>".to_owned())?;
    let root = Path::new(&root);
    let document = nuif_testing::responsive_card_fixture();

    write(
        root,
        "codec_roundtrip/canonical-text.nuif.json",
        &CanonicalText.encode(&document).map_err(text)?,
    )?;
    write(
        root,
        "codec_roundtrip/deterministic-cbor.nuif.cbor",
        &DeterministicCbor.encode(&document).map_err(text)?,
    )?;

    let package = NuifPackage::new(document, PackageMode::Authoring)
        .encode()
        .map_err(text)?;
    write(root, "package_decode/profile-zero.nuif", &package)?;
    let penpot = nuif_penpot::export_document(&nuif_penpot::profile_fixture())
        .map_err(text)?
        .bytes;
    write(root, "package_decode/profile-zero.penpot", &penpot)?;

    let png = RasterImage {
        width: 2,
        height: 2,
        rgba: vec![
            255, 0, 0, 255, 0, 255, 0, 128, 0, 0, 255, 64, 255, 255, 255, 0,
        ],
    }
    .to_png()
    .map_err(text)?;
    write_prefixed(root, "resource_decoders/rgba8.png", 0, &png)?;
    write_prefixed(
        root,
        "resource_decoders/ahem.ttf",
        1,
        nuif_text::pinned_font_bytes(),
    )?;

    let adapters = [
        nuif_html::export_document(&nuif_html::profile_fixture())
            .map_err(text)?
            .source,
        nuif_svg::export_document(&nuif_svg::profile_fixture())
            .map_err(text)?
            .source,
        nuif_dtcg::export_document(&nuif_dtcg::profile_fixture())
            .map_err(text)?
            .source,
        nuif_react::export_document(&nuif_react::profile_fixture())
            .map_err(text)?
            .source,
        nuif_svelte::export_document(&nuif_svelte::profile_fixture())
            .map_err(text)?
            .source,
    ];
    for (index, source) in adapters.iter().enumerate() {
        write_prefixed(
            root,
            &format!("adapter_import/profile-{index}.source"),
            u8::try_from(index).map_err(text)?,
            source.as_bytes(),
        )?;
    }
    write(
        root,
        "operation_sequence/scalar-choices",
        b"\x00\x01\x02\x03rename-width-height-value-sequence",
    )?;
    Ok(())
}

fn write(root: &Path, relative: &str, bytes: &[u8]) -> Result<(), String> {
    let path = root.join(relative);
    let parent = path
        .parent()
        .ok_or_else(|| format!("seed path has no parent: {}", path.display()))?;
    fs::create_dir_all(parent).map_err(text)?;
    fs::write(path, bytes).map_err(text)
}

fn write_prefixed(root: &Path, relative: &str, prefix: u8, bytes: &[u8]) -> Result<(), String> {
    let mut input = Vec::with_capacity(bytes.len() + 1);
    input.push(prefix);
    input.extend_from_slice(bytes);
    write(root, relative, &input)
}

fn text(error: impl std::fmt::Display) -> String {
    error.to_string()
}
