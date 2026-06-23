# Geliştirme — otonom maraton & kapılar

AURA, **anayasa + koruma kapıları** disipliniyle geliştirilir. Bu doküman akışı
özetler; felsefe için [`philosophy.md`](philosophy.md), mimari için
[`ARCHITECTURE.md`](ARCHITECTURE.md).

## Branch modeli
- `main` — yayınlanan, CI-yeşil sürüm (v0.x).
- `a1` — otonom maraton dalı (gözden geçirilip uygun görülürse `main`'e merge edilir).
- `a1-known-good` (tag) — her şeyin yeşil olduğu son nokta = **rollback çıpası**.

## Koruma kapıları (her LAND'den önce)
```bash
python3 scripts/soul_check.py                 # anayasa (gizlilik/güvenlik/varsayılanlar)
cd app/src-tauri && cargo test --locked       # Rust
cd app && npm run build && npm test           # tsc + vitest
```
+ kilit metrikte regresyon yok (bkz. [`../BENCHMARKS.md`](../BENCHMARKS.md)) · kaynak/disk güvenli.
Biri patlarsa: `git reset --hard a1-known-good` (ROLLBACK) + sebebi `DEV_JOURNAL.md`'ye yaz.

## Döngü (atomik, 10 adım)
ORIENT → ASSESS → SELECT (kategori; rotasyon: aynı kategori ardışık 2'den fazla olamaz) →
PLAN → CHECKPOINT → EXECUTE → VERIFY (kapılar) → DECIDE (LAND/ROLLBACK) → JOURNAL → REPEAT.
**Sondan-önceki adım** = test/kritik/araştırma; **son adım** = yeni plan yapıp uygula.

## Süreklilik
- [`DEV_JOURNAL.md`](../DEV_JOURNAL.md) — döngü kayıtları (oturum koparsa buradan devam).
- [`IDEAS.md`](../IDEAS.md) — backlog + kategori etiketleri.
- [`BENCHMARKS.md`](../BENCHMARKS.md) — metrik tabanı & seyri.
- [`RESEARCH/`](../RESEARCH) — araştırma/kritik notları.

## Sürüm & yayın
- Sürüm: `app/package.json` + `app/src-tauri/tauri.conf.json` + `Cargo.toml` (üçü senkron).
- Release binary **`RUSTFLAGS="--remap-path-prefix=$HOME=/build"`** ile derlenir (kişisel yol sızmaz).
- `.dmg` GUI gerektirir → CI yalnız `--bundles app`; imzalı `.dmg` yerelde üretilip GitHub Release'e eklenir.
