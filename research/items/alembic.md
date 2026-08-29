---
id: nuif:research:alembic
kind: implementation
status: reviewed
title: Alembic as a baked, time-sampled, non-procedural interchange cache
source:
  url: http://www.alembic.io/
  repository: https://github.com/alembic/alembic
  authors: [Sony Pictures Imageworks, Industrial Light & Magic, Alembic contributors]
  published_at: "v1.8.12 (2026-07-02)"
  license: BSD-3-Clause
retrieved_at: 2026-08-29
tags: [interchange, baked-geometry, time-sampling, cache, deduplication, ogawa, non-procedural, resolved-only]
confidence: 0.9
claims: [nuif:claim:authored-resolved, nuif:claim:multi-level-ir]
relations:
  - type: contradicts
    target: nuif:research:openusd
    note: Alembic deliberately stores only computed results; USD stores composable authored opinions.
  - type: compares_to
    target: nuif:research:openusd-composition-and-crate
    note: An Alembic archive is approximately a flattened USD stage restricted to time-sampled geometry.
  - type: related_to
    target: nuif:research:content-addressed-versioning
    note: Ogawa deduplicates samples by 128-bit content hash.
  - type: related_to
    target: nuif:research:renderers
    note: Caches are the handoff representation between animation and lighting/rendering.
links:
  spec: [spec/09-provenance-and-fidelity.md, spec/01-model.md, spec/08-serialization.md]
  adr: [adrs/0004-serialization.md]
  rfc: [rfcs/0003-authored-resolved-provenance.md, rfcs/0001-multi-level-document-model.md]
  code: []
  experiments: []
---

# Summary

Alembic is an open interchange framework from Sony Pictures Imageworks and ILM that "distills complex, animated scenes into a non-procedural, application-independent set of baked geometric results". Its documentation states that it is "very specifically NOT concerned with storing the complex dependency graph of procedural tools", that it "is not a dependency graph, nor a procedural data transformation tool", and that it would not be used "to make lossless round trips out of and into the same computation context". The data model is an archive containing a hierarchy of objects, each with compound, scalar and array properties whose values are stored as indexed samples related to time by a `TimeSampling` (uniform, cyclic or acyclic). Schemas (`AbcGeom` PolyMesh, Xform, Camera, Curves, Points, SubD, ...) are conventions over this property model; ad hoc data lives in `userProperties`.

The Ogawa back end (Alembic 1.5.0, 2013) replaced HDF5 with a format optimized for multi-threaded reads and deduplicates array samples by a MurmurHash3 128-bit digest so repeated samples are written once. `AbcCoreLayer` adds read-time layering of multiple archives (sparse overrides), and `AbcCoreFactory` selects a back end on open. Alembic is therefore the canonical example of a resolved-only cache: it is what NUIF's "resolved layer" would look like if the authored layer were discarded.

## Evidence

- Purpose statement, "baked geometric results", analogy to rendered images, and the sentence that Alembic "will not attempt to store a representation of the network of computations (rigs, basically)" — http://www.alembic.io/ (Introduction, retrieved 2026-08-29).
- "Alembic Is Not" list: not a dependency graph, not a replacement for native scene formats, not an asset manager, not a rigging storage solution; "Would Not Be Used" list includes transporting procedural rigs and "lossless round trips out of and into the same computation context" — http://www.alembic.io/ (sections "What is Alembic?", retrieved 2026-08-29).
- Positioning as "the greatest common divisor between applications, the 'periodic table of cg primitives'" — same page.
- `TimeSamplingType` semantics: Uniform (start time plus fixed interval), Cyclic (N samples distributed over a cycle, e.g. shutter open/close), Acyclic (strictly increasing explicit times enabling bisection search for floor/ceiling/nearest) — `lib/Alembic/AbcCoreAbstract/TimeSamplingType.h` lines 48–138 (master, retrieved 2026-08-29).
- Archive writer pools `TimeSampling` objects; index 0 is reserved for identity uniform sampling; array compression level is a hint implementations may ignore — `lib/Alembic/AbcCoreAbstract/ArchiveWriter.h` lines 63–84.
- `ArraySampleKey` holds `numBytes`, original and read POD, and a `Digest`; `ArraySample::getKey` computes it with `MurmurHash3_x64_128` — `AbcCoreAbstract/ArraySampleKey.h` lines 47–58; `ArraySample.cpp` lines 73–126; `Util/Digest.h` stores `uint64_t words[2]`.
- Ogawa `WrittenSampleMap`: "A Written Sample ID is a receipt that contains information that refers to the exact location in an Ogawa file that a sample was written to" and is "used to 'reuse' an already written sample by linking it from the previous usage"; `find(key)` returns the prior receipt — `lib/Alembic/AbcCoreOgawa/WrittenSampleMap.h` lines 48–70.
- Ogawa release notes (Alembic 1.5.0, 2013-07-22): 5–15% smaller files, ~4x single-thread and up to 25x multi-thread read improvement over HDF5, HDF5 kept for backward compatibility, "explicit hierarchical deduplication (OObject::addChildInstance)", hierarchical hash keys (`IObject::getPropertiesHash`, `getChildrenHash`), `AbcCoreFactory::IFactory` — `NEWS.txt` lines 1175–1200.
- Library layering: `AbcCoreAbstract`, `AbcCoreOgawa`, `AbcCoreHDF5`, `AbcCoreLayer`, `AbcCoreFactory`, `Abc`, `AbcGeom`, `AbcMaterial`, `AbcCollection`, `Ogawa`, `Util` — repository listing `lib/Alembic/` (retrieved 2026-08-29).
- `AbcCoreLayer::OrImpl` composes an object from a vector of top-level `ObjectReaderPtr`s across archives (`std::vector<AbcA::ObjectReaderPtr>& iTops`) — `lib/Alembic/AbcCoreLayer/OrImpl.h` lines 50–62; `ArImpl::getTop` collects each archive's top object in list order — `ArImpl.cpp` lines 138–156.
- Layer merge rules: `CprImpl::init` iterates compounds in order, honours property metadata `prune == "1"` ("since pruning is more destructive, it trumps replace") and `replace == "1"` (clears previously merged children), and merges compounds child-wise — `lib/Alembic/AbcCoreLayer/CprImpl.cpp` lines 203–290.
- HDF5 is optional (`-DUSE_HDF5=ON`); dependencies are CMake 3.29+, C++11, Imath 3 — `README.txt` lines 1–60.
- License BSD-3-Clause with Lucasfilm and Sony Pictures Imageworks copyright — `LICENSE.txt` lines 1–12; latest release v1.8.12 published 2026-07-02 (GitHub releases API, retrieved 2026-08-29).
- Recent releases are dominated by fuzzer-driven hardening fixes to Ogawa readers (buffer overruns on malicious dimensions, excessive allocation, infinite recursion) — `NEWS.txt` lines 5–380.
- USD documents `custom` properties as equivalent to Alembic `userProperties` — OpenUSD `pxr/usd/usd/property.h` lines 179–185 (cross-reference).

