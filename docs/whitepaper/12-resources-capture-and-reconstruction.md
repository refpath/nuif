---
id: nuif:whitepaper:resources-capture-reconstruction
kind: whitepaper
status: draft
version: 0.0.1
updated: 2026-08-30
---

# Resources, capture and model-neutral reconstruction

NUIF's next research front is not “add an AI converter.” It is a coordinated
resource, capture and reconstruction architecture with explicit truth
boundaries. Images and fonts must survive as verified resources. Source-backed
browser imports must retain authored/resolved evidence. Screenshot-only imports
must remain honest probabilistic hypotheses. Every path converges through one
core operation, validation, rendering and fidelity contract.

## Decision summary

The recommended direction is:

1. define stable assets separately from immutable byte resources;
2. make `.nuif` a deterministic portable package after cross-writer proof;
3. grow the executable narrow PNG and static TrueType resource baselines only
   through named profiles and measured hostile-input budgets;
4. add a pinned browser-capture adapter separate from static source sync;
5. build screenshot reconstruction as a replaceable observation/proposal loop;
6. freeze a structural and visual evaluation suite before training;
7. consider adaptation or distillation only after the untuned loop exposes a
   repeatable learnable error distribution.

RFC 0010 and RFC 0011 remain proposed contracts. Their bounded package, narrow
PNG/static-font and capture/reconstruction experiments are implementation
evidence only for the named subsets; they are not published conformance or
standards claims.

## One core, two import lanes

```text
Source-backed lane
HTML/CSS + browser execution + resource responses
      -> retained source + resolved observations
      -> deterministic adapter/lowering + explicit inference where needed
                              |
                              v
                    typed NUIF operations
                              |
                              v
                     core validation/apply
                              |
                              v
                      resource-aware NUIF
                              ^
                              |
Screenshot-only lane          |
pixels + context              |
      -> OCR/CV/grounding observations
      -> hierarchy/layout/resource hypotheses
      -> typed operations -> render/diff/correct
```

The lanes differ in evidence, not in their mutation authority. Both use the same
typed operations. Neither provider can write core structs directly. A browser
observation may support an exact resolved value under one pinned context, but it
does not automatically reveal authored intent. A screenshot cannot establish
source equivalence regardless of visual score.

## Portable resource model

Four identities must remain distinct:

| Concern | Identity | Change behavior |
|---|---|---|
| editable semantic asset | `AssetId` | stable when its content is replaced |
| immutable encoded bytes | `ResourceDigest` | changes for any byte change |
| package/resolver location | locator | may change without changing bytes |
| source/derivation history | provenance record | may grow without renaming asset/bytes |

An asset points to a content descriptor containing media type, SHA-256 and byte
length. A package path or external URL only locates candidate bytes; size and
digest are checked before decoding. External resolution is opt-in. Opening a
document never triggers network access.

Resource roles clarify hash and retention behavior:

- `source`: original encoded bytes;
- `authoring`: exact bytes needed to evaluate/edit semantics;
- `derived`: crop, trace, selected frame, conversion or generated result with
  input digests and transformation identity;
- `cache`: decoded pixels, GPU textures or acceleration state that can be
  deleted without changing the semantic document.

This avoids a common failure: storing only a decoded bitmap and calling the
source image preserved, or hashing an editable asset by its current bytes and
therefore breaking every reference after replacement.

## Candidate `.nuif` package

The proposed first package is a deliberately small deterministic ZIP profile:

```text
mimetype
manifest.cbor
document.cbor
blobs/sha256/<digest>
```

`mimetype` is first and stored. Manifest and document are deterministic CBOR.
Every embedded blob is addressed by exact SHA-256 bytes. The first profile uses
stored members only, fixed metadata and sorted ASCII paths to make independent
writer byte equality attainable and to avoid compression-version variability.

There are three hashes:

- semantic document hash over canonical `document.cbor`;
- resource digest over each exact blob;
- package hash over the exact ZIP artifact.

