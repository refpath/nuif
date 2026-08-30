---
id: nuif:research:ttf-parser
kind: implementation
status: verified
title: ttf-parser retirement after RUSTSEC-2026-0192
source:
  url: https://rustsec.org/advisories/RUSTSEC-2026-0192.html
  authors: [RustSec Advisory Database contributors, ttf-parser contributors]
  published_at: "2026-06-29"
  license: CC0-1.0
retrieved_at: 2026-08-30
tags: [fonts, opentype, rust, parser, maintenance, security]
confidence: 0.99
claims: [nuif:claim:bounded-untrusted-input]
relations:
  - type: related_to
    target: nuif:research:opentype-font-embedding-and-portability
    note: Records why the former metadata dependency was retired; it does not decide redistribution rights.
  - type: compares_to
    target: nuif:research:fontations
    note: Skrifa replaced the direct dependency while a pinned HarfBuzz capture preserves an external oracle.
links:
  spec: [spec/05-geometry-paint-text.md, spec/11-security.md]
  adr: []
  rfc: [rfcs/0010-portable-resource-package.md]
  code: [crates/nuif-font, conformance/font/harfbuzz-14.4.0-ahem.json]
  experiments: [nuif:experiment:font-resource-static-baseline]
---

# Summary

NUIF previously pinned `ttf-parser` 0.25.1 for the static package-font metadata
path. RustSec advisory RUSTSEC-2026-0192 classifies the crate as unmaintained,
lists no patched versions and recommends Skrifa as an alternative. NUIF removed
the dependency instead of suppressing the advisory or maintaining an
unreviewed parser fork.

The useful parts of the former design remain: NUIF still owns sfnt directory,
range, packing, checksum, size and embedding-policy checks. Skrifa 0.46.2 now
provides names, character maps and metrics after those checks. A committed
`hb-info` 14.4.0 metadata capture supplies independent evidence for the exact
Ahem fixture without requiring a foreign executable in every test run.

## Evidence

- RUSTSEC-2026-0192 marks all `ttf-parser` versions unmaintained, lists no
  patched versions and names Skrifa as the alternative. Locator: RustSec
  advisory, issued 2026-06-29, retrieved 2026-08-30:
  https://rustsec.org/advisories/RUSTSEC-2026-0192.html.
- The project's security-reporting discussion remained unresolved and directed
  private reports to HarfBuzz infrastructure that did not cover the standalone
  Rust crate. Locator: issue 217, retrieved 2026-08-30:
  https://github.com/harfbuzz/ttf-parser/issues/217.
- The maintenance discussion records the original author's limited time and
  identifies a community fork, but does not restore an upstream release and
  review boundary. Locator: issue 230, retrieved 2026-08-30:
  https://github.com/harfbuzz/ttf-parser/issues/230.

## Mechanism

The retired implementation constructed `ttf_parser::Face` only after NUIF's
own bounded sfnt validation and used it for names, Unicode coverage, metrics and
embedding flags. The replacement keeps the exact surrounding policy and reads
required `head`, `maxp` and `OS/2` fields independently. It requires Skrifa's
units-per-em and glyph count to agree with the direct fields and derives
embedding restrictions from the profile's explicit bit policy.

The conformance gate no longer compares two in-process Rust parsers. It binds a
pinned HarfBuzz capture to the exact font digest and compares units, glyph
count, family, table inventory and a normalized Unicode-scalar hash. This is a
stronger maintenance boundary but remains only one external fixture.

## Alternatives and decision

Ignoring the advisory would make a known maintenance failure an undocumented
release exception. Adopting `xberg-ttf-parser` would transfer trust to a smaller
fork without improving NUIF's declared static profile. FreeType would add a C
ABI and native deployment surface for a metadata-only path. Continuing a local
fork would require ongoing parser security ownership that the project has not
claimed.

NUIF therefore **retires** `ttf-parser`, **adopts** the already pinned Skrifa
stack for production metadata, and **retains** external HarfBuzz evidence as a
versioned golden. Cargo Deny must pass with no advisory exception.

## NUIF relevance

**Borrow** the former small immutable-parser boundary and fail-closed profile.

**Adapt** the production implementation to Skrifa behind NUIF-owned validation,
with direct required-table checks and a separately produced HarfBuzz golden.

**Reject** advisory suppression, an unreviewed maintenance fork, parser
acceptance as a license decision, and claims of broad OpenType conformance.

## Open questions

- Extend the external corpus beyond Ahem before adding TTC, CFF/CFF2, variable,
  color, bitmap or WOFF2 profiles.
- Add a reproducible pinned HarfBuzz capture job without making native HarfBuzz
  a release-build dependency.
- Consider FreeType or a browser font stack as a third oracle only with a
  measured sandbox and exact version provenance.
