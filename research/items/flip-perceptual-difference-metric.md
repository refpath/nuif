---
id: nuif:research:flip-perceptual-difference-metric
kind: paper
status: reviewed
title: FLIP perceptual difference evaluator for rendered images (LDR-FLIP, HDR-FLIP, NVIDIA reference implementation)
source:
  url: https://doi.org/10.1145/3406183
  doi: 10.1145/3406183
  repository: https://github.com/NVlabs/flip
  authors: [Pontus Andersson, Jim Nilsson, Tomas Akenine-Möller, Magnus Oskarsson, Kalle Åström, Mark D. Fairchild]
  published_at: "2020-08-26"
  license: "Paper: ACM (PACMCGIT 3(2)); reference implementation: BSD-3-Clause (NVIDIA Corporation)"
retrieved_at: 2026-08-29
tags: [perceptual-metric, image-difference, tolerance, conformance, rendering, flip, testing]
confidence: 0.92
claims: []
relations:
  - type: compares_to
    target: nuif:research:ssim-and-classical-image-metrics
    note: FLIP is validated against SSIM, S-CIELAB, SMAPE, LPIPS and others in a user study; the paper documents SSIM's lack of a viewing-distance model.
  - type: implements
    target: nuif:research:vello-testing-and-cpu-reference
    note: Vello's snapshot and GPU/CPU comparison tests pool FLIP error maps via the nv-flip crate with a mean threshold.
  - type: related_to
    target: nuif:research:gpu-rendering-nondeterminism
    note: Perceptual pooling is the tolerance tier used when GPU output cannot be bit-exact.
  - type: related_to
    target: nuif:research:vello
    note: Candidate perceptual gate for the Vello/wgpu interactive backend.
links:
  spec: [spec/00-conformance.md, spec/05-geometry-paint-text.md]
  adr: [adrs/0003-reference-renderer.md]
  rfc: []
  code: [crates/nuif-render, conformance/PLAN.md]
  experiments: []
---

# Summary

FLIP (Andersson et al., Proc. ACM Comput. Graph. Interact. Tech. 3(2), Article 15, 2020) is a full-reference image difference evaluator built for the case where a reference and a test rendering are alternated ("flipped") on the same display. It produces a per-pixel error map in [0, 1] and a set of pooled statistics. The model is parameterised by pixels per degree (PPD), which couples the metric to display size, resolution and viewing distance. Two parallel pipelines are combined: a colour pipeline (contrast-sensitivity filtering in an opponent colour space, Hunt-adjusted L*a*b*, HyAB distance, non-linear remapping) and a feature pipeline (edge and point detectors whose scale depends on PPD). HDR-FLIP (Andersson, Nilsson, Shirley, Akenine-Möller, Eurographics 2021 Short Papers, DOI 10.2312/egs.20211015) extends the method to high-dynamic-range inputs by compositing LDR-FLIP maps over a range of exposures. NVIDIA publishes a BSD-3-Clause reference implementation with C++, CUDA, Python (nanobind) and PyTorch entry points.

The source facts below come from the paper text, the `FLIP.h` header and the repository READMEs. The NUIF interpretation is confined to the "NUIF relevance" section.

## Evidence

