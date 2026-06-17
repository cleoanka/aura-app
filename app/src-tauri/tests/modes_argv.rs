use app_lib::exec::build_mode_argv;

#[test]
fn plan_and_fix_include_json_events_and_never_apply() {
    for mode in ["plan", "fix"] {
        let argv = build_mode_argv(mode, true).expect("mode should be valid");

        assert_eq!(argv[0], "aura");
        assert_eq!(argv[1], mode);
        assert!(argv.iter().any(|arg| arg == "--prompt-file"));
        assert!(argv.iter().any(|arg| arg == "--json-events"));
        assert!(!argv.iter().any(|arg| arg == "--apply"));
    }
}

#[test]
fn review_and_ship_do_not_include_json_events_or_apply() {
    for mode in ["review", "ship"] {
        let argv = build_mode_argv(mode, true).expect("mode should be valid");

        assert_eq!(argv[0], "aura");
        assert_eq!(argv[1], mode);
        assert!(argv.iter().any(|arg| arg == "--prompt-file"));
        assert!(!argv.iter().any(|arg| arg == "--json-events"));
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
