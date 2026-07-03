# BENCHMARKS — metrik tabanı & seyri

> "benchmark" = AURA'nın gerçek başarı metrikleri. Her zoom-out heartbeat'te (ve
> kilit değişikliklerde) güncelle. Regresyon = bu sayıların kötüleşmesi → kapı patlar.

## Taban (2026-06-23, branch `a1`, v0.2.0)

| Metrik | Değer | Nasıl ölçüldü |
|---|---|---|
| Rust testleri | **95 passed, 0 fail** (taban 63 → +32) | `cargo test --locked` |
| Frontend testleri | **10 passed** | `npm test` (vitest) |
| Tip kontrolü | **0 hata** | `npm run build` (tsc) |
| JS bundle | **7 chunk** (D16 split); en büyük editor 610KB / app index 80KB (önce: tek 1,572KB) | vite build çıktısı |
| Release `.app` | ~19 MB · `.dmg` ~8.5 MB (arm64) | `npm run tauri build` |
| `aura` cold-start ek-median | **~30 ms** (« 1.5s eşik) | Faz 0 ölçümü |
| soul_check | ✅ geçiyor | `scripts/soul_check.py` |
| Binary'de kişisel-veri | **0** (remapped build) | `strings … | grep` |

## İndeks & arama (sentetik vault: 360 dosya / 960 chunk, debug build, M-serisi)

| Metrik | Değer | Nasıl ölçüldü |
|---|---|---|
| Cold index | **~2.3 s** | `cargo test --test bench_index -- --ignored --nocapture` |
| Warm (değişmemiş) reindeks | **~11 ms** (önce ~2.2 s — 200×) | aynı bench, ikinci tur |
| Hibrit arama (FTS+vec+RRF) | **~0.4 ms** | aynı bench |

> Warm-yol kazanımı: değişmemiş dosyada hash-önce kontrol + lazy `title_aliases`
> (ikinci tam-okuma turu kalktı) + dosya-kümesi parmak iziyle link-churn atlama.
> Açılıştaki otomatik reindeks artık fiilen bedava → "uygulama kasıyor" ana kökü kapandı.

## Henüz ölçülmedi (çalışan vault gerektirir — ölç ve doldur)
- Embedding throughput (candle e5, batch) · cache hit oranı (tekrarlı sorgu seti)
- Ask cevabı ilk-token gecikmesi (Fast / Deep lane)

## Seyir
<!-- Her anlamlı iyileşmeyi tarih + commit + önce→sonra ile buraya ekle. -->
- 2026-06-23: taban kaydedildi.
- 2026-06-23 (D16/D30-33): bundle 1→7 chunk; sqlite-vec vec0 ANN; VaultExplorer content-visibility.
- 2026-07-03: **workspace (repo) semantiği** + async komut katmanı (UI donması kökten çözüldü);
  warm reindeks 2.2 s → 11 ms; sorgu embed pad'i kalktı (~10-30× daha az BERT hesabı);
  `synchronous=NORMAL` + embed batch tek-tx; cargo test 88 → 95; `bench_index` eklendi.
