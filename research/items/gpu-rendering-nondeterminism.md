---
id: nuif:research:gpu-rendering-nondeterminism
kind: synthesis
status: reviewed
title: Sources of nondeterminism in GPU rendering and determinism tiers for conformance
source:
  url: https://www.w3.org/TR/webgpu/
  authors: [W3C GPU for the Web Working Group, Khronos Vulkan Working Group, Nathan Whitehead, Alex Fit-Florea, James Demmel, Hong Diep Nguyen, Sylvain Collange, David Defour, Stef Graillat, Roman Iakymchuk]
  published_at: "WebGPU/WGSL living specifications (retrieved 2026-08-29); NVIDIA whitepaper 2011 (rev. current); IEEE TC 2015; Parallel Computing 2015"
  license: "W3C Document License; Khronos specification license; NVIDIA documentation; IEEE; Elsevier"
retrieved_at: 2026-08-29
tags: [determinism, floating-point, gpu, webgpu, wgsl, wgpu, msaa, rasterization, tolerance, conformance, reproducibility]
confidence: 0.88
claims: []
relations:
  - type: extends
    target: nuif:research:webgpu-security
    note: Complements the security record with the portability and reproducibility statements of the same specifications.
  - type: related_to
    target: nuif:research:vello-testing-and-cpu-reference
    note: Vello's fast-math and precision notes are concrete instances of the variance catalogued here.
  - type: related_to
    target: nuif:research:flip-perceptual-difference-metric
    note: Perceptual pooling is the tolerance policy for the non-deterministic tier.
  - type: related_to
    target: nuif:research:webrender-reftests
    note: platform()/swgl predicates and per-platform references are one industrial response to GPU variance.
  - type: related_to
    target: nuif:research:renderers
    note: Motivates the renderer-trait boundary and a CPU reference backend.
links:
  spec: [spec/00-conformance.md, spec/05-geometry-paint-text.md]
  adr: [adrs/0003-reference-renderer.md]
  rfc: []
  code: [crates/nuif-render, conformance/PLAN.md]
  experiments: []
---

# Summary

GPU rasterisation is specified so that several results are permitted for the same input. WGSL does not fix a rounding mode, allows reassociation and fusion of floating-point operations, allows subnormal flushing, permits implementations to assume no NaN or infinity at runtime, gives only ULP bounds for transcendental functions, and gives no error bound at all for derivatives and determinants. WebGPU defines pixel-centre sampling and a standard multisample pattern but leaves pixel-centre-on-edge inclusion, line rasterisation and polygon barycentrics for more than three vertices implementation-dependent; Vulkan only guarantees standard sample locations when `standardSampleLocations` is reported. Independent of the API, IEEE-754 arithmetic is non-associative, so any change in reduction order (thread count, workgroup scheduling, atomics) or in compiler contraction (FMA) changes the low-order bits. Reproducible summation is possible but requires algorithms that renderers do not use. Rendering projects therefore stratify: a CPU reference path with fixed evaluation order for exact comparison, platform-keyed baselines for known-good variants, and perceptual or count-and-delta tolerances for GPU output.

## Evidence

