use super::is_third_party_path;

#[test]
fn vendored_native_sources_are_outside_echo_format_ownership() {
    assert!(is_third_party_path(std::path::Path::new(
        "cpp/echo-audio/third_party/fftconvolver/FFTConvolver.cpp"
    )));
    assert!(!is_third_party_path(std::path::Path::new(
        "cpp/echo-audio/src/space_processor.cpp"
    )));
}
