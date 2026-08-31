---
id: nuif:rfc:0010
kind: rfc
status: proposed
---

# RFC 0010 — Portable resource and package profile

Status: proposed, with an executable experimental container subset. The
reference implementation now provides stable assets, deterministic
`nuif-package-0`, explicit verified resolution and the package segment of
`nuif:experiment:portable-package-resources`. This RFC does not add image
rendering or general packaged-font conformance to profile 0. The orthogonal
`nuif-png-rgba8-0` experiment now implements a narrow cross-decoder and CPU
image path. The `nuif-opentype-static-single-0` experiment likewise implements
one narrow package/parser/policy baseline; broader image and font profiles still
have to satisfy their acceptance criteria.

## Motivation

The current implementation serializes a semantic document as canonical text or
CBOR. That is sufficient for profile-0 fixtures because images are unsupported
and the one text font is an external pinned conformance input. It is not a
portable authoring package: images, fonts, source correspondence and other
resources have no common descriptor or delivery contract.

The draft serialization module previously called `.nuif` a package while
leaving its archive technology experimental. The roadmap then described
“package/assets” as complete for profile 0. Those statements conflict. This RFC
defines a candidate profile and restores the implementation gate.

The design must distinguish:

- stable semantic asset identity;
- identity of immutable resource bytes;
- a locator inside or outside a package;
- source and derivation provenance;
- semantic document hash;
- exact delivered-package hash.

Using one path, URL or hash for all six roles would break editing, offline
portability or reproducibility.

## Prior art and evidence

- `nuif:research:epub-ocf-package-container`: OCF separates an abstract
  container from a constrained ZIP form, requires a manifest and identifies the
  media type in a fixed first uncompressed member.
- `nuif:research:oci-resource-descriptors`: OCI binds media type, size and
  digest, verifying cheap limits and bytes before content interpretation.
- `nuif:research:content-addressed-versioning`: immutable byte identity is not
  stable editable entity identity.
- `nuif:research:png-image-preservation-and-decoding`: original encoded image
  bytes and declared decode parameters are distinct from decoded caches.
- `nuif:research:opentype-font-embedding-and-portability`: reproducible text
  needs exact bytes, but redistribution policy must be explicit.
- `nuif:research:penpot`: the existing bounded adapter proves defensive ZIP
  handling but not this package layout or cross-writer determinism.

## Proposed semantics

### 1. Encoding and extension names

- `.nuif` identifies `nuif-package-0`, the portable package profile.
- `.nuif.cbor` identifies bare canonical `nuif-cbor-0` document bytes.
- `.nuif.json` identifies bare canonical `nuif-text-0` document bytes.

During the alpha migration, readers MAY recognize historical bare documents
whose name ends in `.nuif`. Writers MUST emit the package for `.nuif` and MUST
use an explicit bare extension for new bare documents. Legacy recognition is
read-only compatibility and MUST NOT weaken canonical codec validation.

### 2. Resource identities

`AssetId` is the stable semantic identity referenced by document entities.
Replacing an asset's bytes does not change `AssetId`; it changes the asset's
bound `ResourceDigest` through a semantic operation.

`ResourceDigest` in package profile 0 is:

```text
sha256:<64 lowercase hexadecimal digits>
```

It identifies exact encoded bytes. Implementations MUST verify declared size
and digest before decoding the resource.

`ResourceDescriptor` contains:

```text
digest       ResourceDigest
size         unsigned byte length
media_type   normalized ASCII media type without a retrieval-dependent value
role         source | authoring | derived | cache
locator      embedded path, or explicit linked locator plus expected digest
derivation   required for role=derived; absent for source/authoring/cache
```

The descriptor is immutable by digest. Metadata that changes interpretation of
the bytes belongs in the semantic asset or a versioned decoder profile, not in
an untracked package field.

### 3. Asset semantics

An `Asset` contains stable identity, kind, current resource digest, portability
policy and kind-specific semantic metadata. Initial kinds are `image` and
`font`; unknown future kinds follow extension-preservation rules.

An image asset records intrinsic dimensions and the decoder profile. An
`ImagePaint` refers to `AssetId` and records fit, crop, transform, sampling,
opacity and declared color conversion. Decoded RGBA and GPU textures are
deletable caches, never source resources.

