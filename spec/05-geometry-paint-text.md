---
id: nuif:spec:geometry-paint-text
kind: specification
status: draft
---

# 05 — Geometry, paint and text

Status: draft.

Geometry follows established 2D vector mathematics: affine transforms, rectangles/rounded rectangles, ellipses, lines and Bézier paths. Path semantics SHOULD align with SVG where possible.

Paint supports solid colors, gradients, images, strokes, opacity, clipping/masks and compositing/blend modes. Color values MUST declare a color space; conversions are evaluator responsibilities.

## Asset and resource boundary

An asset is a semantic entity with stable `AssetId`. A resource is an immutable
byte sequence identified by `ResourceDigest`. Package paths and external
locators are resolution hints and MUST NOT be used as either semantic or byte
identity.

Replacing the bytes bound to an asset MUST be expressed as a semantic operation
that preserves `AssetId` and changes its `ResourceDigest`. Every resource
descriptor declares media type and byte length. An implementation MUST verify
the declared size and digest before media-specific decoding.

Resource roles are:

- `source` — exact bytes received from an origin;
- `authoring` — bytes required to evaluate or edit the semantic document;
- `derived` — bytes produced from named inputs by a declared transformation;
- `cache` — deletable acceleration data that cannot affect semantic hashes.

Derived resources MUST identify all source digests and the transformation
artifact/profile. A crop or trace created from a screenshot is `derived`; it
MUST NOT be presented as the original source asset.

## Image assets

`ImageAsset` records `AssetId`, current encoded-resource digest, intrinsic pixel
dimensions and decoder profile. `ImagePaint` refers to the asset and records
fit, crop, affine transform, sampling, opacity and color-conversion policy.
Decoded pixels and GPU textures are caches keyed by encoded digest plus decoder
profile.

Renderer-independent scenes MUST store a unique decoded surface once per
resource-digest/decoder-profile pair and reference it from image commands.
The reference scene budget is 64 MiB of decoded RGBA surfaces and MUST be
checked from bounded metadata before allocating the next decode.

The affine fields use `[a c tx; b d ty; 0 0 1]` and map the selected crop's
normalized source coordinates forward into normalized coordinates of the
fitted paint rectangle. Fit is calculated first. Rasterizers inverse-map
destination pixel centers, apply crop selection after that inverse, and clip to
the entity rectangle. The origin is the fitted rectangle's top-left; callers
encode any center-origin adjustment in the translation. Executable matrices
MUST be finite and invertible. The reference bound rejects components or
inverse components above 1,000,000 in magnitude and determinant magnitudes
below `1e-12` as unsupported fidelity.

The first executable image profile, `nuif-png-rgba8-0`, is PNG-only and
deliberately narrow. It accepts non-interlaced RGBA8 with no ancillary chunk or
one valid pre-image `sRGB` chunk, interprets the encoded samples as sRGB and
keeps alpha straight through decoding. It rejects palette, grayscale, RGB-only,
16-bit, CICP, ICC, gamma/chromaticity, Exif, animation, arbitrary ancillary
chunks and trailing bytes. Its exact chunk sequence, dimensions, pixel/byte
budgets, fit/crop, bounded affine transform, nearest/fixed-bilinear sampling, opacity
and encoded-sRGB integer composition contract is in
`crates/nuif-media/PROFILE.md`. JPEG, WebP, AVIF, animation, video and SVG are
unsupported by that profile rather than silently decoded through host defaults.

The separately named `nuif-png-basic-rgba8-1` profile accepts the
non-interlaced PNG colour/depth combinations that normalize to RGBA8 without
sample-precision loss, including required palettes and valid `tRNS`
transparency. It does not change profile zero and still rejects 16-bit,
interlaced and colour-managed inputs. Its exact matrix is also in
`crates/nuif-media/PROFILE.md`.

An animation or video adapter MAY create a derived still resource when it
records source digest, selected frame/time, decoder profile and item-level loss.
SVG is evaluated only through a declared safe adapter profile; otherwise its
bytes remain inert and preserved.

## Font assets and portability

Text stores Unicode scalar content, style runs, paragraph attributes,
direction/language hints and a requested content-addressed font identity. A
font SHA-256 reference MUST contain 64 lowercase hexadecimal digits. Font size
and line height MUST be finite and positive. An optional `font_asset` binds the
text item to a stable font asset; a family or PostScript name is never a
substitute for that identity.

A font asset additionally records media type, face or collection index, names
used for matching, variation axes, feature selections, coverage and portability
policy. The policy is `portable`, `private_authoring`, `linked`, `substituted`
or `unavailable`. OpenType embedding flags and explicit license metadata are
policy evidence; the format does not claim to make a complete legal decision.

A portable package MUST NOT embed a font whose effective export policy forbids
that embedding. Linked fonts retain expected digest and explicit resolver hint;
resolution is opt-in and digest-checked. Substitution and unavailability MUST
produce item-level fidelity. Family/PostScript names alone MUST NOT satisfy an
exact-font profile.

