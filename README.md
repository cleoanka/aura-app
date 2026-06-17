# AURA Desktop

macOS Apple Silicon için **Obsidian-benzeri, AI-native yerel Markdown "ikinci beyin"**.

Motor: mevcut `aura` CLI orkestratörü (**claude = ana beyin**, gemini = research, codex = implementation). Sıfırdan LLM entegrasyonu yok — kanıtlanmış CLI'lar `zsh -lc` + stdin ile sarılır.

> Bu uygulama, TEKNOFEST AURA yarışma projesinden (`~/aura`) ve `aura` CLI'ından (`~/.local/bin/aura`) **ayrıdır**.

## Özellikler
- **Ask (ikinci beyin):** notların üzerinde hibrit arama (FTS5 + vektör → RRF) + RAG; **exact-match cache** (sıfır yanlış-cevap), streaming cevap, lane rozeti.
- **aura modu:** `plan / review / fix / ship` uygulama içinden (Fix **yalnız önizler**, dosya değiştirmez; asla commit etmez).
- **Consensus (opsiyon, varsayılan KAPALI):** aynı soruyu claude+gemini+codex'e paralel sorar, **claude sentezler**; tek-iki ajan düşse de çalışır.
- **İki yerel katman:** (a) yerel embedding (arama/cache, candle hedef), (b) **Lane 0** yerel üretim (Ollama, opsiyonel, varsayılan kapalı).
- **Agent Manager:** claude/gemini/codex'i app içinden **algıla / kur / giriş (gömülü PTY/xterm) / sağlık / limit**. claude = ANA BEYİN.
- **Bilgi grafiği:** `[[wikilink]]` → react-force-graph; dangling düğümler; tıkla→aç.
- **CodeMirror 6** markdown editör; **bol Settings** (zero-config başlar, bozuk alan varsayılana düşer — çökmez).

## Mimari
Tauri v2 + Rust (backend) + React/TS/Vite (frontend). Veri: tek `aura.sqlite` (FTS5 + vektör + cache). Per-job kısa-ömürlü `aura` süreci (daemon yok); iptal = process-group kill; prompt/context **dosya→stdin** (shell-injection yok). Detaylar: `docs/ultraplan-FINAL.md` (master plan + playbook), `docs/plan-v2.1.md`, `docs/faz0/`.

## Geliştirme
```bash
cd app
npm install
npm run tauri dev      # geliştirme (pencere açılır)
npm run tauri build    # release .app + .dmg (target/release/bundle/)
```
Gereksinimler: macOS arm64, Rust 1.93+, Node 24+, Xcode CLT. `aura` CLI PATH'te olmalı (`~/.local/bin/aura`), üç alt-CLI auth'lu (Agent Manager'dan giriş yapılabilir).

## Dağıtım (notarization)
Release `.app`/`.dmg` üretiliyor (ad-hoc imzalı, lokal çalışır). Genel dağıtım için **Apple Developer ID** ile `codesign --options runtime` + `xcrun notarytool submit` + `stapler staple` gerekir (kullanıcının Apple kimliğini ister). Faz 0 kararı: **non-sandboxed Developer ID + hardened runtime + `com.apple.security.inherit`** (alt-CLI'lar kendi keychain/dosya auth'una erişir).

## Durum
Çekirdek tamam ve `.app` + `.dmg` build oluyor. İlerleme: `PROGRESS.md`. Üretildi: Opus 4.8 (orkestratör/mimar) + Codex (implementer) + Gemini (doğrulama) — `aura` modeliyle dogfood.
