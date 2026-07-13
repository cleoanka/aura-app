# AURA Desktop — Durum, Bilinen Sınırlar, Yol Haritası

## ✅ Tamamlanan (kodlandı + derlendi + test edildi)
- Tauri v2 + Rust + React/TS iskeleti; release **`.app` + `.dmg`** build oluyor, açılıyor, çökmüyor.
- **Agent Manager**: detect/install/health/limit + **gömülü PTY login** (xterm), claude=ANA BEYİN. doctor JSON sözleşmesi (Python+Rust tek kaynak, test).
- **Veri**: tek `aura.sqlite` (rusqlite **bundled**) — FTS5 + **sqlite-vec vec0 ANN** (cosine, brute-force fallback) + cache/cache_deps/cache_query_vec/meta.
- **Indexer**: markdown/wikilink/hierarchical chunk + petgraph knowledge graph + content-hash incremental.
- **Arama**: hibrit FTS5+vektör → RRF.
- **AI ask**: exact-match cache (sıfır false-positive) → retrieval → lane → per-job `aura --json-events` spawn → Channel streaming → pgid cancel.
- **aura modu**: plan/review/fix/ship (Fix yalnız önizler).
- **Consensus** (default OFF): 3 AI paralel → claude sentezi, graceful degrade.
- **Lane 0** (default OFF): yerel Ollama üretimi.
- **UI**: CodeMirror 6 editör, react-force-graph graf, vault explorer, arama/ask/settings panelleri, Obsidian-dark tema, özel ikon.
- **BYOK** (v0.2.0): kendi Anthropic API anahtarınla çalış (app + CLI ortak `~/.aura/anthropic_api_key`, 0600; default OFF).
- **`.gitignore`-duyarlı indeksleme**: denylist + vault'un kendi `.gitignore`'u → kara-delik klasörler dışlanır.
- Cache invalidation dosya-hash'leriyle senkron (retrieval-fingerprint + dep content-hash + silinen not/chunk'ta girdi-silme); `tests/cache_invalidation.rs` ile kanıtlı.
- **Workspace (repo) semantiği**: aynı anda TEK aktif kök; liste/arama/graf/Ask aktif köke daralır; MRU switcher + "unut" (indeks temizliğiyle). `tests/workspace.rs` ile kanıtlı.
- **Gerçek candle/e5 embedding** (default feature, LAZY yükleme): runtime'da yalnız Settings'te *semantic search* açık ve model indirilmişken; aksi halde StubEmbedder + FTS5.
- **Semantic-cache** (opt-in, default OFF): cosine≥0.96 + dep-hash recheck; gerçek-e5 eval FP=0.
- **Async komut katmanı**: index/arama/agent-install/doctor komutları ana thread'i BLOKLAMAZ (spawn_blocking) → indeksleme sırasında UI donmaz.
- `cargo test`: **95 test / 31 suite PASS**; 0 derleme uyarısı.

## ⚠️ Kullanıcı gerektiren adımlar (headless yapılamaz)
1. **Canlı GUI QA**: `cd app && npm run tauri dev` ile aç; vault seç → indexle → ara → bir not sor (Ask). Backend komutları derlendi/test edildi ama görsel akışı senin doğrulaman gerekir.
2. **Notarization (genel dağıtım)**: Apple Developer ID ile `codesign --options runtime` + `xcrun notarytool submit --wait` + `stapler staple`. Faz 0 kararı: non-sandboxed + hardened + `inherit`. (Dev/lokal build imzasız çalışır.)
3. **Alt-CLI auth**: Agent Manager → her ajan için "Giriş" (PTY) ile OAuth; ya da terminalden `claude /login` vb.

## 🔭 Bilinçli ertelenenler (çalışan build'i riske atmamak için)
- ~~Gerçek candle/MLX embedding~~ **TAMAM** (a1): candle/e5 default feature + lazy yükleme; MLX değerlendirilmedi (CPU e5-small yeterli).
- ~~sqlite-vec gerçek ANN~~ **TAMAM** (a1): rusqlite bundled + vec0 KNN; brute-force artık yalnız küçük-vault fallback'i.
- ~~Semantic-yakınlık cache~~ **TAMAM** (a1): opt-in, çift kapı (threshold + dep-recheck), FP=0 eval.
- ~~JS bundle code-split~~ **TAMAM** (a1): vite manualChunks → 7 chunk (max editor ~610KB).
- **Graph backend-layout**: >2000 düğümde JS yerine Rust layout.
- **Indeksleme batch-commit**: dev vault'ta tek IMMEDIATE tx yerine N-dosyada-bir commit (kilit penceresi/hata-yarıçapı daralır). Mevcut ölçekte sorun değil.
- **Rerank tam-metin overlap**: rerank 200-karakter snippet üzerinde lexical-overlap hesaplıyor; tam chunk metnine geçirilebilir (advanced-retrieval opt-in olduğundan düşük öncelik).

## Mimari kaynaklar
`docs/ARCHITECTURE.md` · `STATE_OF_PROJECT.md` (güncel durum) · arşiv: `docs/history/ultraplan-FINAL.md` (master plan + playbook) · `docs/history/plan-v2.1.md` · `docs/history/faz0/bulgular.md` · `docs/history/PROGRESS.md`.
