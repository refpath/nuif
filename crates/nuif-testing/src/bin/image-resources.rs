use nuif_codec::canonical_hash;
use nuif_core::{
    AffineTransform, Asset, AssetId, AssetKind, AssetPortability, CURRENT_SCHEMA_VERSION, Document,
    Entity, EntityId, EntityKind, ImageAsset, ImageCrop, ImageFit, ImagePaint, ImageSampling,
    ResourceDigest, ResourceRole, SizeIntent,
};
use nuif_layout::EvaluationContext;
use nuif_media::{
    MAX_PNG_BYTES, MAX_PNG_CHUNKS, MAX_PNG_PIXELS, MAX_PNG_WIDTH, PNG_BASIC_RGBA8_PROFILE,
    PNG_RGBA8_PROFILE, Rgba8Image, decode_png_basic_rgba8, decode_png_rgba8,
    inspect_png_basic_rgba8, inspect_png_rgba8,
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
    let basic = basic_profile_trials(&mut primary_micros, &mut independent_micros)?;

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
    let basic_negative = basic_negative_trials(&canonical)?;
    let package = package_trial(&canonical)?;
    let render = render_trials(&canonical)?;
    let passed = accepted.iter().all(passed_trial)
        && basic.iter().all(passed_trial)
        && negative.iter().all(passed_trial)
        && basic_negative.iter().all(passed_trial)
        && package.iter().all(passed_trial)
        && render.iter().all(passed_trial);
    let report = json!({
        "schema_version": 1,
        "experiment": "nuif:experiment:image-resource-rgba8-baseline",
        "status": if passed { "passed" } else { "failed" },
        "profiles": [{
            "name": PNG_RGBA8_PROFILE,
            "accepted": "non-interlaced RGBA8 with optional sRGB",
        }, {
            "name": PNG_BASIC_RGBA8_PROFILE,
            "accepted": "non-interlaced 1/2/4/8-bit grayscale and indexed, plus RGB8, grayscale-alpha8 and RGBA8; valid palette or color-key transparency",
        }],
        "decoder_contract": {
            "primary_decoder": "png 0.18.1",
            "independent_decoder": "zune-png 0.5.2 safe options, CRC and Adler enabled",
            "encoded_color": "samples interpreted as encoded sRGB by the selected NUIF profile; optional valid sRGB intent retained",
            "alpha": "straight alpha in source; opacity applied to alpha; encoded-sRGB integer source-over at raster",
            "orientation": "identity only; Exif and all orientation metadata rejected",
            "animation": "rejected",
            "sampling": "nearest or fixed 16-bit-weight bilinear",
            "transform": "identity only in these decoder-profile trials",
        },
        "limits": {
            "encoded_bytes": MAX_PNG_BYTES,
            "width": nuif_media::MAX_PNG_WIDTH,
            "height": nuif_media::MAX_PNG_HEIGHT,
            "pixels": MAX_PNG_PIXELS,
            "chunks": MAX_PNG_CHUNKS,
        },
        "measurements": {
            "accepted_decodes": accepted.len() + basic.len(),
            "primary_total_microseconds": primary_micros,
            "independent_total_microseconds": independent_micros,
            "total_microseconds": started.elapsed().as_micros(),
        },
        "accepted_trials": accepted,
        "basic_profile_trials": basic,
        "negative_trials": negative,
        "basic_negative_trials": basic_negative,
        "package_trials": package,
        "render_trials": render,
        "source": source_identity(),
        "non_claims": [
            "no 16-bit interlaced ICC gamma/chromaticity CICP Exif animation or arbitrary ancillary PNG support",
            "no non-identity image transform",
            "no GPU image sampling or cross-platform image-render reproduction yet",
            "the two decoders are independent libraries but the fixture author and harness are in this repository",
        ],
        "summary": {
            "accepted": accepted.len() + basic.len(),
            "negative": negative.len() + basic_negative.len(),
            "package": package.len(),
            "render": render.len(),
            "blocking_failures": accepted.iter().chain(&basic).chain(&negative).chain(&basic_negative).chain(&package).chain(&render).filter(|trial| !passed_trial(trial)).count(),
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
        accepted.len() + basic.len(),
        negative.len() + basic_negative.len(),
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

fn decode_independent_basic(bytes: &[u8]) -> Result<Rgba8Image, String> {
    let profile = inspect_png_basic_rgba8(bytes).map_err(|error| error.to_string())?;
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
        || decoder.depth() != Some(ZuneBitDepth::Eight)
        || decoder.is_animated()
    {
        return Err("independent decoder changed the accepted basic profile".to_owned());
    }
    let samples = result
        .u8()
        .ok_or_else(|| "independent basic decoder did not return u8 samples".to_owned())?;
    let color = decoder
        .colorspace()
        .ok_or_else(|| "independent basic decoder omitted colorspace".to_owned())?;
    let pixels = usize::try_from(u64::from(profile.width) * u64::from(profile.height))
        .map_err(|error| error.to_string())?;
    if samples.len() != pixels.saturating_mul(color.num_components()) {
        return Err("independent basic decoder emitted the wrong byte count".to_owned());
    }
    let mut rgba = Vec::with_capacity(profile.decoded_bytes);
    match color {
        ZuneColorSpace::RGBA => rgba.extend_from_slice(&samples),
        ZuneColorSpace::RGB => {
            for pixel in samples.as_chunks::<3>().0 {
                rgba.extend_from_slice(pixel);
                rgba.push(255);
            }
        }
        ZuneColorSpace::LumaA => {
            for pixel in samples.as_chunks::<2>().0 {
                rgba.extend([pixel[0], pixel[0], pixel[0], pixel[1]]);
            }
        }
        ZuneColorSpace::Luma => {
            for value in samples {
                rgba.extend([value, value, value, 255]);
            }
        }
        _ => return Err("independent basic decoder produced an unsupported colorspace".to_owned()),
    }
    if rgba.len() != profile.decoded_bytes {
        return Err("independent basic normalization emitted the wrong byte count".to_owned());
    }
    Ok(Rgba8Image {
        width: profile.width,
        height: profile.height,
        rgba,
    })
}

#[expect(
    clippy::too_many_lines,
    reason = "the explicit fixture table keeps every admitted color/depth case inspectable"
)]
fn basic_profile_trials(
    primary_micros: &mut u128,
    independent_micros: &mut u128,
) -> Result<Vec<Value>, String> {
    let palette = [1, 2, 3, 10, 20, 30, 40, 50, 60, 70, 80, 90];
    let cases = vec![
        (
            "grayscale-1",
            encode_basic_png(
                4,
                &[0b0101_0000],
                ColorType::Grayscale,
                BitDepth::One,
                None,
                None,
                false,
            )?,
            gray_rgba(&[0, 255, 0, 255]),
        ),
        (
            "grayscale-2",
            encode_basic_png(
                4,
                &[0b00_01_10_11],
                ColorType::Grayscale,
                BitDepth::Two,
                None,
                None,
                true,
            )?,
            gray_rgba(&[0, 85, 170, 255]),
        ),
        (
            "grayscale-4",
            encode_basic_png(
                4,
                &[0x05, 0xaf],
                ColorType::Grayscale,
                BitDepth::Four,
                None,
                None,
                false,
            )?,
            gray_rgba(&[0, 85, 170, 255]),
        ),
        (
            "grayscale-8",
            encode_basic_png(
                2,
                &[7, 23],
                ColorType::Grayscale,
                BitDepth::Eight,
                None,
                None,
                false,
            )?,
            gray_rgba(&[7, 23]),
        ),
        (
            "grayscale-8-trns",
            encode_basic_png(
                2,
                &[7, 23],
                ColorType::Grayscale,
                BitDepth::Eight,
                None,
                Some(&[0, 7]),
                false,
            )?,
            vec![7, 7, 7, 0, 23, 23, 23, 255],
        ),
        (
            "rgb-8",
            encode_basic_png(
                2,
                &[1, 2, 3, 4, 5, 6],
                ColorType::Rgb,
                BitDepth::Eight,
                None,
                None,
                true,
            )?,
            vec![1, 2, 3, 255, 4, 5, 6, 255],
        ),
        (
            "rgb-8-trns",
            encode_basic_png(
                2,
                &[1, 2, 3, 4, 5, 6],
                ColorType::Rgb,
                BitDepth::Eight,
                None,
                Some(&[0, 1, 0, 2, 0, 3]),
                false,
            )?,
            vec![1, 2, 3, 0, 4, 5, 6, 255],
        ),
        (
            "indexed-1",
            encode_basic_png(
                4,
                &[0b0101_0000],
                ColorType::Indexed,
                BitDepth::One,
                Some(&palette[..6]),
                None,
                false,
            )?,
            vec![1, 2, 3, 255, 10, 20, 30, 255, 1, 2, 3, 255, 10, 20, 30, 255],
        ),
        (
            "indexed-2-trns",
            encode_basic_png(
                4,
                &[0b00_01_10_11],
                ColorType::Indexed,
                BitDepth::Two,
                Some(&palette),
                Some(&[0, 85, 170, 255]),
                false,
            )?,
            vec![1, 2, 3, 0, 10, 20, 30, 85, 40, 50, 60, 170, 70, 80, 90, 255],
        ),
        (
            "indexed-4",
            encode_basic_png(
                4,
                &[0x01, 0x23],
                ColorType::Indexed,
                BitDepth::Four,
                Some(&palette),
                None,
                false,
            )?,
            vec![
                1, 2, 3, 255, 10, 20, 30, 255, 40, 50, 60, 255, 70, 80, 90, 255,
            ],
        ),
        (
            "indexed-8",
            encode_basic_png(
                4,
                &[0, 1, 2, 3],
                ColorType::Indexed,
                BitDepth::Eight,
                Some(&palette),
                None,
                true,
            )?,
            vec![
                1, 2, 3, 255, 10, 20, 30, 255, 40, 50, 60, 255, 70, 80, 90, 255,
            ],
        ),
        (
            "grayscale-alpha-8",
            encode_basic_png(
                2,
                &[9, 10, 11, 12],
                ColorType::GrayscaleAlpha,
                BitDepth::Eight,
                None,
                None,
                false,
            )?,
            vec![9, 9, 9, 10, 11, 11, 11, 12],
        ),
        (
            "rgba-8-compatibility",
            encode_basic_png(
                2,
                &[1, 2, 3, 4, 5, 6, 7, 8],
                ColorType::Rgba,
                BitDepth::Eight,
                None,
                None,
                true,
            )?,
            vec![1, 2, 3, 4, 5, 6, 7, 8],
        ),
    ];
    let mut trials = Vec::with_capacity(cases.len());
    for (name, bytes, expected) in cases {
        let primary_started = Instant::now();
        let primary = decode_png_basic_rgba8(&bytes).map_err(|error| error.to_string())?;
        *primary_micros += primary_started.elapsed().as_micros();
        let independent_started = Instant::now();
        let independent = decode_independent_basic(&bytes)?;
        *independent_micros += independent_started.elapsed().as_micros();
        let header = inspect_png_basic_rgba8(&bytes).map_err(|error| error.to_string())?;
        trials.push(json!({
            "name": name,
            "encoded_color": format!("{:?}", header.color_type),
            "encoded_depth": format!("{:?}", header.bit_depth),
            "has_srgb": header.has_srgb,
            "has_transparency": header.has_transparency,
            "rgba_sha256": sha256(&primary.rgba),
            "passed": primary == independent && primary.rgba == expected,
        }));
    }
    Ok(trials)
}

fn gray_rgba(values: &[u8]) -> Vec<u8> {
    values
        .iter()
        .flat_map(|value| [*value, *value, *value, 255])
        .collect()
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
    let basic_bytes = encode_basic_png(
        4,
        &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12],
        ColorType::Rgb,
        BitDepth::Eight,
        None,
        None,
        true,
    )?;
    let basic_digest = ResourceDigest::from_sha256_hex(sha256(&basic_bytes));
    let (basic_asset, basic_entity) =
        image_model_for_profile(basic_digest, PNG_BASIC_RGBA8_PROFILE, 4, 1);
    let mut basic_document = Document::empty(EntityId::new(1));
    basic_document.assets.insert(basic_asset.id, basic_asset);
    basic_document.roots.push(basic_entity.id);
    basic_document
        .entities
        .insert(basic_entity.id, basic_entity);
    let basic_layout = nuif_layout::evaluate(&basic_document, &context);
    let basic_scene = build_scene_with_resources(&basic_document, &basic_layout, &context, |_| {
        Some(&basic_bytes)
    })
    .map_err(|error| error.to_string())?;
    let basic_raster = render_cpu(
        &basic_scene,
        RenderTarget {
            width: 4,
            height: 4,
            scale_factor: 1.0,
        },
    )
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
        trial(
            "basic_profile_lowers_and_renders",
            matches!(basic_scene.commands.as_slice(), [DrawCommand::Image { decoder_profile, .. }] if decoder_profile == PNG_BASIC_RGBA8_PROFILE)
                && basic_raster.rgba.len() == 4 * 4 * 4,
        ),
    ])
}

