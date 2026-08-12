use super::*;
use crate::infer_runtime::tests::FakeTransport;

#[test]
fn unpublished_audio_event_capability_fails_closed_without_transport() {
    let client = InferRuntimeClient::with_transport(std::sync::Arc::new(FakeTransport::default()));
    let error = client
        .detect_audio_events(
            std::path::Path::new("not-opened.wav"),
            &AudioEventDetectionIntent::default(),
        )
        .unwrap_err();
    assert_eq!(error.kind, InferRuntimeErrorKind::ContractMismatch);
    assert_eq!(error.code, "capability_contract_unsupported");
}
