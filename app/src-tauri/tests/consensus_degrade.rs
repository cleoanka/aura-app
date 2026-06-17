use app_lib::consensus::{consensus_answer_mode, pick_synthesizer, ConsensusAnswerMode};

#[test]
fn synthesizer_prefers_claude_then_gemini_then_codex() {
    assert_eq!(pick_synthesizer(&["codex", "gemini"]), Some("gemini"));
    assert_eq!(
        pick_synthesizer(&["codex", "claude", "gemini"]),
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
