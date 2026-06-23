# IDEAS — backlog & öncelik

> Maraton döngüsünün yakıtı. Kategori etiketleri: A doğruluk · B kararlılık ·
> C performans · D araştırma/icat · E optimizasyon · F UI/UX · G test · H kod
> sağlığı/doküman · I gözlemlenebilirlik · J ölçeklendirme/yetenek · K repo/yayın.
> Aynı kategori arka arkaya 2'den fazla seçilemez. Tükenince zoom-out + yeni fikir üret.

## Tamamlandı (Döngü 1–14)
- [x] [G] indexer `.gitignore`/`snippet`/`IndexStats` testleri (D1/D5/D6) · ai.rs deep_query/normalize/fingerprint (D12) · apikey validate_key (D3)
- [x] [B/A] BYOK key doğrulaması app+CLI paritesi (D3/D8)
- [x] [I/F] `IndexStats.elapsed_ms` backend + UI gösterimi (D5/D9) + workspace görseli (D11)
- [x] [J] `.auraignore` desteği (D13)
- [x] [K/H] CI rozeti (D4), docs linkleme + CHANGELOG güncel (D2/D10), RESEARCH kritiği (D7)

## Yüksek öncelik (sıradaki)
- [J] **rusqlite + sqlite-vec'e geçiş** (tech-debt): sistem `libsqlite3` FFI + brute-force cosine → gerçek ANN. Flag arkasında A/B, `BENCHMARKS.md`'de gecikme ölç. (RİSK: yüksek — dep+FFI; kapılarla koru.)
- [x] [C] **Semantic-cache** TAMAM (D25 eval FP=0 / D26 db / D27 ai-wiring, default-OFF). Açık: UI toggle + daha geniş eval ile eşik ~0.90'a indirme.
- [F] **JS bundle code-split**: tek chunk ~1.57MB (gzip 491KB). manualChunks ile böl.
- [x] [B] **Stress test** eşzamanlı reindex↔ask TAMAM (D24).

## Orta
- [H] **`docs/` derinleştir**: API yüzeyini dokümante et; diyagramları güncel tut.
- [J] **Notarization pipeline** (kullanıcı Apple Developer ID gerektirir): script + CHANGELOG akışı.
- [I] **Arama/cache metrikleri** UI'da (LiveActivity): arama gecikmesi, cache hit oranı.

## Araştırma (RESEARCH/)
- [D] Daha iyi RRF/rerank ağırlıkları; graph-retrieval sinyal katkısının ölçümü (eval gerekir).
- [D] Rust-side force-graph layout (>2000 düğüm) — JS yerine backend.
- [D] Incremental embedding scheduler (idle-time, nazik throttle).

## Rafa kalkanlar (time-box aşıldı / yakınsamadı)
- (henüz yok)

## Rafa kalkanlar (time-box aşıldı / yakınsamadı)
- (henüz yok)
