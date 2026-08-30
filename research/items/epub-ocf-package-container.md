---
id: nuif:research:epub-ocf-package-container
kind: standard
status: reviewed
title: EPUB 3.3 OCF package and resource-manifest discipline
source:
  url: https://www.w3.org/TR/epub-33/
  authors: [W3C]
  published_at: 2025-06-24
  license: W3C Document License
retrieved_at: 2026-08-30
tags: [package, zip, manifest, resources, fonts, security, interoperability]
confidence: 0.99
claims: [nuif:claim:resource-identity-separation, nuif:claim:bounded-untrusted-input]
relations:
  - type: related_to
    target: nuif:research:content-addressed-versioning
    note: OCF supplies the physical-container precedent; content addressing supplies immutable resource identity.
  - type: related_to
    target: nuif:research:penpot
    note: Both use ZIP packages, but OCF has a mature normative container profile and explicit resource manifest.
links:
  spec: [spec/08-serialization.md, spec/11-security.md]
  adr: [adrs/0004-serialization.md]
  rfc: [rfcs/0009-profile-zero-resource-budgets.md]
  code: []
  experiments: [nuif:experiment:portable-package-resources]
---

# Summary

EPUB 3.3 separates an abstract publication container from its physical OCF ZIP
representation. Publication resources are declared in a package manifest and
are normally carried inside the container; remote resources are a deliberate
exception. OCF narrows ZIP to an interoperable, inspectable subset and reserves
an uncompressed first `mimetype` member for early format identification.

NUIF can borrow the package discipline without borrowing EPUB's publication
model. The important precedent is that a portable document does not depend on
an unconstrained filesystem or arbitrary ZIP behavior: it has a manifest,
well-defined members, restricted compression and explicit external resources.

## Evidence

- EPUB 3.3 §3.3 requires publication resources to be listed in the package
  document manifest and normally bundled in the container. §3.6 defines remote
  resources as a distinct resource-location case.
- §4.2 defines one rooted abstract filesystem, reserves `mimetype`, and keeps
  container configuration separate from publication resources.
- §4.3.2 prohibits split or spanned archives and ZIP encryption, permits only
  stored and Deflate entries, and requires UTF-8 file names.
- §4.3.3 requires `mimetype` to be the first member, stored and unencrypted,
  with no extra field or surrounding whitespace. The value identifies the
  package before processing the rest of the archive.
- §4.4 treats embedded-font handling as an explicit package concern rather
  than assuming that every font may be redistributed.

## Mechanism

An OCF reader identifies the archive from a fixed first member, validates the
ZIP profile, resolves one package document, then obtains the declared resource
set from its manifest. Physical member paths locate bytes; the package document
defines their semantic role. Those two responsibilities are not conflated.

For NUIF this suggests a similarly narrow ZIP envelope:

```text
mimetype                    fixed first stored member
manifest.cbor               descriptors and semantic roots
document.cbor               canonical semantic document
blobs/sha256/<hex-digest>   immutable resource bytes
```

The path is a package locator, not the resource identity. The manifest binds a
media type, byte length and digest to every resource before a decoder consumes
it.

## NUIF relevance

**Borrow** the abstract/physical container separation, first-member media-type
identification, mandatory manifest, rooted paths, UTF-8 names and small ZIP
feature subset.

**Adapt** member identity to content digests and separate the semantic document
hash from the complete package hash. Required portable resources are embedded;
linked resources remain explicit, digest-pinned and unavailable without an
opt-in resolver.

**Reject** publication reading order, EPUB-specific metadata, font obfuscation,
container XML and any assumption that a remote URL is stable resource identity.

## Open questions

- Should the first NUIF package profile permit Deflate at all, or use stored
  blobs initially so package hashing and resource limits remain simplest?
- Which fixed ZIP metadata fields are required for byte-reproducible packages
  across independent writers?
- Does streaming import require the manifest before `document.cbor`, or is the
  fixed ordering only an authoring recommendation after `mimetype`?
