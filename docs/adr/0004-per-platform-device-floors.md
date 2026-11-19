# ADR-0004: Per-Platform Device Floors

- Status: accepted
- Amended by: ADR-0007 (fallback is the CPU reference pipeline, NGA stays enabled)
- Date: 2026-05-05

## Context

PRD §14 Q2 asked for a single minimum-supported device. Under the
cross-platform pivot, "minimum device" is now per-shell. The floor is the
hardware required to hold the < 16ms preview budget and run NGA inference
within latency targets.

## Decision

| Shell (phase) | Floor | Rationale |
|---|---|---|
| Web (v1.0) | Any browser with **WebGPU** — Chrome 113+, Safari 17.4+, Firefox 121+ | WebGPU is the hard gate for the rendering pipeline and the WebGPU EP for `ort`. All three browsers shipped WebGPU stable in 2023–2024 and are ubiquitous by 2026. Older browsers fall back to WebGL2 for **preview only**, with NGA disabled. |
| Android (v1.5) | **Snapdragon 8 Gen 1** or equivalent (Vulkan 1.3, NNAPI 1.3) | Sustained GPU compute and a usable NPU for INT8 inference. |
| Windows (v1.5) | DX12-class GPU + DirectML-supported NPU/GPU | Tauri 2 + wgpu's DX12 backend; DirectML EP for `ort`. |
| iOS (v2.0) | **A15 (iPhone 13)** | Sustained Metal performance and ANE generation that meets the < 2s NGA target. |
| macOS (v2.0) | Apple Silicon (M1+) | Same toolchain as iOS; Intel Macs not supported. |

## Consequences

- The web build's hard gate is WebGPU presence. WebGL2 fallback is preview
  only; NGA, scopes, and any compute-shader features are disabled in that
  mode. Surfaced clearly in onboarding.
- iPhone 12 and older are **not supported** even when the iOS shell ships;
  those users use the web build.
- The Android floor excludes mid-tier 2022 and earlier devices; revisit this
  closer to v1.5 launch with then-current market data.
