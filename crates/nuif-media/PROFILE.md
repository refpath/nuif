# NUIF PNG RGBA8 profile zero

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

Only the identity image transform is executable in this first profile.
Unresolved resources, dimension mismatch, unsupported decoder/profile values,
non-identity transforms and invalid crop/opacity values produce item-level
fidelity or typed errors; they never substitute a bounds rectangle or fetch a
resource implicitly.

## Evidence boundary

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

The gate does not establish GPU or cross-platform image-raster equivalence,
arbitrary affine image transforms, a broad real-world corpus, the rejected PNG
forms above, or any non-PNG image format. Those require distinct profiles and
fixtures.
