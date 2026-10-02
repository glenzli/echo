use super::*;

#[test]
fn scan_requires_at_least_one_worker() {
    assert!(Cli::try_parse_from(["echo-cli", "scan", "absent.sqlite", "--workers", "0"]).is_err());
    for workers in ["1", "2"] {
        assert!(
            Cli::try_parse_from(["echo-cli", "scan", "absent.sqlite", "--workers", workers])
                .is_ok()
        );
    }
}
