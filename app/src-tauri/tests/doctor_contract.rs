use app_lib::agent::{DoctorReport, TokenLocation};

#[test]
fn doctor_fixture_matches_rust_contract() {
    let fixture_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../contracts/doctor.fixture.json"
    );
    let fixture = std::fs::read_to_string(fixture_path).expect("doctor fixture should be readable");
    let report =
        serde_json::from_str::<DoctorReport>(&fixture).expect("doctor fixture should deserialize");

    assert_eq!(report.schema, "aura.doctor.v1");
    assert!(report.agents.contains_key("claude"));
    assert!(report.agents.contains_key("gemini"));
    assert!(report.agents.contains_key("codex"));

    let claude = report.agents.get("claude").expect("claude agent exists");
    assert!(matches!(&claude.token_location, TokenLocation::Keychain));
}
