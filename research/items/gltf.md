---
id: nuif:research:gltf
kind: standard
status: active
title: glTF core and extension registry model
source:
  url: https://github.com/KhronosGroup/glTF
  authors: [Khronos Group]
  published_at: null
  license: Khronos specification license
retrieved_at: 2026-08-29
tags: [extensions, registry, capabilities, conformance]
confidence: 0.99
claims: [nuif:claim:opaque-preservation]
relations: []
links:
  spec: [spec/07-extensions-and-dialects.md]
  adr: []
  rfc: [rfcs/0002-extension-preservation.md]
  code: []
  experiments: []
---
# Summary
glTF keeps a focused base format and grows through registered extensions. `extensionsUsed` and `extensionsRequired` let consumers distinguish optional data from capabilities required for correct loading/rendering. Prefix governance separates Khronos, multi-vendor and vendor namespaces.

## NUIF relevance
Borrow explicit used/required capability declarations and staged extension governance. NUIF additionally needs a normative opaque-preservation rule so an editor can round-trip unknown payloads without understanding them.
