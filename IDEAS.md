# IDEAS — backlog & öncelik

> Maraton döngüsünün yakıtı. Kategori etiketleri: A doğruluk · B kararlılık ·
> C performans · D araştırma/icat · E optimizasyon · F UI/UX · G test · H kod
> sağlığı/doküman · I gözlemlenebilirlik · J ölçeklendirme/yetenek · K repo/yayın.
> Aynı kategori arka arkaya 2'den fazla seçilemez. Tükenince zoom-out + yeni fikir üret.

## Yüksek öncelik
- [J] **rusqlite + sqlite-vec'e geçiş** (tech-debt): şu an sistem `libsqlite3` FFI + brute-force cosine. Gerçek ANN → büyük vault'ta arama gecikmesi düşer. Flag arkasında A/B, `BENCHMARKS.md`'de ölç. (RİSK: yüksek — dep + FFI değişimi; kapılarla koru.)
- [G] **gitignore_names / is_ignored_path için unit test** (indexer.rs): yeni `.gitignore` parse mantığı testsiz. Düşük risk, hızlı kazanç.
- [C] **Semantic-yakınlık cache** (opt-in) + eval fixture: exact-match'in üstüne, anayasayı (sıfır yanlış-cevap) bozmadan; threshold + eval ile.
- [F] **JS bundle code-split**: tek chunk ~1.57MB (gzip 491KB). manualChunks ile böl; desktop için kritik değil ama temiz.

## Orta
- [I] **Yapılandırılmış log/metrik**: index süresi, arama gecikmesi, cache hit oranı app içinde görünür (LiveActivity genişlet).
- [B] **Eşzamanlı reindex ↔ ask** dayanıklılık taraması (FK/lock edge-case'leri zaten ele alınmış; fuzz/stress testi ekle).
- [H] **`docs/` derinleştir**: architecture diyagramlarını güncel tut; API yüzeyini dokümante et.
- [J] **Notarization pipeline** (kullanıcı Apple Developer ID gerektirir): script + CHANGELOG akışı.

## Araştırma (RESEARCH/)
- [D] Daha iyi RRF/rerank ağırlıkları; graph-retrieval sinyalinin katkısının ölçümü.
- [D] Rust-side force-graph layout (>2000 düğüm) — JS yerine backend.
- [D] Incremental embedding scheduler (idle-time, nazik throttle).

## Rafa kalkanlar (time-box aşıldı / yakınsamadı)
- (henüz yok)
