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

#[test]
fn edit_inspect_requires_an_explicit_session_and_exposes_only_supported_commands() {
    assert!(Cli::try_parse_from(["echo-cli", "edit", "inspect"]).is_err());
    assert!(
        Cli::try_parse_from([
            "echo-cli",
            "edit",
            "inspect",
            "--session",
            "private-session"
        ])
        .is_ok()
    );
    assert!(Cli::try_parse_from(["echo-cli", "edit", "capabilities"]).is_ok());
    for operation in ["apply", "preview", "export"] {
        assert!(Cli::try_parse_from(["echo-cli", "edit", operation]).is_err());
    }
}
