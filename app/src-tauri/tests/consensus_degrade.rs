use app_lib::consensus::{consensus_answer_mode, pick_synthesizer, ConsensusAnswerMode};

#[test]
fn synthesizer_prefers_claude_then_agy_then_codex() {
    // Preference order is claude > agy (Antigravity) > codex.
    assert_eq!(pick_synthesizer(&["codex", "agy"]), Some("agy"));
    assert_eq!(
        pick_synthesizer(&["codex", "claude", "agy"]),
        Some("claude")
    );
    assert_eq!(pick_synthesizer(&["codex"]), Some("codex"));
    assert_eq!(pick_synthesizer(&[]), None);
}

#[test]
fn answer_mode_distinguishes_empty_single_and_multi_answer_paths() {
    assert_eq!(consensus_answer_mode(0), ConsensusAnswerMode::NoAnswers);
    assert_eq!(consensus_answer_mode(1), ConsensusAnswerMode::SingleAgent);
    assert_eq!(consensus_answer_mode(2), ConsensusAnswerMode::Synthesize);
    assert_eq!(consensus_answer_mode(3), ConsensusAnswerMode::Synthesize);
}
