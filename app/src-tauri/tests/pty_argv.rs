use app_lib::pty::login_argv;

#[test]
fn claude_login_argv_is_fixed() {
    let argv = login_argv("claude").expect("claude should be supported");

    assert_eq!(argv, vec!["claude", "/login"]);
}

#[test]
fn agy_login_argv_is_fixed() {
    let argv = login_argv("agy").expect("agy should be supported");

    assert_eq!(argv, vec!["agy"]);
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
