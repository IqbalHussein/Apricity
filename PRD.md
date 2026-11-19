# Apricity — Product Requirements Document

**Version:** 1.0  
**Date:** May 4, 2026  
**Status:** v1.0 web shell implemented (scope per ADR-0005); pending deploy to GitHub Pages (ADR-0008). Beta: all features unlocked (ADR-0009).

---

## Table of Contents

1. [Executive Summary](#1-executive-summary)
2. [Problem Statement](#2-problem-statement)
3. [Goals & Non-Goals](#3-goals--non-goals)
4. [Target Audience & User Personas](#4-target-audience--user-personas)
5. [User Stories](#5-user-stories)
6. [Feature Specifications](#6-feature-specifications)
7. [Technical Architecture](#7-technical-architecture)
8. [UI/UX Specifications](#8-uiux-specifications)
9. [Monetization](#9-monetization)
10. [Success Metrics](#10-success-metrics)
11. [Competitive Landscape](#11-competitive-landscape)
12. [Risks & Mitigations](#12-risks--mitigations)
13. [Phased Roadmap](#13-phased-roadmap)
14. [Open Questions](#14-open-questions)

---

## 1. Executive Summary

Apricity is a professional-grade, AI-assisted color grading application built on a portable Rust core, launching to the web first and shipping to Android, Windows, iOS, and macOS as native shells in subsequent phases. The Rust core encapsulates the color pipeline, the on-device AI, and all parameter schemas; per-platform shells provide the native UX. It bridges the gap between the technical precision of high-end color science tools and the intuitive, language-driven workflows that modern creators expect.

The core differentiator is the **Neural Grading Agent** — an on-device AI that translates natural language prompts ("moody, teal-and-orange cinematic") into exact color science parameters, while simultaneously providing full transparency about every adjustment it makes. Apricity does not replace the colorist; it empowers creators at every skill level to work at the speed of intention.

---

## 2. Problem Statement

### The Gap in the Market

Professional color grading tools (DaVinci Resolve, Final Cut Pro Color) deliver precision but require years of training to use effectively. Consumer tools (VSCO, Lightroom Mobile presets) are approachable but shallow — they apply fixed looks with no understanding of shot context, and offer no path toward professional-quality work.

### Pain Points

| Persona | Pain Point |
|---|---|
| Mobile photographer | Spends 30+ minutes manually adjusting sliders to match a reference "look" across a batch of shots. |
| Content creator | Knows what emotion they want ("warm, dreamy, golden hour") but doesn't know which parameters to touch. |
| Aspiring colorist | Learns color theory in isolation from a real-time feedback loop that explains *why* a parameter matters. |
| Hybrid creator | Switches between multiple apps (one for scopes, one for LUTs, one for AI) losing time and quality with each export. |

### The Root Cause

Color grading has a **translation problem**: the vocabulary of human emotion and aesthetic intent is entirely disconnected from the vocabulary of color science (lift/gamma/gain, HSL curves, color matrices). No current mobile tool solves this translation problem while preserving technical depth.

---

## 3. Goals & Non-Goals

### Goals

- **G1 — Democratize precision:** Enable any creator to produce technically correct, professionally graded work without formal color science training.
- **G2 — Preserve creative control:** Every AI decision must be auditable, editable, and overridable. AI is a co-pilot, never an autopilot.
- **G3 — Maintain professional standards:** All rendering, scopes, and export formats must meet broadcast and cinema industry standards.
- **G4 — Mobile-first performance:** Zero-latency, GPU-accelerated processing on consumer iPhone hardware.
- **G5 — Closed AI loop (privacy):** All AI inference runs fully on-device. No user media is ever sent to a server.

### Non-Goals

- Apricity is **not** a video editor. It does not cut, trim, add effects, titles, or transitions.
- Apricity is **not** a social media platform. It has no feed, discovery, or social graph.
- Apricity will **not** support real-time video capture or in-camera grading in v1.
- Apricity will **not** offer cloud sync or collaborative grading sessions in v1.
- Apricity is **not** targeting Hollywood-scale productions requiring server-rendered HDR pipelines (this is a long-term stretch goal).

---

## 4. Target Audience & User Personas

### Persona 1 — "The Instagrammer" (Sofia, 24)
- **Role:** Lifestyle content creator, 180K followers.
- **Device:** iPhone 16 Pro.
- **Workflow:** Shoots RAW on iPhone, exports to Instagram and TikTok.
- **Goal:** Consistent, recognizable aesthetic across every post with minimal time per image.
- **Frustration:** Lightroom presets look great on one shot and terrible on another with different lighting. Has to manually fix every single one.
- **Willingness to Pay:** $8–12/month if it saves her 2+ hours per week.

### Persona 2 — "The Wedding Videographer" (Marcus, 31)
- **Role:** Freelance wedding and event videographer.
- **Device:** iPhone 15 Pro + GoPro Hero 13.
- **Workflow:** Shoots LOG footage, needs to deliver 100–300 color-matched clips per project.
- **Goal:** Batch color matching across multiple cameras and lighting conditions, fast enough to turn around a same-week highlight reel.
- **Frustration:** DaVinci Resolve is too complex for mobile use between shoots. No mobile tool handles LOG footage with real scopes.
- **Willingness to Pay:** $20–30/month; would pay per-project if pricing is clear.

### Persona 3 — "The Film Student" (Priya, 20)
- **Role:** Undergraduate film student.
- **Device:** iPhone 14, student budget.
- **Workflow:** Shoots short films on iPhone with Filmic Pro, grades in DaVinci Resolve on a laptop.
- **Goal:** Understand color science by doing, not just reading about it. Wants to know *why* an adjustment creates a certain look.
- **Frustration:** DaVinci Resolve teaches nothing; it just lets you do things. There's no feedback loop.
- **Willingness to Pay:** Primarily a free tier user; might pay $5/month if there's a clear student educational benefit.

### Persona 4 — "The Hybrid Creator" (James, 38)
- **Role:** Senior marketing manager who creates his own brand content.
- **Workflow:** iPhone shooter who wants semi-professional results without a learning curve.
- **Goal:** A single app that handles everything from RAW import to export-ready file. No juggling five apps.
- **Frustration:** Every app does one thing well. He uses separate apps for HEIC→RAW conversion, color grading, scopes, and export compression.
- **Willingness to Pay:** $15/month for all-in-one convenience.

---

## 5. User Stories

### Core Grading Flow

- **US-01:** As a creator, I want to describe a look in plain English so that the AI applies a matching grade without me knowing color theory terms.
- **US-02:** As a creator, I want to see exactly which parameters the AI changed (e.g., "Lifted shadows by +12, added +8 green in highlights") so that I can understand and learn from its decisions.
- **US-03:** As a colorist, I want to manually override any AI-set parameter without losing the rest of the grade so that I retain full creative control.
- **US-04:** As a videographer, I want the app to automatically normalize exposure and white balance before applying a creative grade so that the look is consistent across shots.

### Batch & Consistency

- **US-05:** As a wedding videographer, I want to apply a single grade to a batch of clips and have the AI intelligently match each clip to the grade's intent so that I don't need to adjust each one individually.
- **US-06:** As a content creator, I want to save a grade as a named "Recipe" so that I can reapply my signature look to future projects in one tap.

### Scopes & Monitoring

- **US-07:** As an aspiring colorist, I want to see a live waveform monitor while adjusting sliders so that I can learn the visual relationship between luminance and the waveform shape.
- **US-08:** As a professional, I want a vectorscope that shows me the exact color gamut of a shot so that I can ensure my grade stays within broadcast legal limits.

### LUT & Interoperability

- **US-09:** As a videographer, I want to export any grade as a 3D LUT (.cube format) so that I can apply the same look inside DaVinci Resolve or Premiere on my desktop.
- **US-10:** As a creator, I want to import existing 3D LUTs into Apricity so that I can use LUTs I've purchased or created elsewhere.

### Comparison & Feedback

- **US-11:** As a creator, I want a wipe comparison tool so that I can visually compare the graded and original versions side by side.
- **US-12:** As a creator, I want to A/B between two different AI-generated grades so that I can pick the better look before committing.

---

## 6. Feature Specifications

### 6.1 Two-Stage Grading Workflow

#### Stage 1 — Technical Correction Layer

The purpose of Stage 1 is to establish a neutral, calibrated baseline. It runs automatically on import and its results are shown to the user before the creative grade is applied.

| Sub-feature | Description | Priority |
|---|---|---|
| Auto-Exposure Normalization | Analyzes luminance histogram; adjusts exposure to bring median brightness to 45% (adjustable target). | P0 |
| White Balance Correction | Identifies the scene's dominant light source (daylight, tungsten, shade) using color temperature detection; neutralizes the white point. | P0 |
| Black Point Calibration | Ensures blacks are truly neutral (0,0,0 in a calibrated color space) without crushing shadow detail. | P0 |
| Log-to-Display Conversion | Applies the correct display LUT for LOG profiles (LOG-C, S-Log2, S-Log3, Apple ProRes Log, etc.) to bring footage to a Rec.709 viewing standard. | P0 |
| Highlight Recovery | Uses multi-exposure data (if available in RAW) to recover clipped highlights before the grade is applied. | P1 |
| Skin Tone Protection | Detects and flags skin tones in Stage 1 so that Stage 2 AI grades do not inadvertently shift them. | P1 |

**Stage 1 is non-destructive.** All adjustments are stored as metadata applied at render time.

#### Stage 2 — Artistic Grade Layer

Stage 2 is where the Neural Grading Agent operates.

| Sub-feature | Description | Priority |
|---|---|---|
| Natural Language Grade Input | Free-text prompt field where the user types or speaks a creative intent. | P0 |
| AI Parameter Translation | Agent converts prompt to specific adjustments across: color temperature, tint, lift/gamma/gain, saturation, hue curves, split toning, and grain. | P0 |
| AI Explainability Panel | Inline list showing every parameter the AI changed with before/after values and a one-sentence rationale for each. | P0 |
| Shot Coherence Engine | When multiple clips/images are in a project, the AI analyzes the reference grade on a "hero shot" and extrapolates a cohesive look for all other shots, accounting for differences in exposure, lighting color, and camera source. | P1 |
| Grade Iteration | User can type a follow-up prompt ("make it cooler, less green") and the AI applies a delta on top of the existing grade. | P0 |
| Grade Randomization | A "Surprise Me" button that generates a grade within the mood/tone inferred from the existing prompt. | P2 |

### 6.2 Neural Grading Agent

The NGA is the central product differentiator. It is a small, fine-tuned on-device language model (targeting Apple Neural Engine via Core ML) that maps natural language descriptions to color grading parameter sets.

**Interaction Model:**
1. User types a prompt in the NGA input field (e.g., *"Desaturated, cold, noir — like Se7en"*).
2. Agent parses intent and generates a structured parameter set.
3. Agent applies the grade in real time; preview updates live.
4. Explainability Panel shows the full translation (e.g., "Desaturated overall saturation to -35; shifted shadows toward blue (-12 temp); applied heavy vignette").
5. User can accept, refine with a follow-up prompt, or manually adjust any individual parameter.

**Prompt Types Supported:**
- Mood/emotional: *"hopeful, warm, sunrise"*
- Film reference: *"Kodak Portra 400 film stock"*
- Director/cinematographer reference: *"Roger Deakins — Blade Runner 2049"*
- Technical specification: *"Teal and orange LUT, low contrast shadows, lifted blacks"*
- Iterative delta: *"A bit warmer, more green in the mids"*

### 6.3 Advanced Manual Console

A full-featured manual grading console available in parallel to the NGA, designed for users who want fine-grained control.

| Control | Description | Priority |
|---|---|---|
| Lift / Gamma / Gain Wheels | Three-way color wheels for shadow, midtone, and highlight color adjustment. Industry-standard control. | P0 |
| HSL Curves | Per-channel (H, S, L) curve editor for targeted hue, saturation, and luminance adjustments. | P0 |
| Primary Sliders | Individual sliders for Exposure, Contrast, Highlights, Shadows, Whites, Blacks, Temperature, Tint, Vibrance, Saturation. | P0 |
| Tone Curve | Master RGB curve + individual R, G, B channel curves. | P0 |
| Split Toning | Independent color controls for highlights and shadows (hue + saturation). | P1 |
| Grain / Texture | Adjustable film grain with size, roughness, and intensity controls. | P2 |
| Vignette | Circular/oval vignette with center, size, softness, and midpoint controls. | P1 |
| Lens Corrections | Distortion, chromatic aberration, and vignette removal based on detected device/lens profile. | P1 |

### 6.4 Scopes & Monitoring

Professional-grade real-time monitoring tools that update live as adjustments are made.

| Scope | Description | Priority |
|---|---|---|
| Waveform Monitor | Luminance waveform (parade style: R, G, B separated). Shows distribution of brightness values across the horizontal axis of the image. | P0 |
| Vectorscope | Polar plot showing the color distribution of the image. Includes skin-tone line and broadcast-legal color gamut markers (Rec.709, P3). | P0 |
| RGB Parade | Separate R, G, B waveforms for diagnosing color channel imbalance. | P0 |
| Histogram | Luminance + RGB histogram overlaid with zone system guides. | P1 |
| False Color Overlay | Applies a color map overlay to the image canvas showing zones (overexposed, skin, midtone, underexposed) without switching away from the image. | P1 |
| Zebra Pattern | Highlights over-exposed regions above a user-set IRE threshold (default: 100 IRE). | P2 |
| Broadcast Legal Check | Automated flag when luminance exceeds 100 IRE or chrominance exceeds broadcast-legal limits. | P1 |

### 6.5 Preset & LUT Management

| Feature | Description | Priority |
|---|---|---|
| Save as Recipe | Saves the full two-stage grade (correction + creative) as a named, reusable preset. Recipes are stored locally. | P0 |
| Recipe Library | Browsable grid of saved recipes with thumbnail previews rendered on the current image. | P0 |
| Export as 3D LUT (.cube) | Exports the Stage 2 creative grade (or the full pipeline) as a 33-point 3D LUT in standard .cube format. | P1 |
| Import LUT | Imports a .cube or .3dl LUT file and applies it within the Stage 2 layer, overriding the AI grade. | P1 |
| AI Recipe Sharing | Users can share Recipes with other Apricity users via a deep link (the link encodes the parameter set, not any media). | P2 |

### 6.6 Comparison & A/B Tools

| Feature | Description | Priority |
|---|---|---|
| Wipe Comparison | Draggable split-screen wipe (vertical or horizontal) comparing graded vs. original or graded vs. a previous version. | P0 |
| A/B Grade Toggle | Stores two independent grades on a single shot. User can toggle between them with one tap to compare looks. | P1 |
| History Timeline | Scrubable timeline of grading history for a shot, allowing non-destructive reversion to any previous state. | P1 |

---

## 7. Technical Architecture

### 7.1 Architecture & Languages

Apricity is a **portable Rust core wrapped by per-platform UI shells**. The core is built once; every shell links against the same engine.

- **Core language:** Rust. Pure-Rust crates for color math, parameter schemas, recipes, LUTs, scope computation, and the NGA-output Explainability differ.
- **GPU code:** WGSL shaders running through `wgpu`. A single WGSL source compiles to WebGPU (browser), Vulkan (Android, Linux), DirectX 12 (Windows), and Metal (iOS, macOS).
- **AI runtime:** ONNX Runtime via the `ort` Rust crate. Per-platform execution providers select the best available accelerator: WebGPU EP (browser), NNAPI EP (Android), DirectML EP (Windows), Core ML EP / ANE (iOS, macOS).
- **FFI:** `wasm-bindgen` for the web (Rust → WASM), `uniffi-rs` for Kotlin and Swift bindings.
- **Shell stacks:**
  - **Web (v1.0 — primary launch):** TypeScript + Solid.js + Vite + WebGPU + WASM core
  - **Android (v1.5):** Kotlin + Jetpack Compose + UniFFI bindings
  - **Windows desktop (v1.5):** Tauri 2 wrapping the web UI; Rust core linked directly
  - **iOS / macOS (v2.0, gated on Mac access):** Swift + SwiftUI + UniFFI bindings (iOS); Tauri 2 or SwiftUI for macOS (TBD)
- **Per-platform device floors:**
  - **Web:** any browser with WebGPU (Chrome 113+, Safari 17.4+, Firefox 121+ — all shipped 2023–2024)
  - **Android (v1.5):** Snapdragon 8 Gen 1 / equivalent
  - **Windows (v1.5):** Windows 10 22H2 with DirectX 12
  - **iOS (v2.0):** A15 / iPhone 13

### 7.2 Rendering Pipeline

```
Source Image (JPEG / PNG / HEIC / WebP for web v1.0; ProRAW + LOG video on native shells from v1.5)
    ↓
[Stage 1: Technical Correction]
  Color Matrix → White Balance → Exposure Normalization → Log-to-Display LUT
    ↓
[Internal Color Space: Scene-Linear Rec.2020, 32-bit float]
    ↓
[Stage 2: Creative Grade]
  Lift/Gamma/Gain → HSL Curves → Split Tone → Tone Curve → Grain/Vignette
    ↓
[Output Transform]
  Rec.709 / P3-D65 / HDR (HLG/PQ) depending on export target
    ↓
[Rendered Preview / Export]
```

All stages are computed in `wgpu` compute shaders (WGSL). The same shader source runs on every backend (Metal / Vulkan / DX12 / WebGPU). Performance floor: < 16ms per frame (60fps preview) on every supported platform; native shells with hardware acceleration target < 8ms.

### 7.3 On-Device AI

- **Model architecture:** A bespoke narrow text encoder (~30–80M parameters) with a parameter-regression head that emits the `GradeParameters` struct directly. Not a general-purpose LLM. INT8 quantized post-training. Target: < 200MB active memory, < 500MB on-disk.
- **Model input:** Tokenized natural language prompt + optional image feature vector (scene analysis for lighting, key colors, and subject detection).
- **Model output:** Structured `GradeParameters` JSON mapping 1:1 to the Manual Console controls.
- **Inference runtime:** ONNX Runtime via the `ort` Rust crate. The same ONNX model runs on every shell via the best available execution provider per platform.
- **Latency target:** < 3s on browser via WebGPU EP (re-baselined from the original 2s — browser overhead is real); < 2s on native shells with hardware accelerator (NNAPI / DirectML / ANE).
- **Offline operation:** All inference runs locally on every platform. No network call is required for the core grading workflow.

### 7.4 File Format Support

| Format | Web (v1.0) | Android / Windows (v1.5) | iOS / macOS (v2.0) |
|---|---|---|---|
| JPEG / HEIC | ✓ | ✓ | ✓ |
| PNG | ✓ | ✓ | ✓ |
| WebP | ✓ | ✓ | ✓ |
| ProRAW (DNG) | — (browser API limit) | ✓ | ✓ |
| ProRes (LOG & 422) | — | ✓ | ✓ |
| H.264 / HEVC | ✓ via WebCodecs | ✓ | ✓ |
| 3D LUT (.cube) | ✓ (v1.0, ADR-0005) | ✓ | ✓ |
| TIFF (16-bit) | v1.5 | v1.5 | v2.0 |
| CinemaDNG | — | v2.0 | v2.0 |

### 7.5 Privacy & Security

- No user media is uploaded to any server for any reason. This is enforced by the browser sandbox on web and by simply not making network calls from the pipeline on native shells.
- The NGA model is bundled with the app (web) or downloaded once on first launch (native shells), then cached.
- Recipes shared via deep link (post-v1.0) encode only parameter values — no pixel data is transmitted.
- Core grading workflow requires no network. Network is only used for: payment processing (Stripe on web; Play Billing on Android; Microsoft Store / Stripe on Windows; StoreKit on iOS / macOS), opt-in analytics, and model updates.

---

## 8. UI/UX Specifications

### 8.1 Layout Principles

- **Thumb-Zone First:** All primary controls (NGA input, playback, key sliders) live in the bottom 40% of the screen. Scopes and preview fill the top 60%.
- **Single-Column Layout:** No side panels or multi-column layouts on iPhone. All panels slide up from the bottom sheet.
- **Progressive Disclosure:** Simple controls visible by default. Advanced controls revealed through a single "expand" gesture. Never hide primary functionality.

### 8.2 Context-Aware Workspaces

The UI mode is selected from a persistent bottom tab bar (three modes):

| Mode | Primary Focus | Secondary Focus |
|---|---|---|
| **Quick** | NGA prompt + single-tap presets | Minimal scopes (histogram only) |
| **Grade** | Full Manual Console + scopes | NGA as floating assistant |
| **Review** | Full-screen preview + wipe tool | Scope overlay on canvas |

Switching modes is instant; all state is preserved.

### 8.3 Neural Grading Agent Input

- Full-width text field pinned above the bottom tab bar.
- Supports keyboard and voice (Speech-to-Text via Apple's on-device SFSpeechRecognizer).
- Prompt history accessible via swipe-up on the input field (last 20 prompts).
- After a grade is applied, the Explainability Panel slides up with a summarized diff of changes.

### 8.4 Explainability Panel

- Shows a plain-language summary: *"The AI made 6 adjustments."*
- Expands to a list of each parameter change with a before/after value and a one-line reason.
- Each parameter change has a right-arrow that jumps directly to that slider in the Manual Console for inline editing.

### 8.5 Wipe Comparison Tool

- Activated by a two-finger horizontal swipe or a dedicated icon in Review mode.
- Divider line is draggable. The user can set it to any position.
- Left side: current grade. Right side: selectable from {Original RAW, Stage 1 Only, Previous Grade, Imported Reference}.
- Orientation can be toggled between vertical and horizontal.

### 8.6 Haptics & Gestures

| Gesture | Action |
|---|---|
| Tap slider | Selects slider, shows numeric input |
| Long press slider | Resets to default value (haptic confirmation) |
| Two-finger pinch on canvas | Zooms preview (1x–8x) |
| Swipe left on canvas | Toggles to next image/clip in project |
| Swipe up on NGA input | Opens prompt history |
| Force touch on grade thumbnail | Peeks at before/after diff |

Haptic feedback is used for: slider snapping to keyframe values, reset confirmation, and AI grade application complete.

### 8.7 Accessibility

- All controls meet WCAG AA contrast ratios in both light and dark mode.
- Full VoiceOver support for all grading controls, with spoken before/after values.
- Dynamic Type support for all text elements.
- Color-blind safe scope overlays (no red/green only indicators).

---

## 9. Monetization

### 9.1 Tier Overview

| Feature | Free | Pro ($14.99/mo or $99.99/yr) |
|---|---|---|
| Import photos/videos | Up to 50 files/month | Unlimited |
| AI grading prompts | 15 prompts/month | Unlimited |
| Manual Console | Basic sliders only (no curves, no wheels) | Full console |
| Scopes | Histogram only | Full waveform, vectorscope, RGB parade, false color |
| Export resolution | 2K max | 4K + ProRes |
| Export watermark | Yes | No |
| 3D LUT export | No | Yes |
| LUT import | No | Yes |
| Batch processing | No | Yes (up to 300 clips/project) |
| Saved Recipes | 5 | Unlimited |
| Shot Coherence Engine | No | Yes |
| A/B Grade Tool | No | Yes |
| Priority support | No | Yes |

### 9.2 Pricing Rationale

- **$14.99/mo** positions Apricity between Lightroom ($9.99/mo) and LumaFusion ($29.99 one-time). It targets professional creators who already pay for creative tools.
- **$99.99/yr** ($8.33/mo effective) drives annual conversion and reduces churn.
- The free tier is genuinely useful for casual creators (15 AI prompts/month is 2–3 shoots) but creates clear upgrade pressure for professionals.
- Pricing is identical across all platforms; entitlements are per-shell in v1.0–v1.5 and unified in v2.0+.

### 9.3 Payment Infrastructure (per shell)

- **Web (v1.0):** Stripe subscriptions with hosted checkout. Webhook-driven entitlement updates. Customer portal for plan management.
- **Android (v1.5):** Google Play Billing — required by Play Store policy for digital subscriptions.
- **Windows (v1.5):** Stripe (Microsoft Store does not require its own billing for non-Store distribution).
- **iOS / macOS (v2.0):** StoreKit 2 — required by App Store policy.
- **Cross-shell sync:** Unified subscription state across shells is a v2.0+ feature, gated on a thin server (e.g. Supabase). v1.0 and v1.5 entitlements are platform-local.

### 9.4 Future Monetization

- **Pay-Per-Project:** One-time unlock for a single project (wedding, commercial shoot). Pricing TBD.
- **Recipe Marketplace:** Creators sell/buy community-made Recipes (30% platform fee).
- **Enterprise/Education Licensing:** Volume licensing for film schools and content studios.

---

## 10. Success Metrics

### 10.1 Acquisition

| Metric | 3-Month Target | 12-Month Target |
|---|---|---|
| App Store downloads | 10,000 | 100,000 |
| Free → Pro conversion rate | 5% | 8% |
| Monthly Recurring Revenue (MRR) | $5,000 | $75,000 |

### 10.2 Engagement

| Metric | Target |
|---|---|
| D7 retention | > 35% |
| D30 retention | > 20% |
| Average sessions per week (Pro users) | > 4 |
| NGA prompt usage per session (Pro) | > 3 |
| Recipe saves per user per month | > 2 |

### 10.3 Quality

| Metric | Target |
|---|---|
| App Store rating | ≥ 4.5 stars |
| Crash-free sessions | > 99.5% |
| AI grade latency (p95) | < 3s on browser via WebGPU EP; < 2s on native shells with hardware accelerator |
| Preview render latency (p95) | < 16ms (60fps) on every supported platform |
| Export time (4K H.264, 60s clip) | < 90 seconds on web; ProRes export is v1.5 native shells only |

---

## 11. Competitive Landscape

| App | Strengths | Weaknesses vs. Apricity |
|---|---|---|
| **Lightroom Mobile** | Huge user base, RAW support, sync across devices | No video support, no scopes, no AI grading, no LUT export |
| **VSCO** | Beautiful presets, strong aesthetic community | Presets only, no manual control, no AI, no scopes |
| **DaVinci Resolve (iPad)** | Industry standard, full pro feature set | Steep learning curve, no AI, no mobile-optimized UX |
| **Halide + Darkroom** | Excellent RAW capture + editing combo | No AI, no scopes, no video, no LUT tools |
| **CapCut / VN** | Viral growth, video + color tools | Consumer-grade, no scopes, no RAW, AI is trend-based not creative |
| **Filmr / KineMaster** | Video-first mobile editing | No professional color tools, no scopes, no LUT support |

**Apricity's defensible differentiation:** The combination of (1) on-device AI with explainability, (2) professional scopes, and (3) two-stage technical+creative workflow in a mobile-first UX has no direct competitor. The closest threat is Adobe adding AI grading to Lightroom Mobile, which remains a meaningful risk.

---

## 12. Risks & Mitigations

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| On-device AI model quality is insufficient for nuanced prompts | High | Critical | Invest in fine-tuning dataset quality; allow cloud fallback (opt-in, paid tier) for complex prompts. |
| A16/A15 performance targets not met on older devices | Medium | High | Define a minimum supported device (A14 Bionic / iPhone 12). Degrade gracefully on older hardware (lower precision, slower preview). |
| Adobe adds AI grading to Lightroom Mobile | Medium | High | Accelerate differentiated features: explainability panel, shot coherence engine, LUT ecosystem. Build creator community moat. |
| User confusion between Stage 1 and Stage 2 | High | Medium | Onboarding flow explicitly walks through the two-stage concept with visual examples. Separate visual affordances for correction vs. creative controls. |
| Over-reliance on AI creates a "black box" feel | Medium | High | The Explainability Panel is P0, not P2. Never ship without it. |
| App Store rejection (AI or privacy concerns) | Low | Critical | Pre-submit consultation with Apple Developer Relations. Strict on-device-only AI policy eliminates most privacy objections. |
| LUT export quality issues (color space mismatches) | Medium | Medium | Validate .cube exports against DaVinci Resolve, Premiere, and Final Cut Pro in QA. Publish a compatibility guide. |

---

## 13. Phased Roadmap

### Phase 1 — v1.0: Web Launch (Target: Q3 2026)

The minimum viable product, shipped as a web application accessible at a public URL. Built on the Rust core that all later shells reuse — no engine work is wasted.

- **Rust core crates:** `apricity-core`, `apricity-render` (wgpu/WGSL), `apricity-scopes`, `apricity-lut`, `apricity-recipes`, `apricity-nga` (ONNX/`ort`), `apricity-io`, `apricity-ffi` (wasm-bindgen)
- Stage 1 technical correction (exposure, white balance, log-to-display)
- Stage 2 AI grade via NGA with Explainability Panel
- Basic Manual Console (primary sliders + tone curve)
- Waveform, vectorscope, RGB parade
- Wipe comparison tool
- Save/load Recipes (up to 5 on free tier; persisted in localStorage / OPFS)
- Import/export: JPEG, PNG, HEIC, WebP, H.264 (via WebCodecs)
- Monetization: Free + Pro subscription via Stripe
- Web shell stack: TypeScript + Solid.js + Vite + WebGPU

**As built.** ADR-0005 pulled these v1.5 items into v1.0 web: lift/gamma/gain wheels, 8-band HSL curves, split tone, vignette, seeded grain, `.cube` LUT import/export, histogram, A/B grade toggle, history/undo, the broadcast-legal flag, and false color/zebra overlays. ADR-0006 replaced the ONNX model with a deterministic lexicon engine behind a `GradeAgent` trait for v1.0, and ADR-0009 builds the Free/Pro gating without a checkout. Still excluded from v1.0: the Shot Coherence Engine, native shells, ProRAW/ProRes, lens corrections, and recipe deep-link sharing.

### Phase 2 — v1.5: Native Mobile + Desktop (Target: Q1 2027)

- **Android shell:** Kotlin + Jetpack Compose, UniFFI bindings to Rust core, Play Billing
- **Windows desktop shell:** Tauri 2 wrapping the v1.0 web UI; Rust core linked directly; Stripe billing
- Full Manual Console (3-way wheels, HSL curves, split toning)
- Shot Coherence Engine (batch grading)
- LUT import/export (.cube) on every shell
- A/B Grade Tool
- False color + zebra overlays
- ProRAW / ProRes import on native shells
- ProRes export on native shells

### Phase 3 — v2.0: Apple Platforms + Advanced (Target: Q3 2027, gated on Mac access)

- **iOS shell:** Swift + SwiftUI, UniFFI bindings to Rust core, StoreKit 2, Core ML EP for ANE acceleration
- **macOS shell:** Tauri 2 (reusing web UI) or native SwiftUI — decision pending
- Recipe sharing via deep link
- CinemaDNG support
- AI model update system (pull updated ONNX model OTA)
- Optional cloud prompt fallback (advanced prompts, opt-in Pro feature)
- Cross-platform subscription sync (single login, entitlements unified across shells via thin server)

### Phase 4 — v3.0: Ecosystem (Target: 2028)

- Recipe Marketplace
- Studio batch-processing API
- Enterprise / education licensing
- Collaboration / shared project links

---

## 14. Open Questions

The original seven questions are largely resolved by the cross-platform pivot. Resolutions live in `/docs/adr/`. Questions newly introduced by the pivot are listed below.

### Resolved

1. **NGA model strategy** — RESOLVED: bespoke narrow model exported as ONNX, executed via `ort` with per-platform execution providers (WebGPU / NNAPI / DirectML / Core ML). See ADR-0003.

2. **Minimum supported device** — RESOLVED per platform: web requires WebGPU (Chrome 113+ / Safari 17.4+ / Firefox 121+); Android v1.5 baseline Snapdragon 8 Gen 1; iOS v2.0 baseline A15 / iPhone 13. See ADR-0004.

3. **Shot Coherence in v1.0 vs. v1.5** — RESOLVED: pushed to v1.5. Marcus persona served by v1.0 web with single-clip workflow first; batch coherence ships with the native Android shell in v1.5.

4. **Recipe Sharing privacy** — RESOLVED: opt-in per recipe, default private. The schema flag is added in v1.0; sharing UI ships in v2.0.

5. **Cloud fallback design** — RESOLVED for v1.0–v1.5: out of scope. Reconsidered for v2.0 as opt-in Pro feature; privacy boundary communication TBD at that time.

6. **Cross-platform rendering engine** (originally framed as the "Android Vulkan vs. iOS Metal" problem) — RESOLVED: `wgpu` with single-source WGSL shaders. Same source compiles to Metal / Vulkan / DX12 / WebGPU. No per-platform pipeline rewrite required.

### Still open (validated after first beta)

7. **Onboarding flow depth.** Hypothesis: 3-step visual onboarding + guided first grade. Validate after first beta.

   *Status:* still open. The three-step hypothesis ships behind the `onboarding` flag (on by default, `?onboarding=0` turns it off) so it can be validated after the first beta.

### New, introduced by the pivot

8. **Web UI framework.** Recommended: Solid.js + Vite (lightest reactivity model, best for high-frequency slider updates). Alternative: React, if community/hire concerns dominate. Decide before M4.

   *Resolution (ADR-0002):* Solid.js + Vite + TypeScript.

9. **Web payment provider.** Recommended: Stripe (mature, lowest fees, full subscription support). Alternative: Lemonsqueezy or Paddle as Merchant-of-Record to offload tax handling. Decide before M6.

   *Status (ADR-0009):* still open. v1.0 ships an `EntitlementProvider` interface, the §9.1 tier limits and a `BETA_ALL_PRO` flag, with no checkout. A new ADR must decide this before the flag is turned off.

10. **Tauri vs. egui for desktop shell (v1.5).** Recommended: Tauri 2 (reuses the web UI codebase). Alternative: egui (fully Rust-native, but introduces a third UI codebase). Decide before v1.5 kickoff.

11. **WebGPU fallback for older browsers.** Recommended: WebGL2 fallback for preview rendering only; NGA is disabled if WebGPU is unavailable (graceful degradation, not full feature parity). Decide before M4.

   *Resolution (ADR-0007):* without WebGPU the shell renders with the `apricity-render` CPU reference path (the code the parity tests hold the GPU to) at reduced resolution. The NGA stays enabled, because the v1 lexicon engine (ADR-0006) does not need WebGPU. There is no WebGL2 path.

12. **Cross-platform subscription sync.** When and how to unify subscription state across shells. Likely v2.0 with a thin server (Supabase or similar). Out of scope for v1.0–v1.5.

13. **Web-tier file format limits.** How aggressively to push browser file APIs (File System Access API, WebCodecs) for ProRAW / ProRes work in v1.0 vs. accepting these as native-shell-only features. Current recommendation per §7.4: native-shell-only.

   *Resolution:* native-shell-only stands. v1.0 web does not read ProRAW or ProRes.
