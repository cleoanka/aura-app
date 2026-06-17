# AURA Desktop (aura-app)

macOS Apple Silicon, Obsidian-benzeri AI-native yerel Markdown "ikinci beyin".

- **Motor:** mevcut `aura` CLI orkestratörü (claude = ana beyin, gemini = research, codex = implementation). Sıfırdan LLM entegrasyonu yok.
- **Yığın:** Tauri v2 + Rust + React/TS/Vite · sqlite-vec + FTS5 (RRF hibrit arama) · candle (embedding) · CodeMirror 6 · react-force-graph.
- **Özellikler:** Ask (RAG) + aura-mode (plan/review/fix/ship) + opsiyonel Consensus (3 AI → sentez, varsayılan kapalı) · iki yerel katman (embedding + Lane 0 yerel üretim) · uygulama-içi Agent Manager (CLI'ları kur/login-PTY/health/limit) · bol Settings.

> Bu, TEKNOFEST AURA yarışma projesinden (`~/aura`) ve `aura` CLI'ından (`~/.local/bin/aura`) AYRI bir uygulamadır.

## Durum
Henüz kod yok — **plan aşaması**. Bkz:
- `docs/plan-v2.1.md` — red-team sonrası v2.1 mimari plan.
- `docs/ultraplan-FINAL.md` — (yakında) uygulama playbook'u (Opus 4.8 + Codex için adım-adım).

## Uygulayıcı
Opus 4.8 (mimar + reviewer, ağır yük) + en güncel Codex CLI (implementer). Geliştirme: Rust 1.93, Node 24, Tauri CLI, Xcode CLT, M4 Pro (arm64).
