---
id: nuif:research:wasm-component-model
kind: standard
status: active
title: WebAssembly Component Model and WIT
source:
  url: https://component-model.bytecodealliance.org/
  authors: [Bytecode Alliance, WebAssembly community]
  published_at: null
  license: open specifications
retrieved_at: 2026-08-29
tags: [idl, components, plugins, wasm, ffi]
confidence: 0.95
claims: []
relations: []
links:
  spec: [spec/12-cli-api-and-automation.md]
  adr: []
  rfc: []
  code: []
  experiments: []
---
# Summary
WIT defines language-neutral interfaces and worlds for WebAssembly components and drives generated bindings across Rust, C/C++, Go, C# and other languages. The ecosystem also demonstrates text/binary interface round-trips and introspection.

## NUIF relevance
Potential future plugin/adapter ABI, especially for sandboxed importers/exporters. Do not couple the initial core API to Component Model stability; keep an adapter boundary that can later expose WIT.
