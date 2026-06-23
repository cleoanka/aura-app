# DEV_JOURNAL — AURA Desktop otonom geliştirme günlüğü

> Süreklilik dosyalarda yaşar. Oturum koparsa: bu günlüğü oku → gerekirse
> `git reset --hard a1-known-good` → kaldığın döngüden devam et. Her döngü
> ATOMİK bir değişiklik + koruma kapıları (derleme · testler · `soul_check` ·
> kilit metrikte regresyon yok). Kapı patlarsa ROLLBACK + sebebi buraya yaz.

## Koruma kapıları (her LAND'den önce)
1. `cd app/src-tauri && cargo test --locked`
2. `cd app && npm run build && npm test`
3. `python3 scripts/soul_check.py`
4. Kilit metrikte regresyon yok (bkz. `BENCHMARKS.md`); test sayısı azalmadı
5. Kaynak/kararlılık (kaçak süreç/RAM/disk yok; disk ≥10GB headroom)

---

## Döngü kayıtları

### 2026-06-23 — Döngü 6 [Kategori G: test]
- **Seçim:** `snippet()` (whitespace-collapse + 240 truncation + char-boundary) testsizdi.
- **Değişiklik (atomik):** indexer tests'e snippet testi (daraltma, kesme `...`, çok-baytlı panik-yok). +1 test.
- **Kapılar:** soul_check ✅ · `cargo test --locked` **72 passed (71→72), 0 fail** · regresyon yok.
- **Karar:** LAND. Rotasyon notu: G→...→G arası 4 döngü var, kural ihlali yok.

> **CHECKPOINT (oturum sınırı):** FAZ 1 + Döngü 1–6 tamam, hepsi `a1`'de, `a1-known-good`=c'.
> Kategoriler: G,H,B,K,I,G. Devam için: `git checkout a1` → `IDEAS.md`'den sıradakini seç
> (J: rusqlite+sqlite-vec, C: semantic-cache, F: bundle split, I: elapsed_ms UI gösterimi) →
> 10-adım döngü + kapılar. Süreklilik bu dosyada.

### 2026-06-23 — Döngü 5 [Kategori I: gözlemlenebilirlik]
- **Seçim:** `index_vault` indeksleme süresini raporlamıyordu; ayrıca frontend `IndexStats` tipi `pruned`'ı kaçırmıştı (Rust ile desenkron).
- **Değişiklik (atomik):** `IndexStats.elapsed_ms` (geriye-uyumlu `#[serde(default)]`) + `Instant` ile ölçüm. Frontend tipine `pruned?`+`elapsed_ms?`. Serialize-sözleşmesi testi (snake_case alan adları). +1 test.
- **Kapılar:** soul_check ✅ · tsc ✅ · `cargo test --locked` **71 passed (70→71), 0 fail** · regresyon yok.
- **Karar:** LAND. (UI'da gösterimi follow-up — IDEAS [I].)

### 2026-06-23 — Döngü 4 [Kategori K: repo/yayın]
- **Seçim:** README'de gerçek (çalışan) CI status rozeti yoktu.
- **Değişiklik (atomik):** README'ye `actions/workflows/ci.yml/badge.svg` rozeti (gerçek Actions durumuna linkli). main'de CI yeşil → rozet yeşil/dürüst.
- **Kapılar:** soul_check ✅ · ci.yml mevcut · kod değişmedi.
- **Karar:** LAND.

### 2026-06-23 — Döngü 3 [Kategori B: kararlılık/sağlamlık]
- **Seçim:** `apikey::write_key` herhangi bir non-empty string'i kabul ediyordu → içinde boşluk/satır olan yanlış-yapıştırma sessizce kaydedilirdi.
- **Değişiklik (atomik):** saf `validate_key` (trim + tek-token kontrolü, disk'e dokunmaz → test-edilebilir; success path'i test etmek ~/.aura'yı ezerdi). `write_key` artık onu kullanıyor; UI hata mesajı gösterir. +3 test.
- **Kapılar:** soul_check ✅ · `cargo test --locked` **70 passed (67→70), 0 fail** · regresyon yok.
- **Karar:** LAND. Rotasyon: B kullanıldı.

### 2026-06-23 — Döngü 2 [Kategori H: kod sağlığı/doküman]
- **Seçim:** yeni docs (philosophy/simple/glossary) ve CONTRIBUTING/CHANGELOG README'den keşfedilemiyordu.
- **Değişiklik (atomik):** README "Deep dives" + yeni "Contributing" satırları → tüm dokümanlar + soul_check linklendi.
- **Kapılar:** soul_check ✅ · tüm link hedefleri mevcut · kod değişmedi (cargo/vitest etkilenmez).
- **Karar:** LAND. Rotasyon: sıradaki H olamaz.

### 2026-06-23 — Döngü 1 [Kategori G: test]
- **Seçim:** `.gitignore` indeksleme mantığı (`gitignore_names`/`is_ignored_path`/`is_ignored_dir`) testsizdi (IDEAS yüksek-öncelik, düşük risk).
- **Değişiklik (atomik):** `indexer.rs`'e `#[cfg(test)] mod tests` — denylist, extra-gitignore-set, parser (glob/`!`/path atlanır; `build/`→`build`, `/out`→`out`), dosya-yok durumu. **+4 test.**
- **Kapılar:** soul_check ✅ · `cargo test --locked` **67 passed (63→67), 0 fail** · regresyon yok.
- **Karar:** LAND. `a1-known-good` ilerletildi. Rotasyon: sıradaki G olamaz.

### 2026-06-23 — FAZ 1: baseline + otonom altyapı (branch `a1`)
- **Durum:** `a1` branch'i `main`'den (v0.2.0, CI yeşil) açıldı.
- **M0 baseline doğrulandı:** `cargo test --locked` → **63 test / 27 suite, 0 hata**. (npm build + vitest önceki oturumda yeşildi; bu döngüde tekrar doğrulanacak.)
- **M1 soul_check eklendi:** `scripts/soul_check.py` — anayasa maddeleri 2/3/4/7/8/9'u grep+dosya ile denetler. İlk koşuda `/Users/` needle'ı placeholder'ları (`example`, `<user>`) yanlış-yakaladı → kural SIKILAŞTIRILDI (placeholder allowlist + negatif lookahead). Şimdi ✅.
- **M2 süreklilik dosyaları:** bu dosya + `IDEAS.md` + `BENCHMARKS.md` oluşturuldu.
- **Karar:** LAND (tüm kapılar yeşil). Sıradaki: CI'ye soul_check, standart repo dosyaları, docs, sonra `a1-known-good` tag.

<!-- Yeni döngüleri buranın ALTINA, en yeni en üstte ekle. -->