Cache or report changes may change the package hash while leaving the semantic
document hash untouched. Bare canonical forms remain `.nuif.json` and
`.nuif.cbor`. Historical alpha `.nuif` raw files need read-only detection during
migration; new `.nuif` output becomes the package only after RFC acceptance.

The package reader rejects duplicate or unsafe paths, symlinks, directories,
encryption, split archives, unsupported compression, inconsistent headers,
undeclared/missing blobs and digest mismatch. It does not extract to a
filesystem. Exact byte fixtures, member/resource limits and two independent
writers are acceptance gates.

Package-to-session handoff uses shared immutable buffers. The release gate
passes an 8 MiB resource through package, handle map and session with the same
allocation pointer while keeping handoff allocator traffic and retained
bookkeeping below 1 MiB. This prevents a host from paying one full resource
copy merely to enter the core.

## Images

The original encoded image is authoritative. A semantic image asset records its
resource digest and intrinsic interpretation; each image paint records fit,
crop, transform, sampling, opacity and color conversion. Derived decoded pixels
and GPU textures are caches.

PNG is the correct first format because its current W3C specification covers
lossless encoded pixels, alpha and explicit colour metadata. The executable
`nuif-png-rgba8-0` baseline chooses an intentionally smaller contract:
non-interlaced RGBA8, no ancillary metadata or one valid `sRGB` chunk, encoded
samples interpreted as sRGB, straight decoded alpha, identity encoded orientation,
declared fit/crop/sampling/opacity and bounded integer CPU composition. `png`
0.18.1 and `zune-png` 0.5.2 must emit identical RGBA bytes for the accepted
fixtures; encoded resources remain digest-identical through package edits.

This avoids pretending that decoder agreement on simple images settles PNG
Third Edition. The compatible `nuif-png-basic-rgba8-1` profile now admits the
lossless-to-RGBA8 subset: 1/2/4/8-bit greyscale and indexed images, RGB8,
greyscale-alpha8, RGBA8 and valid palette/colour-key transparency. It preserves
encoded bytes and requires exact normalized RGBA agreement between both
decoders. It is separately named so profile zero never changes meaning.

Image-paint affine semantics are orthogonal to decoder choice. The executable
matrix `[a c tx; b d ty; 0 0 1]` maps crop-local source coordinates forward
into the fitted rectangle. The CPU reference inverse-maps destination pixel
centers, clips to the entity, and rejects singular or numerically unbounded
matrices. Flip, rotation and translation fixtures make composition order
observable; live host trials are still required for vendor interoperability.

Decoded pixels are interned once per digest/profile in the renderer-independent
scene and commands carry compact deterministic handles. A 64 MiB total is
preflighted before each new inflation. The release gate retains one 1 MiB
surface for 1,024 image instances under 8 MiB allocated and 4 MiB retained,
and rejects a 64 MiB plus 16 byte declared total before the second decode.

Any broader profile still has to pin:

- accepted chunks and metadata conflicts;
- color-space precedence and output space;
- Exif orientation;
- conversion and premultiplication points for every accepted colour signal;
- sampling and compositing;
- encoded, pixel, decoded-byte, chunk and metadata limits;
- independent decoder and malformed-input fixtures.

16-bit/interlaced PNG, CICP/ICC/gamma/chromaticity, Exif, animation,
perspective/tiling and host-specific affine equivalence are not claimed. A
Linux/Windows/macOS CI matrix runs the profile, but the cross-platform claim
remains withheld until its hosted artifacts pass. JPEG, WebP, AVIF, video and
SVG follow as separate profiles.
Freezing a frame or tracing a screenshot crop is a derived approximation, not
recovery of the original asset. Generative upscaling/inpainting requires an
explicit user policy and cannot silently become canonical source evidence.

## Fonts

Exact typography depends on exact font bytes, face/collection index, variation
axes, features, coverage, shaping inputs and renderer parameters. Packaging also
depends on redistribution policy.

The proposed policy states are `portable`, `private_authoring`, `linked`,
`substituted` and `unavailable`. OpenType `fsType` is preserved as machine-
readable evidence, including restricted/preview/editable/no-subsetting/bitmap
flags, but is not treated as a complete legal license decision.

