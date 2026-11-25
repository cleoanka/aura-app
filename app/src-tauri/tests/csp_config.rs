// CSP regresyon kalkanı: `"csp": null` iken markdown'daki uzak görsel/istek
// kullanıcı tıklamadan dışarı veri taşıyabiliyordu (prompt injection exfil).
// Frontend hiçbir uzak host'a doğrudan istek atmıyor; tüm backend erişimi IPC.

fn directive(csp: &str, name: &str) -> Vec<String> {
    csp.split(';')
        .map(str::trim)
        .find_map(|part| part.strip_prefix(name).filter(|rest| rest.starts_with(' ')))
        .unwrap_or_else(|| panic!("CSP'de {name} yok: {csp}"))
        .split_whitespace()
        .map(str::to_owned)
        .collect()
}

#[test]
fn csp_blocks_remote_loads_and_allows_only_ipc() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/tauri.conf.json");
    let config: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(path).expect("tauri.conf.json okunamadı"))
            .expect("tauri.conf.json geçerli JSON değil");
    let csp = config["app"]["security"]["csp"]
        .as_str()
        .expect("app.security.csp bir string olmalı (null = CSP kapalı)");

    assert_eq!(directive(csp, "default-src"), ["'self'"]);
    assert_eq!(directive(csp, "script-src"), ["'self'"]);
    assert_eq!(directive(csp, "img-src"), ["'self'", "data:"]);
    assert_eq!(
        directive(csp, "connect-src"),
        ["ipc:", "http://ipc.localhost"]
    );
    assert_eq!(directive(csp, "object-src"), ["'none'"]);
    assert!(!csp.contains("unsafe-eval"));
    assert!(!csp.contains("https:") && !csp.contains('*'));
}