For an exact binding, the asset resource SHA-256 MUST equal the requested text
hash. For a substituted binding, the text retains the requested hash and the
asset resource identifies the exact replacement bytes. For an unavailable
binding, the asset MUST carry no resource. Layout and rendering MUST use an
available declared replacement with `approximated` fidelity; if replacement
bytes are absent, or the asset is unavailable, rendering MUST emit no text
command and MUST report item-level `unsupported` fidelity. Resolution MUST NOT
query a platform font database or perform I/O.

The first executable resource subset, `nuif-opentype-static-single-0`, accepts
only one canonically packed, checksummed TrueType-outline sfnt face at index
zero. It requires exact `font/ttf` bytes, matching family names and Unicode
coverage, no variation axes, matching `fsType` evidence, a non-empty license
expression and an explicit embedding review. It rejects TTC, CFF/CFF2,
variable, color, bitmap, SVG and WOFF/WOFF2 sources. Exact limits and non-claims
are versioned in `crates/nuif-font/PROFILE.md`. This narrow package resource
profile does not establish shaping or raster equivalence.

A conformance profile that compares resolved text MUST declare the exact font bytes and hash, shaper and Unicode-data versions, direction, language, script-selection rule, feature set, cluster level, cluster coordinate unit, positioning unit and resource limits. Resolved runs contain source text plus ordered glyph identifiers, clusters, advances and offsets; they MUST NOT depend on system font discovery. Profile 0 uses Unicode-scalar indices for cluster coordinates and unscaled font units for advances and offsets.

Shaping and rasterization are distinct conformance stages. A shaping pass does not imply raster conformance. A raster profile MUST additionally declare outline extraction, hinting, stem darkening, anti-aliasing, subpixel quantization, color/blend space and compositing rules. Until those parameters and their foreign/cross-platform trials exist, an implementation MUST classify a glyph-ID bitmap proxy as `approximated` rather than exact text rendering.

Implementations MUST preserve source text even when resolved glyph information is present.

## CPU render profile 0

Profile 0 is deliberately narrower than the complete model. Its supported visual operations are encoded-sRGB solid fills of rectangles and ellipses, plus the text subset below. Color channels are finite numbers in the inclusive range 0 through 1. The reference raster starts as opaque white RGBA8.

Logical geometry is multiplied by the target scale factor before rasterization. A rectangle covers every pixel in `floor(x)..ceil(x + width)` and `floor(y)..ceil(y + height)`, clipped to the target; it does not compute fractional edge coverage. Float color channels become bytes with `round(clamp(channel, 0, 1) × 255)`. For mask coverage `coverage`, effective source alpha is `(source_alpha × coverage + 127) / 255` using integer division. Each encoded-sRGB destination color channel becomes `(source_channel × alpha + destination_channel × (255 - alpha) + 127) / 255`; output alpha remains 255.

An ellipse is the closed four-cubic path inscribed in its bounds using control coefficient `0.551915024494`. It is rasterized with nonzero fill into an 8-bit grayscale mask by Zeno 0.3.3, crates.io checksum `6df3dc4292935e51816d896edcd52aa30bc297907c26167fec31e2b0c6a32524`, then composited by the integer rule above. `conformance/render/profile-zero-v1.json` fixes rectangle and ellipse scene/PNG hashes on the recorded platform matrix.

Profile-0 text uses Ahem 1.50, HarfRust 0.13.3 with Unicode 17.0.0, unhinted Skrifa 0.46.2 outlines in signed 26.6 font units, and Zeno 0.3.3 grayscale masks. CRLF is one hard break; CR, LF, NEL, LINE SEPARATOR and PARAGRAPH SEPARATOR are individual hard breaks. Each hard line is shaped independently. Intrinsic width is the greatest shaped line advance; intrinsic height is the number of hard lines times `line_height`. The first baseline is 800 Ahem font units below the line top, subsequent baselines differ by `line_height`, LTR starts at the left edge, RTL starts at the right edge, and output is clipped to the text box. Profile 0 performs no automatic soft wrapping. Because wrapping is not an authored property in this profile, absence of soft wrapping is exact profile behavior rather than an approximation.

Path geometry, image assets, component-instance materialization and extension-defined paint/effects are not supported by CPU render profile 0. Lowering MUST emit `unsupported` or `preserved_unrenderable` fidelity with the originating entity and property pointer; it MUST NOT substitute bounds rectangles or silently omit the data. `nuif-png-rgba8-0` is an orthogonal experimental image segment and does not change profile-0 results. Future profiles may compose accepted segments explicitly.

The asset and broad-font requirements above remain draft inputs for broader
profiles. The narrow executable image and static-font resource segments do not
implicitly enter CPU render profile 0.