The reference scene interns decoded RGBA by resource digest plus decoder
profile and gives commands deterministic numeric surface handles. Its 64 MiB
unique decoded-surface total is preflighted from bounded image metadata before
inflation. Repeated asset instances therefore do not duplicate decoded pixels
or descriptor strings.

A font asset records exact byte digest when available, media type, face or
collection index, names used for matching, axes, features, character coverage
and policy evidence. Text shaping continues to pin its execution inputs. An
optional stable text-to-font `AssetId` keeps the requested text hash distinct
from the effective resource hash: exact bindings require equality, substituted
bindings retain the request and select the asset resource, and unavailable
bindings select an asset with no resource.

Font portability policy is one of:

- `portable`: exact bytes can be embedded for the declared package use;
- `private_authoring`: bytes may exist in a private workspace package but not a
  distributable portable package;
- `linked`: bytes are absent; locator and expected digest are explicit;
- `substituted`: another exact resource is used and fidelity identifies the
  substitution;
- `unavailable`: no usable bytes; fidelity identifies the consequence.

OpenType `fsType` is recorded policy evidence, not a complete legal conclusion.

### 4. Package members

`nuif-package-0` is a ZIP archive with these members:

```text
mimetype
manifest.cbor
document.cbor
blobs/sha256/<64 lowercase hexadecimal digits>
```

Optional correspondence, capture, report and cache records MAY be added only at
paths registered by a later profile. Profile 0 package readers reject
unregistered member paths instead of guessing their meaning.

`mimetype` is the first local-file member, stored without compression,
encryption or extra fields. Its exact ASCII bytes are:

```text
application/nuif+zip
```

This media type is provisional until registration and MUST NOT be represented
as IANA-registered.

`manifest.cbor` and `document.cbor` are canonical `nuif-cbor-0`. The manifest
declares profile/version, the semantic document descriptor, required
capabilities, assets, all resource descriptors and their roles. It does not
contain its own digest.

The required-capability set contains at most 256 identifiers of at most 128
ASCII bytes each. Structural readers validate and preserve those requirements.
Hosts use `capability_report` or `require_capabilities` with an explicitly
declared supported set before claiming full package support; missing
requirements are reported exactly. Structural decode alone is intentionally
available to inert inspection, preservation and extraction tools and is not a
semantic-support claim.

A structural SDK, CLI or editor that does not support every required capability
MUST keep the package read-only. It MAY validate, hash, extract the bare
document and copy the unchanged same-mode package, but it MUST NOT evaluate,
change `document.cbor` or change package mode while carrying capability
resources forward unless complete-set negotiation succeeds or a
capability-specific authoring profile explicitly detaches those resources. A
failed partial negotiation grants no authority. This prevents a
content-addressed sidecar from being silently bound to a document revision it
never validated.

Every embedded resource is stored at the path derived from its SHA-256 digest.
The path is a locator. The manifest digest remains the identity and MUST match
the bytes even if a future profile permits another physical layout.

### 5. Deterministic ZIP profile

The first profile uses stored members only. This deliberately trades archive
size for cross-writer byte determinism and simple expansion limits. Images and
fonts are usually already compressed; a later measured profile may add a fixed
compression method without changing semantic hashes.

Writers MUST:

- emit `mimetype` first, then every other member in bytewise ASCII path order;
- use only the exact registered ASCII paths above;
- use ZIP method 0 (stored), no encryption, no data descriptors, no ZIP64 unless
  a later profile permits it, no archive/member comments and no extra fields;
- set the DOS timestamp to 1980-01-01 00:00:00;
- set a fixed creator/version and regular-file external attributes defined by
  the conformance fixture;
- precompute CRC-32 and sizes so local and central headers agree;
- emit no directory entries;
- emit one central-directory entry for each local member in the same order.

The manual writer and the independently implemented `zip` 8.6.0 writer now
produce identical fixture bytes with creator/version `0x030a`, version-needed
`10`, regular-file attributes `0x81a40000` and the rules above. These values are
fixed for the experimental `nuif-package-0` implementation. Standards-track
stability still requires cross-platform and externally maintained reproduction.

