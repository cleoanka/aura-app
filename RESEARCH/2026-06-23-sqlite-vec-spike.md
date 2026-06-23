# Spike — sqlite-vec ANN (2026-06-23) → DENENDİ, ÇÖPE ATILDI (bu mimaride)

## Hedef
[J]: brute-force O(N) vektör aramasını gerçek ANN (sqlite-vec `vec0`) ile değiştir.

## Deney
- `cargo add sqlite-vec` (v0.1.9) — eklendi, C eklentisi derlendi ✓.
- Mevcut **FFI + sistem libsqlite3** bağlantısına `sqlite3_auto_extension(sqlite3_vec_init)` ile kaydetme + `CREATE VIRTUAL TABLE … USING vec0(…)` + KNN smoke testi yazıldı.

## Sonuç: BAŞARISIZ (kesin)
```
sqlite3_auto_extension(...) -> 21  (SQLITE_MISUSE), beklenen 0 (SQLITE_OK)
```
**macOS sistem libsqlite3'ü çalışma-zamanı eklenti kaydına izin vermiyor** (Apple güvenlik gereği load-extension'ı kısıtlar). Dolayısıyla sqlite-vec, projenin mevcut **sistem-sqlite + el-yazımı FFI** mimarisiyle **çalışmaz**.

## Karar
- Spike **rollback** edildi (`git reset --hard a1-known-good`); `sqlite-vec` dep'i kaldırıldı. Brute-force cosine korunuyor (<~50k chunk'ta sorunsuz; partial top-k + bellek korumalı).
- **[J] yalnızca şu yolla mümkün:** veri katmanını **bundled sqlite**'a (rusqlite `--features bundled` veya kendi statik sqlite derlememiz, extension-loading açık) taşımak — büyük, anayasa-kritik (Madde 9: cache/db doğruluğu) bir **migration**. Bu, kör otonom loop'ta güvenli DEĞİL: ~1400 satır FFI'ın rusqlite'a çevrilmesi + tüm db testlerinin yeniden doğrulanması + gerçek büyük-vault gecikme benchmark'ı gerektirir.

## Öneri (insan-onaylı ayrı iş)
1. `db/mod.rs`'i rusqlite (bundled) arkasına al — API yüzeyi (`open/execute/query/cache_*`) korunur, içi değişir.
2. Extension-loading açık → `sqlite3_vec_init` auto_extension ile kaydedilebilir.
3. `vec_chunks` → `vec0` virtual table; `vec_search` → KNN `MATCH … ORDER BY distance`.
4. Kapı: tüm db testleri + gerçek-vault gecikme A/B (`BENCHMARKS.md`); brute-force fallback feature-flag.
