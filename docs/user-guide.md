# Apricity user guide

Apricity grades photos and video in your browser. You can describe a look in words, read exactly what the AI changed, and adjust any of it by hand. Nothing you open is uploaded anywhere.

## Contents

1. [Browser requirements](#browser-requirements)
2. [Open a photo or video](#open-a-photo-or-video)
3. [The two stages: Correction and Grade](#the-two-stages-correction-and-grade)
4. [Describe a look](#describe-a-look)
5. [Read the Explainability Panel](#read-the-explainability-panel)
6. [The Manual Console](#the-manual-console)
7. [Scopes](#scopes)
8. [Review mode](#review-mode)
9. [A/B grades and undo](#ab-grades-and-undo)
10. [Recipes](#recipes)
11. [LUTs](#luts)
12. [Export images and video](#export-images-and-video)
13. [Video](#video)
14. [Offline use](#offline-use)
15. [Free and Pro](#free-and-pro)
16. [Keyboard shortcuts](#keyboard-shortcuts)
17. [Privacy](#privacy)

## Browser requirements

Apricity works best in a browser with WebGPU: Chrome 113 or newer, Safari 17.4 or newer, or Firefox 121 or newer (WebGPU availability in Firefox varies by operating system). With WebGPU the preview runs on your graphics card.

Without WebGPU, Apricity falls back to rendering on the CPU. A banner says: "Your browser doesn't support WebGPU, so the preview runs at reduced resolution. Grading, scopes and export still work." The colors are identical to the GPU path, and the AI agent stays on. The preview is smaller and slower, and video export needs WebGPU. You can dismiss the banner.

Video export also needs WebCodecs, which current Chrome and Edge have.

## Open a photo or video

Choose **Open file**, or drop a file anywhere on the page. Photos can be JPEG, PNG, WebP or HEIC (HEIC only where your browser can decode it). Video can be MP4 or MOV. If a file can't be opened, the message says which formats work.

**Try a sample image** opens a built-in image if you only want to look around.

For speed, Apricity grades a preview copy of your photo (up to 2048 px on the long edge). Export always re-reads your original file at full resolution, so the preview size does not limit your export.

## The two stages: Correction and Grade

Apricity separates fixing an image from styling it.

**Correction (Stage 1)** is technical. It sets exposure, white balance and black point so the image starts from a sound base. It also has an **Input profile** for footage shot in a log format: sRGB, ARRI LogC3, Sony S-Log3, Panasonic V-Log or Apple Log. With a log profile chosen, a **Log to display** switch converts the flat log picture to a normal-looking one.

Each of exposure, white balance and black point has an **Auto** switch. With Auto on, Apricity measures the image and sets the value, and tells you what it found, for example "Detected: tungsten, ≈3200 K". Turn Auto off to set the value yourself. Once the image is analyzed, the Correction panel folds into a one-line summary, such as "auto · daylight ≈6500 K · +0.20 EV". Click it to open the controls.

**Grade (Stage 2)** is creative. Contrast, color, wheels, curves and effects sit on top of the corrected image. The AI agent only changes Grade, never Correction, so your base stays put when you try looks.

## Describe a look

Type into the prompt bar, such as "cold, desaturated noir", and press Enter or choose **Apply**.

**What it understands.** The agent has a built-in vocabulary of:

- moods: moody, dreamy, gritty, soft
- palettes and temperatures: warm, cold, teal and orange, golden hour
- technical terms: matte, high contrast, desaturated, faded
- film stocks: Portra, Kodak and Fuji stocks, Tri-X, CineStill
- genres, eras and reference looks: noir, vintage, 80s, Blade Runner
- scenes: forest, snow

Film and film-maker names describe a look. They don't claim to match the real thing.

**Example prompts.**

- `golden hour, warm and soft`
- `faded film, lifted blacks, muted`
- `portra, warm skin tones, soft pastel`
- `teal-and-orange cinematic`
- `moody forest, no vignette`

**Strength words.** `slightly`, `a bit` and `subtle` halve a term. `very`, `really` and `strong` raise it. `less`, `not` and `too` reverse it. `no`, `without` and `remove` reset that term's controls.

**Apply and Refine.**

- **Apply** starts from a clean Grade when your prompt describes a whole new look. If your prompt sounds like an adjustment ("a bit warmer, less green") and you already have a grade, Apply refines instead.
- **Refine** always keeps your current grade and adds the change on top. Use it for "more contrast", "a bit warmer, less green", or "slightly darker".

Stage 1 is never touched by either button.

**Surprise me** picks a random look and applies it. Each press gives a different result, and it is one undo step like any other change.

**Prompt history.** Press the up arrow in an empty prompt box, or choose the history button, to see your last 20 prompts and re-use one.

**"Didn't recognize".** If part of your prompt isn't in the vocabulary, a line under the bar says "Didn't recognize:" and lists those words. The words it did understand are still applied. If nothing matches, the grade doesn't change and the Explainability Panel says "Nothing changed. Try describing the mood, the light or the color." Try a simpler or more common word.

The agent runs entirely in your browser. There is no microphone input, because browser speech recognition sends audio to a server.

## Read the Explainability Panel

After each prompt, a panel opens under the prompt bar. The first line is a summary, such as "The AI made 6 adjustments." Under it is one row per change:

- the control's name,
- the value before and after, with units,
- the reason, written in plain words and quoting your prompt: "Warmed the color temperature (+12 mired) for 'golden hour'."

Choose the arrow at the end of a row (**Show in console**) to jump to that control in the Grade panel. The control scrolls into view and flashes once. Controls the AI set are marked with a small ring until you touch them. You can collapse the panel by clicking its summary line.

Because the AI only moves ordinary controls, anything it did you can undo, reset or tune by hand.

## The Manual Console

The Grade panel is split into sections you can open and close:

- **Light**: exposure, contrast, highlights, shadows, whites, blacks.
- **Color**: temperature, tint, vibrance, saturation.
- **Wheels**: lift (shadows), gamma (midtones), gain (highlights). Drag the puck to shift hue and saturation, use the arrow keys to nudge it, and double-click to reset.
- **HSL**: eight color bands, each with hue, saturation and luminance.
- **Curves**: master, red, green and blue tone curves. Click to add a point, drag to move it, double-click a point to remove it.
- **Split tone**: separate colors for shadows and highlights.
- **Effects**: vignette and film grain. Grain is seeded, so the same settings always give the same grain.
- **LUT**: import a `.cube` file and set its strength (see [LUTs](#luts)).

Every slider has its unit next to the value. Click the value to type a number. Double-click the track, or press and hold on a touch screen, to reset it. The arrow keys step the slider, and Shift with an arrow steps ten times as far. The slider track shows what it does: the temperature track runs amber to blue, tint runs green to magenta, and the HSL tracks show their color.

Free includes the Light and Color sections. Other sections need Pro (all unlocked during the beta).

## Scopes

Scopes measure the picture rather than how it looks on your screen. Open them from the **Scopes** section in the Grade panel (on a phone, switch the panel to **Scopes**).

- **Waveform**: brightness across the width of the image. Use it to check where shadows and highlights sit.
- **Parade**: separate red, green and blue waveforms, for spotting a color cast.
- **Vectorscope**: color saturation and hue around a circle, with targets for red, magenta, blue, cyan, green and yellow, and a line showing where skin tones usually fall.
- **Histogram**: how many pixels sit at each brightness, for red, green, blue and luma.

A badge below the scope reads **Broadcast legal** or **Not broadcast legal**, followed by what is wrong, for example "1.2% over 100 IRE" or "0.5% out of gamut". Over 0.1% of pixels outside the legal range makes it not legal.

Free includes the histogram. Quick mode shows only the histogram.

## Review mode

Choose **Review** at the top (or the bottom tab bar on a phone) to give the image the whole screen. A wipe divider splits it: drag it to compare, or use the arrow keys when it has focus (Shift for bigger steps, Home and End for the edges).

**Compare with** sets what the other side shows:

- **Original**: the untouched image.
- **Stage 1 only**: the corrected image, before your Grade.
- **Previous grade**: the grade from before your last change (available after you make a change).
- **Reference**: an image you choose, to match a look.

**Divider** switches the wipe between vertical and horizontal.

**Overlay** adds a diagnostic view on the image:

- **False color** maps brightness zones to colors, with a legend: purple for crushed blacks, blue for near black, green for middle grey, pink for skin tones, yellow for near clipping, red for clipped.
- **Zebra** draws stripes over every pixel brighter than a threshold. Set the threshold from 50% to 100%.

Overlays are for preview only. They never appear in an export.

Zoom with the scroll wheel, a pinch, or the `+` and `-` keys. Drag to pan when zoomed. Press `0` or double-click the image to fit it again.

## A/B grades and undo

**A/B** keeps two grades. The A and B buttons in the top bar show which is live. Press the toggle (or `\`) to switch. **A → B** copies the live A grade into B so you can try a variation. A/B is a Pro feature.

**Undo and redo** step through your changes, up to 200 steps. Quick slider drags merge into one step. Each step is named after what you changed, such as "Contrast" or the prompt you typed.

## Recipes

A recipe is a saved grade, Correction and Grade together, that you can apply to any image.

- **Save recipe** (bookmark button in the top bar) names and saves the current grade. The suggested name comes from your last prompt.
- **Library** (grid button) lists your recipes and six starter looks: Golden hour, Teal & orange, Faded film, Noir, Portra, and Cool morning. Starters are built from prompts, are not stored, and don't count toward your limit.
- **Quick mode** shows a strip of recipes you can apply with one click.
- Applying a recipe is one undo step. If a recipe has Auto on for exposure, white balance or black point, your image keeps its own measured values for those and only takes the creative part.
- From the library you can rename, delete, or export a recipe as a `.json` file, and import a recipe `.json`. Exported files are your backup, because recipes are stored only in your browser.

## LUTs

A LUT (look-up table) is a file that maps colors to colors, used to move a look between apps.

**Import.** In the Grade panel's **LUT** section, choose **Import LUT (.cube)**. Only `.cube` 3D LUTs work. Set the **strength** slider to blend the LUT with the grade. The LUT is applied inside Grade, after your wheels and curves. **Remove LUT** takes it off.

**Export.** In **Export**, open the **LUT (.cube)** tab and choose a size (17, 33 or 65 points) and what it contains:

- **Creative (Stage 2 only)** bakes the Grade alone. Use it on footage you correct elsewhere.
- **Full pipeline** also bakes Correction, including the log-to-display conversion if you chose a log input profile.

Vignette and grain depend on where a pixel is in the frame, so they can't be stored in a LUT and are left out.

**Color convention.** Every LUT in Apricity, imported or exported, takes sRGB-encoded Rec.709 values in the 0 to 1 range and returns the same. This is the common convention for creative LUTs in DaVinci Resolve, Premiere and Final Cut. A LUT built for a different input, such as a camera-log-to-Rec.709 conversion LUT, won't look right on an ordinary image.

LUT import and export are Pro features.

## Export images and video

Choose **Export**. For a photo, the **Image** tab offers JPEG, PNG and WebP (WebP only where your browser can encode it) and a quality slider for JPEG and WebP. The dialog shows the output size before you export. Export renders from your original file at full resolution.

Free exports are limited to 2048 px on the long edge and carry a small "Graded with Apricity" mark in the bottom-right corner. Pro exports at full resolution with no mark.

## Video

You can open an MP4 or MOV file, grade it with the same controls, and export it.

- **Playback.** A transport bar appears under the preview with play and pause, previous and next frame, the current time, a scrubber, the duration, loop, and mute. The preview starts muted. The mute button is disabled for clips with no audio. Press Space to play or pause, and `,` and `.` to step one frame back or forward (see [Keyboard shortcuts](#keyboard-shortcuts)).
- **Grading.** Corrections, the AI agent, scopes and Review tools work on the frame you are viewing. Scopes and Auto analysis sample the video about five times a second while it plays.
- **Export.** The **Video** tab in the Export dialog renders every frame through your grade and saves an MP4. It uses H.264 where your browser can encode it and VP9 otherwise, and the result message names the codec used. AAC audio is copied unchanged. If the source audio isn't AAC, it's left out, and the result message says so. A progress bar shows the frame count, elapsed time and time left, and **Cancel** stops it without saving anything. Free export is capped at 2048 px on the long edge and carries the mark.
- **Requirements.** The CPU renderer can preview video but can't export it: video export needs WebGPU and WebCodecs. If your browser can't play a file, the message asks for an H.264 MP4 or MOV.

## Offline use

After your first visit, Apricity saves itself in your browser, and you can grade without a connection. You can also install it from your browser's menu as an app. If you reopen it offline, it loads from that saved copy. A new version is picked up the next time you are online.

## Free and Pro

| | Free | Pro |
|---|---|---|
| Photo imports | 50 a month | Unlimited |
| AI prompts | 15 a month | Unlimited |
| Console sections | Light and Color | All |
| Scopes | Histogram | Waveform, parade, vectorscope, histogram |
| Saved recipes | 5 (starters don't count) | Unlimited |
| Export | 2048 px long edge, with mark | Full resolution, no mark |
| LUT import and export | No | Yes |
| A/B grades | No | Yes |

**During the beta, every feature is unlocked for everyone.** There is no checkout yet, so nobody is asked to pay. Counters for the monthly limits are kept in your browser. Clearing your site data resets them, and also deletes your saved recipes.

## Keyboard shortcuts

| Keys | What it does |
|---|---|
| Ctrl+Z or Cmd+Z | Undo |
| Shift+Ctrl+Z or Shift+Cmd+Z | Redo |
| `\` | Switch between A and B grades |
| Enter (in the prompt box) | Apply the prompt |
| Up arrow (in an empty prompt box) | Open prompt history |
| Esc (in the prompt box) | Close prompt history |
| `+` / `-` / `0` (image focused) | Zoom in, zoom out, fit |
| Arrow keys (image focused, zoomed in) | Pan; Shift pans farther |
| Arrow keys (wipe divider focused) | Move the divider; Shift for bigger steps; Home and End go to the edges |
| Arrow keys (slider focused) | Step by one; Shift steps ten; Page Up and Page Down step ten; Home and End go to the limits; Delete resets |
| Space (video) | Play or pause |
| `,` and `.` (video) | Previous and next frame |

The undo and A/B shortcuts are ignored while you are typing in a text box or when a dialog is open. The video keys are ignored in the same cases.

## Privacy

All processing happens on your device. Apricity has no server that receives your photos, video, prompts or grades. The AI agent is a set of rules that runs in your browser, so a prompt never leaves the page. After the app loads, the only network use is fetching the app and its fonts, and checking for a new version.

Recipes, history, imported LUTs, settings and usage counters are stored in your browser. They're lost if you clear site data, so export recipes you want to keep.