- WGSL rounding: "No rounding mode is specified. An implementation may round an intermediate result up or down." (WGSL §15.7.2 Differences from IEEE-754).
- WGSL finite-math assumption: "Implementations may assume that overflow, infinities, and NaNs are not present during shader execution", and in that case an overflowing runtime expression yields "an indeterminate value of the target type"; implementations may also ignore the sign of zero (WGSL §15.7.2).
- WGSL flush-to-zero: "Any inputs or outputs of operations listed in § 15.7.4 Floating Point Accuracy may be flushed to zero"; other operations must preserve subnormals (WGSL §15.7.2).
- WGSL reassociation and fusion: "An implementation may reassociate operations." and "An implementation may fuse operations if the transformed expression is at least as accurate as the original formulation." (WGSL §15.7.5).
- WGSL accuracy table (f32): `x + y`, `x - y`, `x * y` correctly rounded; `x / y` 2.5 ULP; `exp` 3 + 2|x| ULP; `inverseSqrt` 2 ULP; `cos` absolute error at most 2⁻¹¹ on [−π, π]; `log` absolute error 2⁻²¹ on [0.5, 2]; `min`/`max` on two subnormals may return either input; `dpdx`/`dpdy`/`fwidth` and `determinant` are listed as "Infinite ULP" with notes that implementations "should provide a pragmatically useful" function (WGSL §15.7.4.1).
- WGSL derivatives: invocations in a quad "collaborate to compute approximate partial derivatives"; a derivative call in non-uniform control flow returns "an indeterminate value" (WGSL §15.6.2).
- WGSL portability statement: "WGSL sometimes permits several possible behaviors for a given feature. This is a portability hazard" (WGSL §1, Technical Overview).
- WGSL data races are dynamic errors that "may or may not be detectable" (WGSL §2.3 and §6.5.7 notes on data races).
- WebGPU sampling: with multisampling disabled, fragments are at pixel centres (fract(C) = (0.5, 0.5)) and "If a pixel center is on the edge of the polygon, whether or not it's included is not defined" (WebGPU §23.2.5.4 Polygon Rasterization).
- WebGPU multisample pattern: "Implementations must use the standard sample pattern for the given multisample.count"; count 1: (0.5, 0.5); count 4: (0.375, 0.125), (0.875, 0.375), (0.125, 0.625), (0.625, 0.875) (WebGPU §23.2.5 Rasterization). The same section's polygon step still describes per-pixel sample locations as "implementation-defined" (§23.2.5.4), an inconsistency in the current draft text.
- WebGPU lines and polygons: "The exact algorithm used for line rasterization is not defined, and may differ between implementations" (§23.2.5.2); barycentrics for polygons with more than three vertices are "implementation-dependent" (§23.2.5.3).
- WebGPU invalid data: GPU handling of NaN and infinity in resources is "subject to the accuracy of the GPU hardware implementation of the IEEE-754 standard"; subnormals "may be either preserved or replaced by -0.0 or +0.0"; NaN or Infinity "may be replaced by an indeterminate value" (WebGPU §2.1.5 Invalid Data).
- WebGPU texture LOD: implicit level-of-detail derivation is illustrated only by a non-normative reference to the Vulkan LOD operation (WebGPU GPUSampler section note).
- Vulkan sample locations: standard locations for 1, 2, 4, 8 and 16 samples apply only "If the standardSampleLocations member of VkPhysicalDeviceLimits is VK_TRUE"; otherwise locations are implementation-dependent; the 4-sample table matches WebGPU's (Vulkan specification, Rasterization → Multisampling).
- NVIDIA whitepaper (Whitehead, Fit-Florea, "Precision & Performance: Floating Point and IEEE 754 Compliance for NVIDIA GPUs"): FMA rounds once whereas separate multiply and add round twice (§2.3); rn((A + B) + C) and rn(A + (B + C)) differ (§2); "Different math libraries cannot be expected to compute exactly the same result for a given input" (§5); changing the number of threads in a parallel reduction "rearranges parentheses" and gives different but equally valid results (§5.3); compiler flags `-ftz`, `-prec-div`, `-prec-sqrt` (and `-fmad`) change results (§4.4) (docs.nvidia.com/cuda/floating-point).
- Reproducible reductions: Demmel and Nguyen, "Parallel Reproducible Summation", IEEE Transactions on Computers 64(7):2060–2070, 2015, DOI 10.1109/TC.2014.2345391; Collange, Defour, Graillat, Iakymchuk, "Numerical reproducibility for the parallel reduction on multi- and many-core architectures", Parallel Computing 49:83–97, 2015, DOI 10.1016/j.parco.2015.09.001 (Crossref records). Both establish order-independent summation at extra cost.
- Observed consequences in renderers: Vello attributes non-zero GPU/CPU differences to "fast math on the GPU or different precisions" (`vello_tests/src/compare.rs`) and platform "fast math" on Apple (`vello_tests/README.md`); resvg excludes a gradient fixture for "a SIMD rounding difference" even on the CPU (`crates/resvg/tests/gen-tests.py`); the FLIP tool's own error maps are not pixel-identical across operating systems (`NVlabs/flip src/python/README.md`); WebRender renders CI reftests with OSMesa "to get consistent rendering across platforms" and still annotates `fuzzy-if(platform(swgl),...)` (`gfx/wr/README.md`; `wrench/reftests/text/reftest.list`).

