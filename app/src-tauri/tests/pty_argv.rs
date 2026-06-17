use app_lib::pty::login_argv;

#[test]
fn claude_login_argv_is_fixed() {
    let argv = login_argv("claude").expect("claude should be supported");

    assert_eq!(argv, vec!["claude", "/login"]);
}

#[test]
fn gemini_login_argv_is_fixed() {
    let argv = login_argv("gemini").expect("gemini should be supported");

    assert_eq!(argv, vec!["gemini"]);
}

#[test]
fn codex_login_argv_is_fixed() {
    let argv = login_argv("codex").expect("codex should be supported");

    assert_eq!(argv, vec!["codex", "login"]);
}

#[test]
fn unknown_agent_is_rejected() {
    assert!(login_argv("shell").is_err());
}
