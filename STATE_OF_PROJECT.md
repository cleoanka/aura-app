# STATE OF PROJECT — branch `a1` (güncel: 2026-07-03)

İnsan-incelemesi için devir dokümanı. `a1`, `main` (v0.2.0) üzerine otonom maraton
dalı; gözden geçirilip uygun görülürse merge edilir. Hiçbir döngü `main`'e dokunmadı.
Anlık durum (final değil).

## 2026-07-03 oturumu — "mükemmelleştir" turu (özet)
Çok-ajanlı derin denetim (7 paralel okuyucu → 69 ham bulgu → 42 bug/perf adversarial
doğrulamada 24 onay) + uygulama. Ana başlıklar:
- **Workspace (repo) semantiği**: TEK aktif kök; liste/arama/graf/Ask aktif köke daralır;
  MRU switcher + "unut" (DB izleri dahil temizlik). `tests/workspace.rs`.
- **UI donması kökten çözüldü**: ağır komutlar async+`spawn_blocking` (sync komut ana
  thread'de Indexer kilidi bekliyordu); açılışta yalnız aktif kök reindekslenir; candle
  lazy yüklenir; warm reindeks 2.2 s → **11 ms** (hash-önce + lazy title_aliases + paths-fp).
- **Cache doğruluğu (kritik fix)**: silinen not/chunk'ta CASCADE dep-silme bayat cevabı
  "geçerli" bırakıyordu → silme-öncesi cache-eviction + regresyon testleri.
- **file_id yol-tabanlı** (atomic-save'de tam re-embed/cache-wipe bitti); NUL/okunamayan-dizin
  dayanıklılığı; lane-settings IPC uyumsuzluğu (toggle'lar no-op'tu); lane0 artık iptal
  edilebilir; consensus stderr/sayaç/timeout düzeltmeleri; PTY zombi fix; i18n/a11y temizliği.
- Dokümanlar koda hizalandı (README/ROADMAP/ARCHITECTURE/BENCHMARKS/CHANGELOG) +
  workspace-switcher görseli & animasyonlu GIF.

## Özet
- **FAZ 1 + 38 döngü + 2026-07-03 turu**, hepsi atomik + koruma kapılarından geçti (ya da kapı patladıysa ROLLBACK + belge).
- **`a1-known-good`** her zaman yeşil noktada (rollback çıpası).
- Backlog'un tamamı: ya **uygulandı+test edildi**, ya **test edilip gerekçeyle elendi**, ya da **harici bağımlılık** nedeniyle bloklu.

## Metrikler (taban v0.2.0 → şimdi)
| | taban | a1 |
|---|---|---|
| Rust testleri | 63 | **95** (+32) · ayrıca `#[ignore]` gerçek-e5 eval + `bench_index` |
| Frontend (vitest) | 10 | 10 |
| tsc / soul_check | — | 0 hata / ✅ (CI'de) |
| JS bundle | tek 1.57MB | **7 chunk** (max editor 610KB) |
| Warm reindeks (360 dosya) | ~2.2 s | **~11 ms** |

## Eklenen & test edilen (büyük→küçük)
- **[C] Semantic-cache (opt-in, default OFF):** db `cache_query_vec` + `semantic_cache_lookup` (cosine≥threshold **VE** dep-hash recheck = anayasa Madde 9) + ai.rs entegrasyonu. **Gerçek-e5 eval'i: false-positive=0 @0.96** (`tests/semantic_cache_eval.rs`, #[ignore]). Açık: UI toggle + daha geniş eval ile eşik ~0.90.
- **[B] Stress:** eşzamanlı reindex↔search stabilite testi.
- **[F] Bundle split:** vite manualChunks → 1.57MB tek chunk yerine 7 chunk.
- **[J] `.auraignore`:** git'e dokunmadan indekslemeden hariç tutma.
- **[B/A] BYOK doğrulaması:** app `validate_key` + CLI `aura key set` parite; read_key first-line parse.
- **[I] `elapsed_ms`:** backend + VaultExplorer UI + workspace görseli.
- **[G] +22 test:** indexer (.gitignore/.auraignore/snippet/IndexStats), ai (deep_query/normalize/fingerprint), apikey, db (normalize_embedding/semantic-cache), markdown (chunk_stable_id), cache (edit/no-op/missing).
- **Altyapı:** `soul_check.py`+CI, DEV_JOURNAL/IDEAS/BENCHMARKS/RESEARCH, CONTRIBUTING/CHANGELOG/CITATION, docs/{philosophy,simple,glossary,development}, CI rozeti.

## [J] sqlite-vec ANN — TAMAMLANDI (D28 spike → D30-31)
- Spike sistem-sqlite'ta SQLITE_MISUSE verdi → veri katmanı **rusqlite bundled sqlite**'a taşındı (hibrit: FFI mantığı korundu, Connection rusqlite'ı sarar). rusqlite 0.32 (0.40→libsqlite3-sys 0.38 `cfg_select` rustc1.93'te unstable'dı).
- `vec_search` artık **sqlite-vec vec0 KNN** (cosine); vec_ann türetilmiş index (self-heal backfill + stale-filter + brute-force fallback). **87 test davranış-eşdeğer.** Bkz. `RESEARCH/2026-06-23-sqlite-vec-spike.md`.

## Bloklu / harici bağımlılık
- **Notarization:** Apple Developer ID gerektirir (runtime değil; senin hesabın).
- **Canlı GUI QA:** `cd app && npm run tauri dev` (görsel akış senin onayın).

## İncelerken
- `git log --oneline main..a1` · her commit atomik + DEV_JOURNAL'da gerekçeli.
- Kapılar: `python3 scripts/soul_check.py` · `cd app/src-tauri && cargo test --locked` · `cd app && npm run build && npm test`.
- Merge: hepsi yeşil + main'e dokunulmadı + kişisel veri yok (soul_check enforce).

## Maraton durumu (2026-06-24, FAZ 1 + 38 döngü)
Genuine güvenli backlog **tükendi** — büyük kalemler (semantic-cache, sqlite-vec ANN, stress, bundle-split, BYOK, .auraignore) bitti; 88 test, docs koda hizalı, perf (content-visibility) + gözlemlenebilirlik (elapsed_ms, arama gecikmesi) yapıldı. Kalan işler **deliberate** (semantic-cache threshold'u release-build + 50+ çiftlik eval ister; cron-paced değil) ya da **harici-bloklu** (notarization=Apple ID, GUI QA). Padding yapmamak için 5-dk cron durduruldu. `a1` inceleme/merge'e hazır.