## Mechanism

```text
Sources of variance (S) and where each is permitted
  S1 rounding/contraction:   WGSL 15.7.2 (no rounding mode), 15.7.5 (reassociate, fuse); FMA one rounding vs two
  S2 transcendental ULP:     WGSL 15.7.4.1 (exp, log, cos, /, inverseSqrt bounds; derivatives unbounded)
  S3 subnormals/NaN/Inf:     WGSL 15.7.2 flush-to-zero; finite-math assumption -> indeterminate values
  S4 reduction order:        thread count / workgroup schedule / atomics order change the parenthesisation of sums
  S5 coverage:               pixel-centre-on-edge undefined (WebGPU 23.2.5.4); line algorithm undefined (23.2.5.2)
  S6 multisampling:          standard pattern required by WebGPU; Vulkan only with standardSampleLocations
  S7 texture filtering/LOD:  LOD selection non-normative; filtering precision unspecified
  S8 compiler/driver:        naga/tint/dxc/metal compilers apply different contractions and reassociations

Determinism tiers (NUIF interpretation)
  Tier 0  bit-exact:  CPU reference, fixed evaluation order, no FMA or pinned FMA, single or deterministic multithreading;
                      policy: |a - b| == 0 for all channels (resvg, vello_cpu f32 tolerance 0)
  Tier 1  bounded:    same algorithm, different SIMD/thread schedule or u8 pipeline;
                      policy: per-channel |a - b| <= t (t = 1..2) and count(diff) <= n (WPT/WebRender/Gold fuzzy)
  Tier 2  perceptual: GPU backends across adapters and drivers;
                      policy: mean FLIP(ppd) < tau (Vello 0.01), plus baseline keyed by (backend, adapter class, driver)
  Result record must carry: renderer id+version, tier, backend, adapter/driver, pixel ratio, fixture id, context hash

Reproducible-by-construction rules for Tier 0
  sum in fixed order (no atomics-based accumulation); avoid dpdx/fwidth; avoid transcendental functions in coverage;
  quantise anti-aliasing coverage to a fixed grid; avoid MSAA (use analytic or supersampled area coverage);
  disable fast-math flags; avoid subnormal-dependent branches
```

## NUIF relevance

**Borrow**
- Adopt the three-tier stratification (bit-exact CPU, bounded per-channel, perceptual GPU) with mandatory result metadata, because every surveyed project converged on some form of it and the WebGPU/WGSL texts make a single-tier exact policy impossible for GPU output.
- Adopt WebGPU's standard sample pattern and pixel-centre rule as the definition of coverage in spec/05 where NUIF specifies anti-aliasing semantics, because it is the only normative sample geometry shared by WebGPU and Vulkan (when the limit is present).

**Adapt**
- Specify NUIF's normative coverage as area coverage computed in a fixed evaluation order on the CPU path rather than as MSAA, because the pixel-centre-on-edge rule is undefined and MSAA sample positions are only conditionally standard.
- Turn the WGSL accuracy table into a fixture design rule: render fixtures should not depend on the low-order bits of `exp`, `log`, `cos`, derivatives or determinants, because those are the operations with loose or unbounded error.

**Reject**
- Do not attempt bit-exact conformance across GPU backends, because reassociation, fusion, flush-to-zero and indeterminate values are permitted by the shader language itself.
- Do not use reproducible-summation algorithms in the interactive renderer, because their cost is unjustified when a CPU reference path already provides Tier 0.

## Open questions

- Whether `wgpu`/`naga` exposes or could expose a "no contraction" or "strict" shader compilation option for the Vello backend to shrink Tier 2 variance.
- How to detect at runtime whether an adapter uses standard sample locations (Vulkan reports it; Metal and D3D12 mappings through wgpu are not documented here).
- Whether the WebGPU draft's inconsistency between "must use the standard sample pattern" and "locations, which are implementation-defined" is resolved in a later editor's draft.
