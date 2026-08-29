---
id: nuif:research:ssim-and-classical-image-metrics
kind: paper
status: reviewed
title: SSIM, MS-SSIM, PSNR and exact-pixel comparison policies for rendering tests
source:
  url: https://doi.org/10.1109/TIP.2003.819861
  doi: 10.1109/TIP.2003.819861
  authors: [Zhou Wang, Alan C. Bovik, Hamid R. Sheikh, Eero P. Simoncelli]
  published_at: "2004-04"
  license: "IEEE copyright; author preprint at ece.uwaterloo.ca/~z70wang/publications/ssim.pdf"
retrieved_at: 2026-08-29
tags: [perceptual-metric, ssim, psnr, image-difference, tolerance, conformance, testing]
confidence: 0.9
claims: []
relations:
  - type: compares_to
    target: nuif:research:flip-perceptual-difference-metric
    note: FLIP's user study places SSIM below FLIP on average; SSIM lacks a viewing-distance model.
  - type: related_to
    target: nuif:research:resvg-test-suite
    note: resvg uses a per-channel threshold of 1 with zero tolerated differing pixels, an exact-comparison policy.
  - type: related_to
    target: nuif:research:skia-gold-and-gm-tests
    note: WPT fuzzy matching and Gold's fuzzy matcher are count-and-delta policies rather than perceptual indices.
  - type: related_to
    target: nuif:research:vello-testing-and-cpu-reference
    note: Vello's CPU f32 pipeline uses tolerance 0 per component; GPU paths use FLIP.
links:
  spec: [spec/00-conformance.md]
  adr: [adrs/0003-reference-renderer.md]
  rfc: []
  code: [conformance/PLAN.md, crates/nuif-render]
  experiments: []
---

# Summary

SSIM (Wang, Bovik, Sheikh, Simoncelli, IEEE Trans. Image Processing 13(4):600–612, 2004) replaces error-visibility models with a structural comparison of local luminance, contrast and correlation, computed in a sliding Gaussian window and averaged into a mean SSIM (MSSIM). MS-SSIM (Wang, Simoncelli, Bovik, Asilomar 2003) applies the contrast and structure terms at five dyadic scales with calibrated exponents. PSNR is a monotone transform of MSE and shares its limitations. These indices are cheap and widely implemented, but they carry no display or viewing-distance model, can return negative values, and were calibrated on compression distortions rather than rendering artefacts. Exact per-pixel comparison remains the appropriate policy when the renderer is deterministic by construction (single CPU reference path, pinned fonts, pinned Unicode data); tolerance policies are needed only where the implementation is permitted to vary.

## Evidence

- Identity: IEEE Transactions on Image Processing, vol. 13, no. 4, pp. 600–612, April 2004, DOI 10.1109/TIP.2003.819861 (Crossref record).
- MSE critique: Section I states that MSE "objectively quantifies the strength of the error signal" but that images with the same MSE "may have very different types of errors" (p. 600–601). Figure 2 shows "Boat" distortions all with MSE = 210 but MSSIM ranging from 0.9900 (mean shift) to 0.6949 (JPEG) (p. 603).
- Luminance term: l(x, y) = (2 μ_x μ_y + C1) / (μ_x² + μ_y² + C1), Equation 6; C1 = (K1 L)², Equation 7, with L the dynamic range (255 for 8-bit) (Section III.B).
- Contrast term: c(x, y) = (2 σ_x σ_y + C2) / (σ_x² + σ_y² + C2), Equation 9; C2 = (K2 L)² (Section III.B).
- Structure term: s(x, y) = (σ_xy + C3) / (σ_x σ_y + C3), Equation 10; the paper notes "s(x, y) can take on negative values" (Section III.B).
- Combined index: SSIM = l^α c^β s^γ (Equation 12); with α = β = γ = 1 and C3 = C2/2 the closed form is SSIM(x, y) = (2 μ_x μ_y + C1)(2 σ_xy + C2) / ((μ_x² + μ_y² + C1)(σ_x² + σ_y² + C2)), Equation 13 (Section III.B).
- Windowing: an 11 × 11 circular-symmetric Gaussian window with standard deviation 1.5 samples, normalised to unit sum, defines μ_x, σ_x, σ_xy (Equations 14–16, Section III.C). Constants K1 = 0.01, K2 = 0.03 are described as "somewhat arbitrary" with performance "fairly insensitive" to them (Section III.C, p. 607).
- Pooling: MSSIM(X, Y) = (1/M) Σ_j SSIM(x_j, y_j), Equation 17 (Section III.C).
- Scale dependence: the authors' project page recommends downsampling by F = max(1, round(N/256)) before SSIM for typical viewing distances and states that the right scale "depends on both the image resolution and the viewing distance" (ece.uwaterloo.ca/~z70wang/research/ssim/).
- MS-SSIM identity: Wang, Simoncelli, Bovik, "Multiscale structural similarity for image quality assessment", 37th Asilomar Conference on Signals, Systems and Computers, 2003, pp. 1398–1402, DOI 10.1109/ACSSC.2003.1292216 (Crossref).
- MS-SSIM form: the system iteratively low-pass filters and downsamples by 2; contrast and structure are compared at every scale j, luminance only at the coarsest scale M; SSIM = l_M^α_M Π_j c_j^β_j s_j^γ_j, Equation 7 (Section 3). Calibrated exponents for M = 5: β1 = γ1 = 0.0448, β2 = γ2 = 0.2856, β3 = γ3 = 0.3001, β4 = γ4 = 0.2363, α5 = β5 = γ5 = 0.1333 (Section 3.2); the calibration study fixed viewing distance at 32 pixels per degree (Section 3.1).
- Rendering-specific critique: the FLIP paper reports that SSIM "does not consider viewing distance and pixel size" and produces uninterpretable negative values, and that SSIM and Butteraugli "spread errors too widely, particularly near fireflies" (FLIP paper, Section 6.1).
- Exact-comparison practice: resvg counts a pixel as different when any channel differs by more than 1 and asserts zero differing pixels (`crates/resvg/tests/integration/main.rs`, `DIFF_THRESHOLD: u8 = 1`, `is_pix_diff`); Vello's sparse-strips tests use a per-component tolerance of 0 for the f32 CPU pipeline and 2 for the u8 pipeline (`sparse_strips/vello_dev_macros/src/lib.rs`, lines 12–23); WPT fuzzy matching uses `maxDifference` (per-channel) and `totalPixels` (web-platform-tests reftest documentation, "Fuzzy Matching").

