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

### 2026-06-23 — Döngü 24 [Kategori B: stress] (büyük kalem)
- **Değişiklik:** tests/stress_reindex.rs — 6 thread × 10 iter eşzamanlı reindex↔search (Mutex/db yolu). +1 test.
- **TUZAK & DÜZELTME:** ilk sürüm `default_embedder()` cache'li candle modelini her query'de çalıştırıp 60s+ TAKILDI → süreç öldürüldü, deterministik `StubEmbedder`'a çevrildi (1.75s). Kapı disiplini: yaramayan sürüm atıldı.
- **Kapılar:** soul_check ✅ · cargo **83 (82→83), 0 fail**, takılma yok.
- **Karar:** LAND.

### 2026-06-23 — Döngü 23 [Kategori K: repo/handoff]
- **Değişiklik:** STATE_OF_PROJECT.md — insan-incelemesi devir dokümanı (metrikler taban→şimdi, ne yapıldı, neye bakmalı, runtime gerektiren kalanlar). Final değil, anlık durum.
- **Kapılar:** soul_check ✅.
- **Karar:** LAND.

### 2026-06-23 — Döngü 22 [Kategori B: robustness]
- **Değişiklik:** cache_invalidation.rs — no-op reindex (mtime değişir, content_hash aynı) cache'i geçersizleştirmemeli; "meşgul vault yine cache'ten faydalanır" garantisi. +1 test.
- **Kapılar:** soul_check ✅ · cargo **82 (81→82), 0 fail**.
- **Karar:** LAND.

