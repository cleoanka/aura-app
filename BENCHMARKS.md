# BENCHMARKS — metrik tabanı & seyri

> "benchmark" = AURA'nın gerçek başarı metrikleri. Her zoom-out heartbeat'te (ve
> kilit değişikliklerde) güncelle. Regresyon = bu sayıların kötüleşmesi → kapı patlar.

## Taban (2026-06-23, branch `a1`, v0.2.0)

| Metrik | Değer | Nasıl ölçüldü |
|---|---|---|
| Rust testleri | **85 passed, 0 fail** (taban 63 → +22) | `cargo test --locked` |
| Frontend testleri | **10 passed** | `npm test` (vitest) |
| Tip kontrolü | **0 hata** | `npm run build` (tsc) |
| JS bundle | **7 chunk** (D16 split); en büyük editor 610KB / app index 80KB (önce: tek 1,572KB) | vite build çıktısı |
| Release `.app` | ~19 MB · `.dmg` ~8.5 MB (arm64) | `npm run tauri build` |
| `aura` cold-start ek-median | **~30 ms** (« 1.5s eşik) | Faz 0 ölçümü |
| soul_check | ✅ geçiyor | `scripts/soul_check.py` |
| Binary'de kişisel-veri | **0** (remapped build) | `strings … | grep` |

## Henüz ölçülmedi (çalışan vault gerektirir — ölç ve doldur)
- Index süresi (N dosyalık vault) · ilk-byte arama gecikmesi (FTS5 / hibrit)
- Embedding throughput (candle e5, batch) · cache hit oranı (tekrarlı sorgu seti)
- Ask cevabı ilk-token gecikmesi (Fast / Deep lane)

## Seyir
<!-- Her anlamlı iyileşmeyi tarih + commit + önce→sonra ile buraya ekle. -->
- 2026-06-23: taban kaydedildi.
