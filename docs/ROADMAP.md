# AURA Desktop — Durum, Bilinen Sınırlar, Yol Haritası

## ✅ Tamamlanan (kodlandı + derlendi + test edildi)
- Tauri v2 + Rust + React/TS iskeleti; release **`.app` + `.dmg`** build oluyor, açılıyor, çökmüyor.
- **Agent Manager**: detect/install/health/limit + **gömülü PTY login** (xterm), claude=ANA BEYİN. doctor JSON sözleşmesi (Python+Rust tek kaynak, test).
- **Veri**: tek `aura.sqlite` — FTS5 (gerçek) + vektör (brute-force) + cache/cache_deps/meta.
- **Indexer**: markdown/wikilink/hierarchical chunk + petgraph knowledge graph + content-hash incremental.
- **Arama**: hibrit FTS5+vektör → RRF.
- **AI ask**: exact-match cache (sıfır false-positive) → retrieval → lane → per-job `aura --json-events` spawn → Channel streaming → pgid cancel.
- **aura modu**: plan/review/fix/ship (Fix yalnız önizler).
- **Consensus** (default OFF): 3 AI paralel → claude sentezi, graceful degrade.
- **Lane 0** (default OFF): yerel Ollama üretimi.
- **UI**: CodeMirror 6 editör, react-force-graph graf, vault explorer, arama/ask/settings panelleri, Obsidian-dark tema, özel ikon.
- **BYOK** (v0.2.0): kendi Anthropic API anahtarınla çalış (app + CLI ortak `~/.aura/anthropic_api_key`, 0600; default OFF).
- **`.gitignore`-duyarlı indeksleme**: denylist + vault'un kendi `.gitignore`'u → kara-delik klasörler dışlanır.
- Cache invalidation dosya-hash'leriyle senkron (retrieval-fingerprint + dep content-hash); `tests/cache_invalidation.rs` ile kanıtlı.
- `cargo test`: **63 test / 27 suite PASS** (gemini→agy rename'inden kalan 3 stale test de düzeltildi); 0 derleme uyarısı.

## ⚠️ Kullanıcı gerektiren adımlar (headless yapılamaz)
1. **Canlı GUI QA**: `cd app && npm run tauri dev` ile aç; vault seç → indexle → ara → bir not sor (Ask). Backend komutları derlendi/test edildi ama görsel akışı senin doğrulaman gerekir.
2. **Notarization (genel dağıtım)**: Apple Developer ID ile `codesign --options runtime` + `xcrun notarytool submit --wait` + `stapler staple`. Faz 0 kararı: non-sandboxed + hardened + `inherit`. (Dev/lokal build imzasız çalışır.)
3. **Alt-CLI auth**: Agent Manager → her ajan için "Giriş" (PTY) ile OAuth; ya da terminalden `claude /login` vb.

## 🔭 Bilinçli ertelenenler (çalışan build'i riske atmamak için)
- **Gerçek candle/MLX embedding**: şu an `StubEmbedder` (deterministik) + FTS5 ana arama. candle+Metal ağır/riskli dep; FTS5 zaten gerçek arama veriyor. Sonra `Embedder` trait arkasında değiştirilebilir.
- **sqlite-vec gerçek ANN**: şu an brute-force cosine (ultraplan'da kabul edilen fallback); büyük vault'ta rusqlite+sqlite-vec'e geçiş.
- **Semantic-yakınlık cache**: şu an exact-match (sıfır yanlış-cevap); semantic opt-in + eval fixture sonraki sürüm.
- **Graph backend-layout**: >2000 düğümde JS yerine Rust layout.
- **JS bundle code-split**: tek chunk ~1MB (gzip 344KB) — desktop için sorun değil, istenirse bölünür.

## Mimari kaynaklar
`docs/ultraplan-FINAL.md` (master plan + playbook) · `docs/plan-v2.1.md` · `docs/faz0/bulgular.md` · `PROGRESS.md`.
