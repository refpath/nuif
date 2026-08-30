# NUIF bounded PNG decoder profiles

Status: executable experimental profile (`nuif-png-rgba8-0`). It is a narrow,
fail-closed image path, not a claim of general PNG support.

## Accepted datastream

The resource is an exact PNG datastream with:

- one `IHDR`, non-zero dimensions at most 8,192 by 8,192, and at most
  16,777,216 pixels;
- bit depth 8, colour type 6 (truecolour with alpha), standard compression and
  filtering, and no interlace;
- optionally one valid `sRGB` chunk before image data;
- one or more contiguous `IDAT` chunks followed by one empty `IEND`;
- no other chunks and no bytes after `IEND`.

Absence of `sRGB` means encoded RGBA samples are interpreted as sRGB by this
profile. It does not mean the source asserted an `sRGB` chunk. The profile
rejects palette, grayscale, RGB-only, 16-bit, CICP, ICC, gamma/chromaticity,
Exif, animation, textual and arbitrary ancillary metadata instead of applying
host-dependent precedence or conversion.

Encoded input is limited to 32 MiB and 4,096 chunks. The decoder verifies PNG
CRC and DEFLATE integrity and allocates exactly four decoded bytes per accepted
pixel. Original encoded bytes remain the content-addressed authoritative
resource; RGBA output is a deletable cache.

## Image-paint lowering

The reference scene supports `fill`, `contain` and `cover`, normalized crop,
nearest or fixed 16-bit-weight bilinear sampling, finite opacity from zero
through one, and `color_conversion = "srgb"`. Source alpha is straight. Paint
opacity multiplies alpha, and the CPU raster applies the same encoded-sRGB
integer source-over rule as profile-zero solid paint.

Only the identity image transform is executable with either current decoder profile.
Unresolved resources, dimension mismatch, unsupported decoder/profile values,
non-identity transforms and invalid crop/opacity values produce item-level
fidelity or typed errors; they never substitute a bounds rectangle or fetch a
resource implicitly.

## Profile-zero evidence

`cargo xtask gate-i-image`:

- generates all five PNG row filters plus the encoder's adaptive selection,
  with and without an explicit `sRGB` chunk;
- requires identical dimensions and RGBA bytes from `png` 0.18.1 and
  independently implemented `zune-png` 0.5.2 with unsafe paths disabled and
  CRC/Adler checks enabled;
- preserves encoded bytes through package fixpoint and an unrelated semantic
  edit;
- repeats scene lowering and CPU rasterization exactly;
- checks hostile/unsupported colour types, metadata, corruption, trailing
  bytes, and dimension, pixel, chunk and encoded-byte one-over cases.

## Basic RGBA8 profile one

Status: executable experimental profile (`nuif-png-basic-rgba8-1`). This is a
new profile, not a silent expansion of profile zero.

Profile one accepts the same bounds, compression/filter methods, contiguous
image data, optional `sRGB` declaration and encoded-sRGB interpretation as
profile zero. It additionally accepts every non-interlaced PNG colour/depth
combination that can be normalized to RGBA8 without discarding sample
precision:

- greyscale at 1, 2, 4 or 8 bits;
- indexed colour at 1, 2, 4 or 8 bits, with its required `PLTE`;
- 8-bit RGB, greyscale-alpha and RGBA;
- one valid pre-image `tRNS` colour key or indexed alpha table where the PNG
  colour type permits it.

Sub-byte greyscale samples use PNG's exact full-range expansion. Indexed
samples use their exact 8-bit palette entries. Missing alpha becomes 255 and
`tRNS` becomes an explicit 8-bit alpha channel. These are lossless
normalizations into RGBA8; original encoded bytes remain authoritative.

The profile rejects 16-bit samples rather than silently dropping precision. It
also continues to reject Adam7 interlace, CICP, ICC, gamma/chromaticity, Exif,
animation, textual data, suggested palettes on non-indexed images and arbitrary
ancillary chunks. Those features need explicit colour/orientation/animation
contracts and independent fixtures.

`cargo xtask gate-i-image` adds thirteen profile-one fixtures spanning every admitted
colour/depth combination and both colour-key and palette
transparency. `png` 0.18.1 and independently implemented `zune-png` 0.5.2 must
produce the same normalized RGBA bytes. Seven profile-one negatives cover
16-bit precision, rejected metadata, suggested palettes, a missing required
palette and the not-yet-profiled interlace path. A profile-one RGB resource
also passes renderer lowering and CPU rasterization.

## Evidence boundary

The gate does not establish GPU or hosted cross-platform image-raster
equivalence, arbitrary affine image transforms, a broad real-world corpus,
16-bit/interlaced/colour-managed PNG, or any non-PNG image format. Those require
distinct profiles and fixtures.