- Venue and identity: Proceedings of the ACM on Computer Graphics and Interactive Techniques, vol. 3, no. 2, Article 15, pp. 15:1–15:23, August 2020, DOI 10.1145/3406183 (paper header; Crossref record; `misc/LDRFLIP.txt` in the repository).
- Design target: the abstract states the map "approximates the difference perceived by humans when alternating between two images" (paper, abstract, p. 15:1).
- PPD model: Equation 1, Section 4.1.1, computes PPD from observer distance d, monitor width W_m (metres) and horizontal resolution W_p (pixels). The paper's default setup is a 0.69 m × 0.39 m, 3840 × 2160 monitor viewed at 0.70 m, giving p = 67 PPD (p. 15:7). The header uses `calculatePPD(0.7f, 3840.0f, 0.7f)` with a 0.7 m width (`src/cpp/FLIP.h`, line 118); both round to 67.
- Colour pipeline: sRGB is linearised, converted to XYZ and to the YyCxCz opponent space; each channel is convolved with a Gaussian approximation of a contrast sensitivity function whose radius is r = ⌈3 σ_max p⌉ (Equation 6, Section 4.1.1); filtered colours are clamped to the RGB cube, converted to L*a*b* under D65 and Hunt-adjusted (Section 4.1.2); the HyAB distance ΔE_HyAB = |ΔL*| + sqrt(Δa*² + Δb*²) is used (Equation 8, Section 4.1.3) because Euclidean L*a*b* distances are only valid for small differences and rendering errors such as fireflies are large.
- Colour remapping constants: distances are raised to q_c = 0.7; the maximum Hunt-adjusted HyAB distance (blue vs. green) is c_h,max = 203, giving c_max = 41 after the power; the range [0, p_c c_max) maps linearly to [0, p_t) and [p_c c_max, c_max] to [p_t, 1], with p_c = 0.4 and p_t = 0.95 (Section 4.1.3, Figure 4). The header stores the same constants as `gqc = 0.7f`, `gpc = 0.4f`, `gpt = 0.95f`, `gw = 0.082f`, `gqf = 0.5f` (`FLIP.h`, lines 128–132).
- Feature pipeline: edge and point features are Gaussian first- and second-derivative responses on the achromatic channel, kernel radius ⌈3σ(w, p)⌉ in pixels, so feature scale follows PPD (Section 4.2.1). The feature difference is ΔE_f = (max(|‖∇R‖ − ‖∇T‖|, |‖∇²R‖ − ‖∇²T‖|) / √2)^q_f with q_f = 1/2 (Equation 9, Section 4.2.2).
- Combination: ΔE = (ΔE_c)^(1 − ΔE_f) (Equation 10, Section 4.3). A feature difference can only increase the colour difference; ΔE = 0 when filtered colours are identical; ΔE = 1 for the blue/green extreme or ΔE_f = 1. The parameters q_c, q_f, p_c, p_t "were chosen based on visual inspection" of many image pairs (Section 4.3).
- Pooling: Section 5 argues that pooling loses information and should be avoided where possible; it proposes a weighted histogram (bucket count multiplied by bucket-centre FLIP value, normalised per megapixel) and single-value summaries. The tool reports mean, weighted median, first and third weighted quartiles, min and max (`src/python/README.md`, example output).
- Validation: 42 subjects, 21 image pairs (11 rendered, 10 natural) at 67 PPD, comparing FLIP with Butteraugli, a CNN visibility metric, HDR-VDP-2, LPIPS, PieAPP, Euclidean RGB distance, S-CIELAB, SMAPE and SSIM; FLIP obtained the best average score (2.1) with non-overlapping 95% confidence intervals against the others on average (Section 6.2, pp. 15:19–15:20). SSIM scored best on two individual pairs (R2, N10).
- Critique of SSIM: the paper notes SSIM "does not consider viewing distance and pixel size" and yields uninterpretable negative values shown as a separate colour in its maps (Section 6.1, p. 15:17; Section 6, p. 15:14).
- HDR-FLIP: Eurographics 2021 Short Papers, DOI 10.2312/egs.20211015 (`misc/HDRFLIP.txt`); it computes "a composite visualization over a number of low dynamic range error maps of exposure compensated and tone mapped image pairs" (NVIDIA publication page abstract). The header exposes `startExposure`, `stopExposure`, `numExposures` (automatic when left at infinity/−1) and tone mappers `reinhard`, `aces` (default) and a third (Hable) (`FLIP.h`, lines 119–122, 138–144, 1395–1420). Since v1.7 automatic exposure handles references whose median luminance is 0 (repository README).
- Tool chapter: Andersson, Nilsson, Akenine-Möller, "Visualizing and Communicating Errors in Rendered Images", Ray Tracing Gems II, ch. 19, pp. 301–320, DOI 10.1007/978-1-4842-7185-8_19 (`misc/FLIP.txt`; Crossref).
- Implementations: single header `src/cpp/FLIP.h` for CPU and CUDA (`-DFLIP_ENABLE_CUDA=ON`), Python package `flip-evaluator` via nanobind, PyTorch loss `src/pytorch/flip_loss.py` (repository README).
- Metric reproducibility caveat: the Python README states output "might differ slightly between the different operative systems"; the repository's own tests compare means to six decimal places while "not all error map pixels are identical" across Windows, Linux and macOS (`src/python/README.md`, "Python (API and Tool)").
- Practical thresholds observed in a Rust renderer: Vello's smoke snapshot tests call `assert_mean_less_than(0.01)`; a known-issue reproduction uses `0.001`; the comparison helper rejects thresholds ≥ 0.1 as implausible for a passing test; PPD is `nv_flip::DEFAULT_PIXELS_PER_DEGREE = 67.0` (`vello_tests/tests/smoke_snapshots.rs`, lines 29, 47, 75, 119; `vello_tests/tests/known_issues.rs`, line 55; `vello_tests/src/compare.rs`, lines 45–48; `nv-flip/src/lib.rs`, line 15).