Readers MUST reject duplicate decoded names, backslashes, absolute paths,
dot-segments, empty segments, non-ASCII paths, symlinks, directories, split or
spanned archives, encryption, unsupported compression, inconsistent headers,
undeclared blobs and declared embedded blobs that are absent.

### 6. External resolution

A portable package embeds every resource required to evaluate its declared
profile. A linked/private authoring package may contain a linked locator, but:

- loading a document MUST NOT cause implicit network access;
- a resolver is an explicit caller-supplied capability;
- resolved bytes MUST match declared size and digest;
- redirects and authentication are resolver policy, never package authority;
- credentials, cookies and bearer tokens MUST NOT be stored in locators or
  provenance records;
- failure to resolve produces a typed fidelity/availability result.

Original URLs are provenance or retrieval hints, not resource identity.

### 7. Hashes

The semantic document hash remains SHA-256 of canonical `document.cbor` bytes.
It changes only when semantic document content changes.

The package hash is SHA-256 of the complete deterministic ZIP bytes. It proves
the exact delivered artifact and changes when package-only records or caches
change.

A resource digest is SHA-256 of exact resource bytes. An asset binding is
semantic and therefore participates in the semantic document hash. Package
locations and deletable caches do not.

## Executable narrow image segment and broader proposal

`nuif-png-rgba8-0` now executes the smallest unambiguous subset: bounded,
non-interlaced RGBA8; no ancillary metadata or one valid pre-image `sRGB`
intent; encoded samples interpreted as sRGB; straight decoded alpha; identity
decoder orientation; declared fit/crop, bounded forward affine transform,
nearest or fixed-bilinear sampling, opacity
and encoded-sRGB integer source-over. It rejects every other colour type,
bit-depth, colour signal, Exif/animation chunk and arbitrary metadata. Two
independent decoder libraries must agree on exact RGBA bytes. Exact rules and
non-claims are versioned in `crates/nuif-media/PROFILE.md`, and
`cargo xtask gate-i-image` emits its machine evidence.

The separately named `nuif-png-basic-rgba8-1` decoder profile adds every
non-interlaced PNG colour/depth combination that can normalize to RGBA8 without
sample-precision loss. Image-paint transforms use
`[a c tx; b d ty; 0 0 1]` from crop-local source coordinates into the fitted
paint rectangle; the CPU reference inverse-samples pixel centers and rejects
singular or numerically unbounded matrices. Decoder and paint semantics remain
separate contracts even though Gate I exercises them together.

The broader PNG experiment remains separate. It must pin:

- accepted PNG conformance and ancillary chunks;
- decoder library/version or independent semantic rules;
- CICP/ICC/sRGB/gamma/chromaticity precedence and conflict policy;
- Exif orientation behavior;
- straight-alpha decoded representation and explicit premultiplication point;
- output color space, sampling and compositing;
- encoded bytes, dimensions, pixel count, decoded bytes, chunks and metadata
  limits;
- malformed-image and independent-decoder fixtures.

Animated PNG, JPEG, WebP, AVIF, video and SVG do not enter this profile by
fallback. An adapter may freeze a selected animation/video frame as a derived
resource and must report the transformation. SVG is imported through a safe
declared adapter subset or retained as inert source; scripts and external
resources never execute when a package opens.

## Executable narrow font segment and broader proposal

`nuif-opentype-static-single-0` accepts exact static TrueType-outline sfnt bytes
only under a declared portability policy. It requires face index zero,
canonically packed and checksummed tables, matching family names and exact
Unicode coverage, no variation axes, a matching `fsType` value, a non-empty
license expression and explicit embedding review. Package encode/decode and
resolved linked bytes run the same validation. The exact rules and non-claims
are in `crates/nuif-font/PROFILE.md`; `cargo xtask gate-i-font` compares Skrifa
0.46.2 behind NUIF-owned sfnt validation with a committed HarfBuzz 14.4.0
metadata capture for the pinned Ahem fixture.

This executable slice deliberately rejects TTC, CFF/CFF2, variable, color,
bitmap, SVG and WOFF/WOFF2 fonts. The broader font-resource profile must pin:

