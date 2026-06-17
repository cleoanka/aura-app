use app_lib::exec::build_mode_argv;

#[test]
fn all_modes_include_json_events_and_never_apply() {
    // Tüm modlar artık --json-events (verbose status + canlı akış); hiçbiri --apply değil.
    for mode in ["plan", "fix", "review", "ship"] {
        let argv = build_mode_argv(mode, true).expect("mode should be valid");

        assert_eq!(argv[0], "aura");
        assert_eq!(argv[1], mode);
        assert!(argv.iter().any(|arg| arg == "--prompt-file"));
        assert!(argv.iter().any(|arg| arg == "--json-events"));
        assert!(!argv.iter().any(|arg| arg == "--apply"));
    }
}

#[test]
fn prompt_file_is_omitted_when_prompt_is_empty() {
    let argv = build_mode_argv("plan", false).expect("mode should be valid");

    assert!(!argv.iter().any(|arg| arg == "--prompt-file"));
    assert!(argv.iter().any(|arg| arg == "--json-events"));
}

#[test]
fn invalid_mode_is_rejected() {
    assert!(build_mode_argv("status", true).is_err());
}
