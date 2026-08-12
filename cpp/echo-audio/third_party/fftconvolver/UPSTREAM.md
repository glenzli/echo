# Upstream record

- Repository: <https://github.com/HiFi-LoFi/FFTConvolver>
- Revision: `f2cdeb04c42141d2caec19ca4f137398b2a76b85`
- Retrieved: 2026-08-13
- Included: `FFTConvolver.{h,cpp}`, `Utilities.{h,cpp}`, `COPYING.txt`
- Excluded: AudioFFT, TwoStageFFTConvolver, examples, tests, and README

AudioFFT is deliberately excluded because its fallback contains Ooura-derived code under a custom
permissive license. Echo uses the standard-MIT Signalsmith DSP adapter documented in `PATCHES.md`.