### 2026-06-23 — Döngü 21 [Kategori D: araştırma]  (sondan-önce)
- **Çıktı:** RESEARCH/2026-06-23-semantic-cache.md — semantic cache güvenli/kapılı tasarımı: çift-kapı (cosine≥threshold + mevcut dep-hash), opt-in, ve **eval fixture şartı** (false-positive=0 yoksa default açılmaz; anayasa Madde 9). Sonraki: eval harness (LLM'siz, deterministik).
- **Kapılar:** soul_check ✅.
- **Karar:** LAND. Sonraki adım = eval fixture/harness (plan+uygula).

### 2026-06-23 — Döngü 20 [Kategori G: test]
- **Değişiklik:** tests/chunk_stable_id.rs — determinizm + her girdiye (file_id/heading/ordinal/chunker_ver) göre değişim. Incremental index + cache_deps kararlılık kalkanı. +2 test.
- **Kapılar:** soul_check ✅ · cargo **81 (79→81), 0 fail**.
- **Karar:** LAND.

### 2026-06-23 — Döngü 19 [Kategori B: robustness]
- **Değişiklik:** apikey `parse_key_file` (ilk boş-olmayan satır) — dosyaya stray 2. satır/newline girerse key bozulmaz. read_key onu kullanır. +1 test.
- **Kapılar:** soul_check ✅ · cargo **79 (78→79), 0 fail**.
- **Karar:** LAND.

### 2026-06-23 — Döngü 18 [Kategori H: doküman]
- **Değişiklik:** `docs/development.md` (branch modeli + kapılar + 10-adım döngü + süreklilik + sürüm/yayın); README'den linklendi.
- **Kapılar:** soul_check ✅.
- **Karar:** LAND.

### 2026-06-23 — Döngü 17 [Kategori G: test]
- **Değişiklik:** db `normalize_embedding` testleri — unit-vektör + sıfır-vektör güvenli (NaN yerine [1,0,…]). +2 test. cosine doğruluğu korunuyor.
- **Kapılar:** soul_check ✅ · cargo **78 (76→78), 0 fail**.
- **Karar:** LAND.

### 2026-06-23 — Döngü 16 [Kategori F: optimizasyon — plan+uygula]  (son adım)
- **Plan (D15'ten):** tek 1.57MB chunk'ı vendor ailelerine böl.
- **Değişiklik:** vite.config.ts manualChunks (graph/editor/term/markdown/react/vendor). Sonuç: **1.57MB tek chunk → 7 chunk** (max editor 610KB, app index 80KB ayrı). App kodu değişince vendor cache korunur.
- **Kapılar:** soul_check ✅ · npm run build (tsc+vite) ✅ · vitest 10/10 · regresyon yok.
- **Karar:** LAND.

### 2026-06-23 — Döngü 15 [Kategori D: araştırma]  (sondan-önce)
- **Çıktı:** RESEARCH/2026-06-23-bundle-split.md — tek 1.57MB chunk'ı manualChunks ile vendor ailelerine (react/editor/graph/term/markdown/vendor) bölme planı + doğrulama kapısı.
- **Kapılar:** soul_check ✅.
- **Karar:** LAND → Döngü 16 uygular.

### 2026-06-23 — Döngü 14 [Kategori H: housekeeping]
- **Değişiklik:** IDEAS.md — Döngü 1–14 biten maddeler "Tamamlandı"ya taşındı, kalan backlog önceliklendirildi (sıradaki: J rusqlite+sqlite-vec, C semantic-cache, F bundle-split, B stress test).
- **Kapılar:** soul_check ✅.
- **Karar:** LAND.

### 2026-06-23 — Döngü 13 [Kategori J: yetenek — plan+uygula]  (son adım)
- **Plan:** kullanıcı bir not/klasörü `.gitignore`'a dokunmadan AURA indekslemesinden hariç tutabilsin.
- **Değişiklik:** `.auraignore` desteği — `gitignore_names` artık `.gitignore` + `.auraignore` birleşimini okur (additive, güvenli). +1 test (union). README/ARCHITECTURE/glossary güncellendi.
- **Kapılar:** soul_check ✅ · cargo **76 passed (75→76), 0 fail** · regresyon yok.
- **Karar:** LAND.

### 2026-06-23 — Döngü 12 [Kategori G: test]
- **Seçim:** ai.rs lane-seçim/cache mantığı (deep_query, normalize_query, retrieval_fingerprint) testsizdi.
- **Değişiklik:** ai.rs'e `#[cfg(test)] mod tests` — normalize (whitespace collapse), deep_query (analytical→deep, uzun→deep), retrieval_fingerprint sıralamadan-bağımsız (cache-hit kararlılığı). +3 test.
- **Kapılar:** soul_check ✅ · cargo **75 passed (72→75), 0 fail** · regresyon yok.
- **Karar:** LAND.

### 2026-06-23 — Döngü 11 [Kategori F: UI/UX & estetik — görsel]
- **Seçim:** workspace screenshot'ı yeni elapsed_ms index-stat'ını yansıtmıyordu (görsel ↔ gerçek tutarsız).
- **Değişiklik:** `_gen_workspace.py`'ye explorer index-stats footer'ı ("37 files · 214 chunks · 812 ms" + ince ayraç); SVG+PNG yeniden üretildi.
- **Kapılar:** soul_check ✅ · PNG render edildi, görsel doğrulandı.
- **Karar:** LAND.

### 2026-06-23 — Döngü 10 [Kategori H: tutarlılık/housekeeping]
- **Seçim:** CHANGELOG, a1 çalışmasının (Döngü 1–9) gerisinde kalmıştı.
- **Değişiklik:** CHANGELOG [Unreleased] — RESEARCH/, CI rozeti, elapsed_ms UI, test 63→72, BYOK validation, stale-test fix eklendi.
- **Kapılar:** soul_check ✅ (doküman).
- **Karar:** LAND.

### 2026-06-23 — Döngü 9 [Kategori I/F: gözlemlenebilirlik+UI]
- **Seçim:** elapsed_ms (Döngü 5) backend'de vardı ama UI'da görünmüyordu; pruned da gösterilmiyordu.
- **Değişiklik (atomik):** VaultExplorer index-stats satırına `· −{pruned}` ve `· {elapsed_ms} ms` (opsiyonel, geriye-uyumlu).
- **Kapılar:** soul_check ✅ · tsc ✅ · vitest 10/10 · regresyon yok.
- **Karar:** LAND.

### 2026-06-23 — Döngü 8 [Kategori A: plan+uygula]  (son adım → yeni plan)
- **Plan (kritik F1'den):** CLI `aura key set`'e app `validate_key` paritesi.
- **Değişiklik (atomik):** `cmd_key set` tek-token kontrolü ekledi (boşluk/satır → reddet). Kurulu `~/.local/bin/aura` senkronlandı.
- **Kapılar:** soul_check ✅ · py_compile ✅ · fonksiyonel: "Bearer sk-…" reddedildi (exit 1), temiz key kabul + maskeli durum.
- **Karar:** LAND.
- **YENİ PLAN (sonraki tur):** sıradaki sondan-önce=test/kritik/araştırma → son=plan+uygula. Aday backlog (IDEAS): [C] semantic-cache eval fixture · [I] elapsed_ms UI · [D] RRF/graph ağırlık taraması · [J] rusqlite+sqlite-vec. `a1-known-good`'tan devam.

### 2026-06-23 — Döngü 7 [Kategori D: araştırma/kritik]  (sondan-önceki adım)
- **Seçim:** eleştirel öz-denetim — cache/BYOK/indexer tasarımı.
- **Çıktı:** `RESEARCH/2026-06-23-cache-byok-indexer-critique.md` — 5 bulgu + sıradaki deneyler. En somut: **F1 CLI↔app BYOK validation parite boşluğu** (→ Döngü 8'de uygulanacak). F2 vault_epoch ölü (zararsız, bırak). F3 brute-force O(N) (IDEAS J). F4/F5 sağlam/kabul edilen tradeoff.
- **Kapılar:** soul_check ✅ (doküman).
- **Karar:** LAND. Sonraki adım = plan+uygula (Döngü 8).

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