- parser and table/resource budgets;
- face/collection selection;
- axes, named instances and feature selection;
- coverage and shaping inputs;
- malformed-table fixtures;
- handling of `fsType`, no-subsetting and bitmap-only flags;
- policy outcomes for portable/private/linked/substituted/unavailable resources.

No profile may infer exact font identity from a family name or screenshot.

## Compatibility and migration

Existing canonical document bytes and hashes remain valid. Packaging them does
not alter `document.cbor` or its semantic hash. Current `.nuif` raw fixtures are
readable during alpha through content detection; tools should rewrite them only
on explicit save/export and should report the transition.

The package layer belongs above `nuif-codec`: codecs own canonical bare
encodings, while the package implementation owns manifest/resource/ZIP rules.
The core owns asset/resource semantics. CLI, editor, WASM, FFI and process
adapters call the same package API and MUST NOT carry independent ZIP policy.

The reference `nuif-wasm-api-0` binding exposes structural package load,
explicit manifest-capability negotiation and deterministic package export over
byte arrays. Its cross-surface gate requires both no-op and edited package bytes
to match the native SDK exactly and preserves an embedded capability resource
without interpreting it. The same gate rejects a semantic edit through a
structurally loaded requirement-bearing package before complete-set
authorization. This is the browser/plugin package transport; host object access
and capability execution remain separate adapters.

## Security

The package is untrusted. Before acceptance, the profile requires calibrated
limits for total archive bytes, member count, per-member bytes, total expanded
bytes, descriptor count, image/font bytes and decoder allocations. Resource
size and digest are verified before media parsing. Readers do not extract to a
filesystem.

The executable allocation gate additionally requires package-to-session
handoff to share immutable resource buffers. Its 8 MiB trial preserves the
allocation pointer and permits at most 1 MiB of handoff allocator traffic and
retained bookkeeping. A 1,024-instance image trial permits at most 8 MiB of
scene-build allocator traffic and 4 MiB retained for one 1 MiB decoded surface.
These are reference-CI regression ceilings rather than wire-format limits.

Package resources never execute by being present. Scripts, shaders, links and
embedded metadata are inert unless a separately authorized sandboxed capability
interprets them.

## Conformance tests

`nuif:experiment:portable-package-resources` must prove:

- two independent writers produce identical bytes on the normative fixture;
- package write/read/write reaches a byte fixpoint;
- required capabilities are bounded identifiers and missing host support is
  returned as the exact deterministic requirement set;
- document hash is unchanged by package creation and deletable-cache changes;
- asset identity survives byte replacement while the resource/document hashes
  change as specified;
- missing, extra, duplicate, traversal, symlink, directory, encrypted,
  unsupported, mismatched-size and mismatched-digest cases fail atomically;
- no implicit external resolution occurs;
- boundary and one-over archive/resource cases pass measured limits.

The narrow PNG experiment and package experiment must pass before claiming
`nuif-png-rgba8-0`. The broader PNG and font experiments must pass before those
resource classes are claimed generally.

## Rejected alternatives

- Keep `.nuif` as ambiguous raw JSON/CBOR indefinitely: prevents reliable media
  identification and resource delivery.
- Use paths or URLs as identity: moving a package or changing a CDN URL would
  change identity without changing bytes.
- Use content hashes as editable asset IDs: every image replacement would break
  semantic references and operation history.
- Store decoded RGBA instead of original images: loses source encoding, color
  metadata and efficient distribution.
- Embed every font found by a browser: local bytes may be inaccessible and
  redistribution may not be permitted.
- Permit ordinary ZIP/ZIP64/compression options: expands attack surface and
  prevents a simple first cross-writer byte profile.
- Use TAR: weak random access and no widely deployed first-member media-type
  convention for this use.
- Use OCI artifacts directly: descriptor ideas are valuable, but registry/image
  layering semantics are unnecessary for a single design package.

## Unresolved questions

- Whether future packages add deterministic compression or rely on outer
  transport compression.
- Whether correspondence/capture/report records are canonical-adjacent members
  or separate linked artifacts.
- Media-type registration timing and final name.
- License-expression vocabulary beyond preserved evidence and user/admin policy.
