# Apricity Web UI: Design Spec (v1.0)

This is the visual and interaction spec for the web shell. It is grounded in one fact about the
subject: **a color grading tool must not color the user's perception.** Colorists grade in
a neutral surround because nearby hues shift how an image is judged. Every choice below
follows from that.

## Principles

1. **Neutral surround, chroma only where it means something.** All chrome is true neutral
   grey (R = G = B; no tinted near-blacks). The only color in the interface is
   *functional*: the image, the scopes, the color-wheel pucks, and slider tracks that show
   what each slider does.
2. **Sliders show their own effect.** The temperature track is the Planckian ramp
   (≈2000 K amber → 10000 K blue), tint runs green → magenta, the HSL hue tracks show
   their band's hue range, and saturation runs grey → band color. This is the memorable
   element of the product; keep everything else quiet.
3. **Two stages, visibly separate.** Stage 1 (Correction) and Stage 2 (Grade) never share
   a panel. Stage 1 is "measured": compact, technical, with an *Auto* state per control.
   Stage 2 is "creative": roomier, with wheels and curves. A 1px divider plus the section
   title separates them, never color alone.
4. **The AI shows its work.** After every prompt, the Explainability Panel opens with the
   summary sentence, and every changed slider animates its thumb from the before value to
   the after value (once, 400 ms, skipped under `prefers-reduced-motion`). A small hollow
   ring marks AI-set controls until the user touches them.
5. **One family, many widths.** Typography is a single variable family; its width axis
   carries the hierarchy, not a second typeface.

## Tokens

### Color: dark (default)
| Token | Hex | Use |
|---|---|---|
| `--surround` | `#262626` | App background around the image (neutral, L* ≈ 15) |
| `--panel` | `#2F2F2F` | Inspector, prompt bar, sheets |
| `--raised` | `#3A3A3A` | Inputs, segmented controls, hovered rows |
| `--rule` | `#4A4A4A` | 1px dividers, slider rails |
| `--text` | `#EDEDED` | Primary text (≥ 12:1 on panel) |
| `--text-dim` | `#A3A3A3` | Secondary text, units (≥ 5.5:1 on panel) |
| `--focus` | `#FFFFFF` | 2px focus ring, 2px offset |

### Color: light
`--surround #C9C9C9`, `--panel #E4E4E4`, `--raised #F2F2F2`, `--rule #B3B3B3`,
`--text #1A1A1A`, `--text-dim #555555`, `--focus #000000`. Keep it neutral; no warm or cool cast.

### Functional color (color-blind safe, Okabe–Ito)
`--warn #E69F00` (broadcast-illegal, limits), `--info #56B4E9` (AI ring and fallback banner
icon), `--ok #009E73` (legal). Never use red/green alone: always pair with an icon and text.

### Type
- **Archivo** (Google Fonts, variable: `wght 100..900`, `wdth 62..125`). One family.
  - Wordmark: `wdth 125`, `wght 600`, size 20px, tracking −0.01em. Under it, a 2px
    Planckian strip (the same gradient as the temperature track).
  - Section titles ("Correction", "Light", "Color", "Wheels", "HSL", "Effects"):
    `wdth 100`, `wght 600`, 13px, sentence case.
  - Control labels and values: `wdth 75` (condensed), `wght 450`, 12px,
    `font-variant-numeric: tabular-nums`. Values right-aligned with their unit in `--text-dim`.
  - Body, panel prose, Explainability rationale: `wdth 100`, `wght 400`, 14px / 1.45.
  - Prompt input: `wdth 100`, `wght 400`, 16px (avoids iOS zoom).
- Scale: 12 / 13 / 14 / 16 / 20. No all-caps labels and no eyebrow text.

### Space and shape
4px base grid. Radii by role: 6px for inputs and buttons, 10px for sheets and popovers, 0 for
the image canvas, full circle for the wheel pucks. No drop shadows on panels; sheets
over the image use `--panel` at 96% opacity with a 1px `--rule` top edge.

## Layout