The executable `nuif-opentype-static-single-0` baseline accepts only one
canonically packed, checksummed TrueType-outline sfnt face at index zero.
Skrifa 0.46.2 supplies package-facing metadata after NUIF validates the
directory, ranges, packing and checksums and directly checks required sfnt and
`OS/2` fields. A committed `hb-info` 14.4.0 capture independently checks Ahem
metrics, family, tables and Unicode coverage. Exact bytes, family names,
coverage, `fsType`, license expression and explicit embedding review must
agree. Package encode/decode and caller-resolved linked bytes run the same
validation. Four static TrueType fixtures are accepted, while six package
trials distinguish portable, private-authoring, linked, substituted and
unavailable outcomes. Each accepted inspection and packaged-font validation is
also measured after warmup against a 4 MiB allocator-traffic and 2 MiB retained
reference ceiling; these are implementation regressions, not format semantics.
Six additional trials retain requested identity separately from a stable font
asset, render with an available declared replacement as `approximated`, and
emit no text command with item-level `unsupported` fidelity when replacement
bytes or the font are unavailable.

This is intentionally not general OpenType support. TTC, CFF/CFF2, variable,
color, bitmap, SVG and WOFF/WOFF2 sources, historic ambiguous permission
combinations, subsetting, cluster-level fallback, arbitrary packaged-font
shaping and cross-platform raster behavior remain separate fixtures and
profiles. The configured three-OS parser/package matrix does not establish
cross-platform raster behavior. Parser acceptance and `fsType` do not grant
redistribution rights.

Browser capture can identify platform fonts used for a node and capture
downloaded web-font response bodies. It generally cannot retrieve arbitrary
local font bytes. A family/PostScript name is therefore never exact resource
identity. Missing bytes produce a link, substitution or unavailable fidelity
record instead of a false portable-font claim.

## Source-backed browser capture

Static Tree-sitter source synchronization and live browser capture solve
different problems. The former preserves source spans for a bounded authored
subset. The latter runs a pinned browser to observe actual cascade, layout,
fonts, resource responses, accessibility and pixels. They should correlate
through provenance, not become one oversized adapter.

The first browser-capture profile records browser/protocol build, OS, viewport,
DPR, page scale, locale, timezone, color/reduced-motion preferences, font
environment, scroll/pseudo state, navigation identity, readiness/network policy
and animation freeze. It collects:

- original HTML/CSS and stylesheet text where accessible;
- DOM snapshots including available frame/template/shadow content;
- boxes, inline text boxes, paint order and a bounded style set;
- downloaded image/font/style response bodies and hashes;
- platform-font usage and font readiness;
- accessibility tree;
- reference screenshots with exact parameters.

Canvas, WebGL, video and worklets are bounded observation surfaces; a frame can
be preserved without pretending its generating program was reconstructed.
Cookies, authorization headers, credentials, storage and secret form values are
not exported. Scripts remain inert.

Multiple viewports and states are more valuable than one oversized capture:
they constrain layout hypotheses and permit held-out responsive evaluation.

## Screenshot reconstruction

A screenshot supplies visible samples but not the unique scene graph or layout
program. The recommended pipeline is:

```text
screenshots + contexts
  -> OCR and baselines
  -> deterministic regions, colors, edges, repetitions and asset candidates
  -> optional replaceable UI grounding
  -> typed observation graph with confidence and evidence regions
  -> replaceable reasoner proposes hierarchy/layout and NUIF operations
  -> core validates and applies atomically
  -> deterministic layout/render
  -> text/structure/geometry/resource/visual differences
  -> bounded corrective operations
```

The model emits typed operations, not an unconstrained full document or code to
execute. Invalid and stale transactions fail without partial state. High-
resolution full views, overlapping tiles and semantic crops share explicit
coordinate transforms; duplicate or conflicting observations remain visible.

The result contains a valid document or no-result, accepted operation log,
observations, derived resources, item fidelity, alternatives/abstentions,
evaluation report and exact pipeline artifact identities.

## Evaluation before training