## Mechanism

```text
PSNR(x, y) = 10 * log10( L^2 / MSE(x, y) ),   MSE = (1/N) * sum_i (x_i - y_i)^2

SSIM(x, y) = (2*mu_x*mu_y + C1) * (2*sigma_xy + C2)
             ---------------------------------------------      # Eq. 13
             (mu_x^2 + mu_y^2 + C1) * (sigma_x^2 + sigma_y^2 + C2)
  C1 = (0.01 * L)^2, C2 = (0.03 * L)^2, L = 255 for 8-bit
  mu, sigma, sigma_xy computed under an 11x11 Gaussian window, sigma_w = 1.5   # Eqs. 14-16
MSSIM = mean over windows                                                     # Eq. 17

MS-SSIM (M = 5):
  x_1 = x; x_{j+1} = downsample2(lowpass(x_j))
  MS-SSIM = l_M^{0.1333} * prod_{j=1..5} (c_j * s_j)^{beta_j}
  beta = [0.0448, 0.2856, 0.3001, 0.2363, 0.1333]                             # Eq. 7

Exact policy (deterministic path):
  pass iff for all pixels, all channels: |a - b| <= t, with t = 0 (bit-exact) or t = 1 (rounding slack),
  and count(different pixels) == 0
Count-and-delta policy (WPT / Gold / WebRender):
  pass iff max_channel_delta <= maxDifference and count(pixels with any delta > 0) <= totalPixels
```

Perceptual hashes (block-mean, DCT or gradient hashes) reduce an image to a short bit string and compare Hamming distance; they are designed for near-duplicate retrieval and are insensitive to exactly the localised, low-amplitude artefacts (one-pixel clipping offsets, anti-aliasing changes on thin strokes) that rendering conformance must detect. This statement is NUIF interpretation; no primary source was retrieved for it.

## NUIF relevance

**Borrow**
- Adopt the count-and-delta policy (`maxDifference`, `totalPixels`) as the intermediate tolerance tier for platform-pinned baselines, because it is auditable, has three independent industrial implementations (WPT, Gold, WebRender) and needs no perceptual model.
- Keep exact comparison (t = 0) as the normative policy for the CPU reference path, because resvg and Vello's f32 CPU pipeline demonstrate that a Rust rasteriser without system dependencies can be bit-identical across platforms.

**Adapt**
- If SSIM is reported at all, report it alongside FLIP and only after the downsampling rule tied to the fixture's PPD, because the index is scale-dependent and its calibration assumed 32 PPD.
- Map any single-channel tolerance t = 1 to an explicit rationale (rounding of premultiplied alpha, SIMD reassociation) recorded in the fixture, because resvg had to exclude a gradient fixture for a SIMD rounding difference.

**Reject**
- Do not use PSNR or MSSIM as conformance gates, because both lack a display model and can score semantically wrong images higher than perceptually acceptable ones (Figure 2 of the SSIM paper).
- Do not use perceptual hashes for conformance, because their design goal is retrieval robustness, which is the opposite of artefact sensitivity.

## Open questions

- Whether a luminance-only SSIM map is still useful as a cheap diagnostic overlay in conformance reports, given that FLIP maps exist.
- What per-channel slack, if any, the normative CPU path should allow for premultiplied-alpha round trips without weakening the exactness claim.
