// GÜVENLİK (codex #3): Ollama URL loopback kontrolü — userinfo bypass'ı dahil.
use app_lib::lane0::is_loopback_url;

#[test]
fn loopback_urls_pass() {
    assert!(is_loopback_url("http://localhost:11434"));
    assert!(is_loopback_url("http://127.0.0.1:11434"));
    assert!(is_loopback_url("http://[::1]:11434"));
    assert!(is_loopback_url("http://localhost"));
}

#[test]
fn non_loopback_and_bypass_rejected() {
    assert!(!is_loopback_url("http://evil.com:11434"));
    assert!(!is_loopback_url("http://10.0.0.5:11434"));
    // userinfo bypass: gerçek host evil.com
    assert!(!is_loopback_url("http://localhost:11434@evil.com"));
    assert!(!is_loopback_url("http://127.0.0.1@evil.com"));
    assert!(!is_loopback_url("https://attacker.example/api"));
}