The benchmark has separate synthetic-exact, licensed real screenshot and
source-backed suites. Synthetic NUIF rendering provides exact entities,
properties, operations and resources. Real images need human-reviewed visible
targets and must preserve ambiguity. Source-backed cases evaluate retained
bytes and observations unavailable to the screenshot-only route.

Required metrics include:

- valid operation/document rate;
- OCR region recall, character/word error and baselines;
- element precision/recall and hierarchy error;
- property/geometry accuracy;
- held-out viewport behavior;
- exact resource digest only when bytes exist;
- provenance/fidelity honesty;
- accessibility evidence where justified;
- raw pixels, FLIP, SSIM and pinned LPIPS diagnostics;
- calibrated confidence, abstention and risk/coverage;
- latency, peak RAM/VRAM, iterations and cost.

No pixel score is sufficient. A page-sized screenshot can be visually perfect
and semantically useless. Structural/text/resource/edit-task metrics prevent
that reward shortcut. Dataset splits group by origin, template, component,
font, resource and generator; near duplicates cannot cross splits.

The ablation ladder is deterministic OCR/CV, one-shot reasoner, observation-
assisted reasoner, hierarchical crops, multi-viewport ranking, correction loop,
then any tuned or distilled student. Every addition uses the same frozen
holdout and budget.

## Adaptation and distillation

Training is justified only after the untuned loop and error taxonomy are
reproducible, rights-cleared traces exist, and the remaining errors appear
learnable. Training examples contain input hashes, observation versions,
proposals, diagnostics, accepted operations, intermediate renders/differences
and final package/fidelity reports. Positive sequence targets are validated
accepted transitions, not raw model transcripts.

Compare prompt/schema/tool improvements and retrieval before fine-tuning. If
adaptation remains justified, compare ordinary supervised tuning, low-rank
adaptation and quantized low-rank adaptation under equal data and evaluation.
Quantized adaptation is a memory technique, not an accuracy claim.

Sequence-level distillation may train a smaller student from the best evaluated
teacher pipeline. The teacher is a measured system of tools plus a model, not a
provider name. Distillation transfers errors too, so render validation and
held-out evaluation remain mandatory.

Models, processors, adapters and datasets are separately versioned optional
artifacts with digests, model cards, dataset datasheets, license lineage and
training manifests. They never redefine `nuif-core` or travel as ordinary
document resources.

Private/authenticated captures default to local processing, no retention and no
training. Remote transfer, telemetry, retention and training are independent
consent/policy decisions.

## Maturity boundary

The current `0.1.0-alpha.3` label belongs to the developer editor application.
It provides no evidence that the broad image/font resource, live browser
capture or screenshot reconstruction accuracy profiles are complete. A
deterministic package plus narrow PNG/static-font segments and fixed
provider-input capture/reconstruction contract baseline are implemented, but
their deliberately narrow evidence does not promote the broader profiles.

Promotion requires the package/resource cross-writer fixtures, pinned capture
reproduction, baseline/closed-loop/calibration harness, leak-resistant licensed
evaluation data, independent result reproduction and at least one real edit
workflow that benefits from the inferred semantics. Until then the work is
research and proposed specification text, not a standard or production
reconstruction promise.

## Primary research records

- `nuif:research:resource-packaging-and-source-capture-synthesis`
- `nuif:research:model-agnostic-screenshot-reconstruction-and-training`
- `nuif:research:epub-ocf-package-container`
- `nuif:research:oci-resource-descriptors`
- `nuif:research:opentype-font-embedding-and-portability`
- `nuif:research:ttf-parser`
- `nuif:research:fontations`
- `nuif:research:chromium-source-backed-ui-capture`
- `nuif:research:design2code-real-world-benchmark`
- `nuif:research:pix2struct-screenshot-parsing-pretraining`
- `nuif:research:screenai-ui-annotation`
- `nuif:research:confidence-calibration-and-selective-prediction`
- `nuif:research:lora-low-rank-adaptation`
- `nuif:research:qlora-quantized-adaptation`
- `nuif:research:sequence-level-knowledge-distillation`
