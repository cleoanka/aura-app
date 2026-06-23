# STATE OF PROJECT — branch `a1` (2026-06-23)

İnsan-incelemesi için devir dokümanı. `a1`, `main` (v0.2.0) üzerine otonom maraton
dalı; gözden geçirilip uygun görülürse merge edilir. Hiçbir döngü `main`'e dokunmadı.

## Özet
- **FAZ 1 + 22 döngü**, hepsi atomik + koruma kapılarından geçti.
- **`a1-known-good`** her zaman yeşil noktada (rollback çıpası).
- Kategori rotasyonu uygulandı (aynı kategori ardışık 2'den fazla seçilmedi).

## Metrikler (taban → şimdi)
| | taban (v0.2.0) | a1 |
|---|---|---|
| Rust testleri | 63 | **82** (+19) |
| Frontend (vitest) | 10 | 10 |
| tsc | 0 hata | 0 hata |
| JS bundle | tek 1.57MB | **7 chunk** (max editor 610KB) |
| soul_check | yoktu | **✅ CI'de** |

## Ne yapıldı (kategoriye göre)
- **Altyapı/anayasa:** `scripts/soul_check.py` (gizlilik/güvenlik/varsayılan denetimi) + CI; `DEV_JOURNAL`/`IDEAS`/`BENCHMARKS`/`RESEARCH/`; `CONTRIBUTING`/`CHANGELOG`/`CITATION`; `docs/{philosophy,simple,glossary,development}.md`.
- **Test (+19):** indexer `.gitignore`/`.auraignore`/`snippet`/`IndexStats`; ai `deep_query`/`normalize`/`fingerprint`; apikey `validate_key`/`parse_key_file`; db `normalize_embedding`; markdown `chunk_stable_id`; cache no-op-reindex.
- **Özellik/sağlamlık:** BYOK key doğrulaması (app+CLI parite); `.auraignore` (git'e dokunmadan hariç tut); `IndexStats.elapsed_ms` (backend+UI+görsel); read_key ilk-satır parse.
- **Perf:** vite manualChunks (bundle split).
- **Görsel/doküman:** CI rozeti, workspace screenshot index-stats footer, README/CHANGELOG/docs güncel.

## İncelerken bak
- `git log --oneline main..a1` (24 commit) · her commit atomik + DEV_JOURNAL'da gerekçeli.
- Kapıları kendin koştur: `python3 scripts/soul_check.py` · `cd app/src-tauri && cargo test --locked` · `cd app && npm run build && npm test`.
- Merge kararı: hepsi yeşil + main'e dokunulmadı + kişisel veri yok (soul_check enforce).

## Sıradaki (runtime/insan gerektirir — kör otonom loop'ta yapılmadı)
- **[C] semantic-cache:** tasarım hazır (`RESEARCH/2026-06-23-semantic-cache.md`) ama **eval fixture gerçek e5 modeli** ister; anayasa Madde 9 (sıfır yanlış-cevap) → false-positive=0 kanıtlanmadan default açılamaz.
- **[J] rusqlite + sqlite-vec (ANN):** yüksek riskli dep/FFI değişimi; gerçek vault'ta gecikme benchmark'ı ile A/B.
- **[B] eşzamanlı reindex↔ask stress:** çok-thread runtime senaryosu.
- **Notarization:** kullanıcı Apple Developer ID'si gerektirir.
- **Canlı GUI QA:** `cd app && npm run tauri dev`.

> Bu doküman maraton ilerledikçe güncellenir; final değil, anlık durum.