## Mechanism

An Alembic archive is an immutable, write-once tree: one top object, child objects with headers (name, metadata), and per-object compound properties containing scalar or array properties. Every property value is a sample addressed by integer index; the property's `TimeSampling` maps indices to times. Uniform and cyclic samplings are described by a start time and a period; acyclic sampling stores an explicit strictly increasing time list, and readers use bisection for floor, ceiling and nearest lookups. Static data is a property with one sample. Nothing in the format encodes how a sample was produced; interpolation, rig evaluation and simulation are all upstream. Consequently, there are no override semantics, no references between archives in the core model and no notion of an unresolved value.

Ogawa is a group/data tree with fixed-size headers designed for lock-free parallel reads. On write, each array sample is hashed (MurmurHash3 128-bit over bytes plus POD size); the `WrittenSampleMap` maps the key to a receipt with the file location of the previously written sample, so repeated samples (typical for static or partially animated properties) are written once and referenced thereafter. `addChildInstance` extends this to whole object subtrees. Hierarchical hashes on objects allow subgraph comparison across archives without decoding samples. Layering (`AbcCoreLayer`) is a read-time overlay: the factory opens several archives and presents a merged object hierarchy. Compound properties are merged child-wise in archive-list order; a property whose metadata carries `replace = "1"` discards previously merged data for that name, and `prune = "1"` removes the name entirely, with prune taking precedence over replace. This is the format's only override mechanism, it operates on property names rather than semantic identities, and it is external to the archive.

Loss is by design: identity is a path in the object hierarchy, geometry is explicit vertex data, and the mapping back to authoring constructs (rig controls, procedural nodes, construction history) exists only in the producing application. The documentation states the intended usage boundary explicitly: hand-off between disciplines, not round trips into the originating computation context.

## NUIF relevance

**Borrow**
- Use content-hash deduplication of resolved samples (Ogawa `WrittenSampleMap`) for NUIF resolved caches keyed by evaluation context, so repeated layouts across breakpoints or states are stored once.
- Adopt explicit sampling-domain descriptors (the `TimeSampling` pattern) for NUIF resolved state indexed by evaluation context (viewport, theme, state), with identity context reserved as index 0.
- Reuse the "cache for hand-off" framing to define NUIF's resolved-only export profile as a legitimate but declared lossy lowering for renderers and runtimes that do not need authored intent.
- Adopt fuzzer-driven hardening of binary readers as a conformance activity; Alembic's release history shows parsers of baked data are the attack surface.

**Adapt**
- Alembic's `userProperties` are unschematized escape hatches; NUIF must namespace such data as extensions with used/required declarations rather than free-form properties.
- Read-time layering across archives is a useful operational pattern but must be lifted into NUIF's authored composition model with provenance rather than remaining an external merge.
- Hierarchical hashes for subgraph comparison map to NUIF canonical snapshot hashes, but NUIF hashes must exclude transport-only differences per spec/08.

**Reject**
- Resolved-only storage as the interchange truth: Alembic's own documentation excludes lossless round trips, which is exactly the property the NUIF thesis requires (RFC 0003).
- Path-based identity: moving an object in an Alembic hierarchy changes its identity; NUIF identity is semantic and path-independent.
- Absence of an override or opinion model: NUIF resolved state must remain scoped to a context and never replace authored intent, whereas Alembic has no authored layer to protect.

## Open questions

- Whether NUIF should specify a standalone "resolved cache" package profile (Alembic-like) with a mandatory back-reference to the authored document hash, or only allow resolved caches embedded in a full package.
- How much of Ogawa's parallel-read layout is relevant to UI documents whose resolved data is small relative to 3D caches.
- Whether hierarchical content hashes should be normative for NUIF diff of resolved output across implementations.
