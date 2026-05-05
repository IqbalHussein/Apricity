# apricity-scopes

Waveform, vectorscope, RGB parade, and histogram. Pure-Rust reference impls
plus WGSL compute kernels for production rendering.

## Color-space contract

Scopes consume scene-linear Rec.2020 frames (the pipeline interior). Display
mappings (e.g. log-encoded waveform, Rec.709 vectorscope) are computed inside
the scope kernel and tagged on the output.

## Test invariants

- Each scope's CPU and GPU outputs match within tolerance on synthetic inputs.
