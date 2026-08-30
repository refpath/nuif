---
id: nuif:rfc:0010
kind: rfc
status: proposed
---

# RFC 0010 — Portable resource and package profile

Status: proposed. This RFC does not change executable profile 0 until
`nuif:experiment:portable-package-resources`, the image profile and the font
profile satisfy their acceptance criteria.

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

A font asset records exact byte digest when available, media type, face or
collection index, names used for matching, axes, features, character coverage
and policy evidence. Text shaping continues to pin its execution inputs.

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

The exact header values become final only when two independent writers pass the
byte fixture. Until then these are proposed semantics, not stable wire bytes.

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

## Image profile proposal

The first executable image profile should be PNG-only. It must pin:

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

## Font profile proposal

The first executable font-resource profile should accept exact OpenType/WOFF2
bytes only under a declared portability policy. It must pin:

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

## Security

The package is untrusted. Before acceptance, the profile requires calibrated
limits for total archive bytes, member count, per-member bytes, total expanded
bytes, descriptor count, image/font bytes and decoder allocations. Resource
size and digest are verified before media parsing. Readers do not extract to a
filesystem.

Package resources never execute by being present. Scripts, shaders, links and
embedded metadata are inert unless a separately authorized sandboxed capability
interprets them.

## Conformance tests

`nuif:experiment:portable-package-resources` must prove:

- two independent writers produce identical bytes on the normative fixture;
- package write/read/write reaches a byte fixpoint;
- document hash is unchanged by package creation and deletable-cache changes;
- asset identity survives byte replacement while the resource/document hashes
  change as specified;
- missing, extra, duplicate, traversal, symlink, directory, encrypted,
  unsupported, mismatched-size and mismatched-digest cases fail atomically;
- no implicit external resolution occurs;
- boundary and one-over archive/resource cases pass measured limits.

The PNG and font experiments must pass before those resource kinds are claimed.

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

- Exact fixed ZIP creator/external-attribute values pending the independent
  writer fixture.
- Whether future packages add deterministic compression or rely on outer
  transport compression.
- Whether correspondence/capture/report records are canonical-adjacent members
  or separate linked artifacts.
- Media-type registration timing and final name.
- License-expression vocabulary beyond preserved evidence and user/admin policy.