## Mechanism

```text
Inputs: reference R, test T (sRGB, same size), pixels per degree p
p = d * (W_p / W_m) * (pi / 180)                       # Eq. 1; 0.70 m, 3840 px, 0.69 m -> 67

Colour pipeline (Section 4.1)
  R_lin, T_lin   = sRGB^-1(R), sRGB^-1(T)
  R_o,   T_o     = XYZ->YyCxCz(RGB->XYZ(.))
  for c in {Yy, Cx, Cz}: R_o[c] = G_c(p) * R_o[c]      # CSF-derived Gaussian, radius ceil(3*sigma_max*p)
  clamp back to RGB cube, convert to L*a*b* (D65), apply Hunt adjustment to a*, b*
  dE_hyab = |dL*| + sqrt(da*^2 + db*^2)               # Eq. 8
  e = dE_hyab ^ q_c                                   # q_c = 0.7, c_max = 41
  dE_c = e < p_c*c_max ? e * p_t/(p_c*c_max)
                       : p_t + (e - p_c*c_max)/(c_max - p_c*c_max) * (1 - p_t)   # p_c = 0.4, p_t = 0.95

Feature pipeline (Section 4.2)
  edge  = |grad G_sigma(p) * Y|,   point = |laplacian-like second derivative|
  dE_f = ( max(|edge_R - edge_T|, |point_R - point_T|) / sqrt(2) ) ^ q_f      # q_f = 0.5

Combination (Eq. 10)
  FLIP = dE_c ^ (1 - dE_f)                            # per pixel, in [0, 1]

Pooling (Section 5)
  weighted histogram; mean; weighted median; weighted quartiles; min; max
HDR-FLIP
  for exposure in linspace(c_start, c_stop, N): map = LDR-FLIP(tonemap(R*2^exposure), tonemap(T*2^exposure))
  composite = per-pixel maximum over exposures; exposure map records the argmax
```

The HDR compositing rule (per-pixel maximum with an exposure map) is stated in the Eurographics paper abstract as a composite over exposure-compensated maps; the exact aggregation is documented in the tool's exposure-map output naming (`src/python/README.md`).

## NUIF relevance

**Borrow**
- Use pooled FLIP mean at a fixture-declared PPD as the tolerance statistic for the non-normative GPU tier of the `render` suite, because it is validated against human judgement of rendering artefacts and is already exercised in the Rust ecosystem via `nv-flip`.
- Adopt the weighted-histogram report (not only the mean) in conformance reports, because the paper documents that single-value pooling discards localisation information.

**Adapt**
- Derive PPD from the evaluation context's pixel ratio and a declared viewing model instead of the fixed 67, because NUIF fixtures are rendered at multiple device pixel ratios and the metric's feature scale depends on p.
- Record the FLIP implementation version and platform with each result, because the metric itself is not pixel-identical across operating systems.

**Reject**
- Do not use FLIP as the gate for the deterministic CPU reference path, because exact pixel equality is the normative requirement there and FLIP would mask real semantic regressions such as off-by-one clipping.
- Do not use HDR-FLIP, because NUIF documents specify display-referred sRGB output rather than scene-referred radiance.

## Open questions

- Which per-fixture-class thresholds (text, thin strokes, gradients) are appropriate; Vello's 0.01 mean is an engineering choice, not a published recommendation.
- Whether a pure-Rust FLIP port with a pinned evaluation order is required so that the metric itself is reproducible across CI platforms.
- How to report FLIP for fixtures with transparent backgrounds, since the metric consumes opaque RGB and Vello discards alpha before pooling.
