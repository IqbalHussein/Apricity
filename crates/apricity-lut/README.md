# apricity-lut

Pure-Rust `.cube` parser/writer and 3D LUT sampling. The GPU kernel that
consumes these tables lives in `apricity-render`.

## Color-space contract

LUTs are tagged with their input and output color spaces. Sampling expects
inputs already in the LUT's input space; the caller is responsible for
upstream conversion.
