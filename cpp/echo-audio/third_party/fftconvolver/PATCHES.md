# Local patches

Echo keeps the upstream formatting and applies only these integration patches:

1. Replace the `AudioFFT.h` include with Echo's Signalsmith AudioFFT adapter.
2. Trim only exact positive or negative zero IR tail samples; upstream used an implicit `1e-6`
   threshold that changed low-level authored tails.
3. Add `resetState()` to clear streaming history without destroying the immutable prepared IR or
   allocating on the realtime thread.

Echo does not force an SSE build flag; upstream architecture detection remains active on x86.