Desktop (≥ 1024px):
```
┌───────────────────────────────────────────────┬───────────────────┐
│ Apricity   [Quick | Grade | Review]   ↶ ↷  A/B│  Inspector        │
├───────────────────────────────────────────────┤  (mode-dependent) │
│                                               │  Correction       │
│            image canvas (surround)            │  ───────────────  │
│                                               │  Light / Color /  │
│                                               │  Wheels / HSL /   │
├───────────────────────────────────────────────┤  Curves / Effects │
│  [ Describe the look you want …     ] Apply   │                   │
├───────────────────────────────────────────────┤  Scopes (Grade)   │
│  Explainability (slides up after a prompt)    │                   │
└───────────────────────────────────────────────┴───────────────────┘
```
Inspector is 360px, scrollable, and left-aligned throughout. Mode switch is a segmented control in
the top bar.

Mobile (< 1024px), per PRD §8.1: the image fills the top ~60%, the prompt bar is pinned above a
**bottom tab bar** (Quick / Grade / Review), and panels are bottom sheets with a drag handle.
Single column. All primary controls are in the bottom 40%.

### Modes (PRD §8.2)
- **Quick:** prompt bar, recipe strip (thumbnails rendered on the current image),
  histogram only, "Surprise me".
- **Grade:** the full inspector (Correction, then Grade sections), plus scopes: waveform,
  parade, vectorscope, histogram, as tabs above a 256px-tall scope area. NGA stays available as
  the prompt bar.
- **Review:** the image takes the full viewport; wipe tool on; the compare source is a
  segmented control (Original / Stage 1 only / Previous grade / Reference); orientation toggle;
  overlays (none / false color / zebra) with a legend. Optional scope overlay on the canvas.

Switching modes is instant and preserves all state.

## Components

- **Slider row:** label · track · value. Click the value to type a number; double-click
  the track, or long-press it on touch, to reset (with haptic `navigator.vibrate(10)` where
  available). Arrow keys step, Shift steps ×10. `aria-valuetext` reads value and unit
  ("+0.30 stops").
- **Color wheel:** a 120px disc with a hue/saturation gradient at 35% opacity (so it doesn't
  dominate the image) and a puck; a master slider sits below. Arrow keys nudge the puck;
  double-click resets.
- **Curve editor:** 240×240 on a neutral grid. Channel tabs: Master, R, G, B, plus HSL
  hue/sat/lum as an eight-knot periodic curve. Click to add a point, drag to move it, and
  double-click a point to remove it.
- **Prompt bar:** placeholder "Describe the look you want, like cold, desaturated noir".
  Buttons: "Apply" (Fresh/Auto) and "Refine" (keeps the grade, adds the change). Swipe up or
  press ↑ to open the history (last 20). Words that weren't understood appear under the bar as
  "Didn't recognize: …".
- **Explainability Panel:** the summary sentence ("The AI made 6 adjustments.") first;
  then a row per change: label, before → after (tabular), rationale, and a chevron button
  "Show in console" that switches to Grade, scrolls to the control, and flashes its row
  once.
- **Stage 1 panel:** an "Auto" toggle per group (exposure, white balance, black point),
  showing the detected light ("Detected: tungsten, ≈3200 K"). Input profile select
  (sRGB, ARRI LogC3, Sony S-Log3, Panasonic V-Log, Apple Log).
- **Empty state:** the canvas shows one sentence and one action: "Open a photo or video to start
  grading." plus an "Open file" button; drop anywhere. A sample image link: "Try a sample image".
- **Fallback banner:** shown when WebGPU is missing, e.g. "Your browser doesn't support WebGPU,
  so the preview runs at reduced resolution. Grading, scopes and export still work."

## Copy rules
Sentence case. Buttons say exactly what happens: "Apply", "Refine", "Save recipe",
"Export JPEG", "Export LUT (.cube)". Errors state what happened and the fix ("This file
isn't a supported image. Use JPEG, PNG, WebP or HEIC."). No apologies, no exclamation marks.
