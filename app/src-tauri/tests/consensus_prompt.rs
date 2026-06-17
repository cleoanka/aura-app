use app_lib::consensus::synth_prompt;

#[test]
fn synth_prompt_includes_query_answers_and_instruction() {
    let answers = vec![
        (
            "claude".to_string(),
            "Claude says use indexed context.".to_string(),
        ),
        (
            "gemini".to_string(),
            "Gemini says flag uncertain claims.".to_string(),
        ),
        (
            "codex".to_string(),
            "Codex says keep the answer concise.".to_string(),
        ),
    ];

    let prompt = synth_prompt("How should consensus work?", &answers);

    assert!(prompt.contains("How should consensus work?"));
    assert!(prompt.contains("claude"));
    assert!(prompt.contains("Claude says use indexed context."));
    assert!(prompt.contains("gemini"));
    assert!(prompt.contains("Gemini says flag uncertain claims."));
    assert!(prompt.contains("codex"));
    assert!(prompt.contains("Codex says keep the answer concise."));
    assert!(prompt.contains("Note agreements"));
    assert!(prompt.contains("flag material conflicts"));
    assert!(prompt.contains("ONE best synthesized answer"));
}

#[test]
fn synth_prompt_accepts_two_answers_for_graceful_degradation() {
    let answers = vec![
        ("claude".to_string(), "First available answer.".to_string()),
        ("gemini".to_string(), "Second available answer.".to_string()),
    ];

    let prompt = synth_prompt("What if one agent fails?", &answers);

    assert!(prompt.contains("What if one agent fails?"));
    assert!(prompt.contains("First available answer."));
    assert!(prompt.contains("Second available answer."));
    assert!(prompt.contains("SYNTHESIZED CONSENSUS ANSWER"));
}