fn image_model(resource: ResourceDigest) -> (Asset, Entity) {
    image_model_for_profile(resource, PNG_RGBA8_PROFILE, 2, 2)
}

fn image_model_for_profile(
    resource: ResourceDigest,
    decoder_profile: &str,
    width: u32,
    height: u32,
) -> (Asset, Entity) {
    let id = AssetId::new(1);
    let asset = Asset {
        schema_version: CURRENT_SCHEMA_VERSION,
        id,
        name: Some("rgba8 fixture".to_owned()),
        resource: Some(resource),
        portability: AssetPortability::Portable,
        kind: AssetKind::Image(ImageAsset {
            width,
            height,
            decoder_profile: decoder_profile.to_owned(),
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

fn basic_negative_trials(canonical: &[u8]) -> Result<Vec<Value>, String> {
    let rgba16 = encode_basic_png(
        1,
        &[0, 1, 0, 2, 0, 3, 0, 4],
        ColorType::Rgba,
        BitDepth::Sixteen,
        None,
        None,
        false,
    )?;
    let indexed = encode_basic_png(
        4,
        &[0b00_01_10_11],
        ColorType::Indexed,
        BitDepth::Two,
        Some(&[1, 2, 3, 10, 20, 30, 40, 50, 60, 70, 80, 90]),
        None,
        false,
    )?;
    let mut interlaced = canonical.to_vec();
    interlaced[28] = 1;
    let cases = [
        ("basic-rgba16", rgba16),
        (
            "basic-gamma",
            insert_chunk_before(canonical, *b"IDAT", *b"gAMA", &45_455_u32.to_be_bytes())?,
        ),
        (
            "basic-exif",
            insert_chunk_before(canonical, *b"IDAT", *b"eXIf", b"MM\0*")?,
        ),
        (
            "basic-animation",
            insert_chunk_before(canonical, *b"IDAT", *b"acTL", &[0; 8])?,
        ),
        (
            "basic-suggested-palette",
            insert_chunk_before(canonical, *b"IDAT", *b"PLTE", &[0, 0, 0])?,
        ),
        (
            "basic-indexed-without-palette",
            remove_chunk(&indexed, *b"PLTE")?,
        ),
        ("basic-interlace-not-yet-profiled", interlaced),
    ];
    Ok(cases
        .into_iter()
        .map(|(name, bytes)| trial(name, decode_png_basic_rgba8(&bytes).is_err()))
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

fn encode_basic_png(
    width: u32,
    pixels: &[u8],
    color: ColorType,
    depth: BitDepth,
    palette: Option<&[u8]>,
    transparency: Option<&[u8]>,
    srgb: bool,
) -> Result<Vec<u8>, String> {
    let mut bytes = Vec::new();
    {
        let mut encoder = png::Encoder::new(Cursor::new(&mut bytes), width, 1);
        encoder.set_color(color);
        encoder.set_depth(depth);
        encoder.set_filter(Filter::NoFilter);
        if let Some(palette) = palette {
            encoder.set_palette(palette);
        }
        if let Some(transparency) = transparency {
            encoder.set_trns(transparency);
        }
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

fn remove_chunk(bytes: &[u8], target: [u8; 4]) -> Result<Vec<u8>, String> {
    let offset = find_chunk(bytes, target)?;
    let length = usize::try_from(u32::from_be_bytes(
        bytes[offset..offset + 4]
            .try_into()
            .expect("four-byte PNG fixture length"),
    ))
    .map_err(|error| error.to_string())?;
    let end = offset
        .checked_add(length + 12)
        .ok_or_else(|| "PNG chunk removal overflow".to_owned())?;
    let mut output = Vec::with_capacity(bytes.len() - (end - offset));
    output.extend_from_slice(&bytes[..offset]);
    output.extend_from_slice(&bytes[end..]);
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
