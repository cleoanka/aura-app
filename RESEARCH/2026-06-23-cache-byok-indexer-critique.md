# Kritik & araştırma — cache · BYOK · indexer (2026-06-23, branch a1)

Eleştirel öz-denetim. Her bulgu: **gözlem → risk → öneri/deney**. Anayasayı ezen
hiçbir öneri uygulanmaz; uygulananlar kapılardan geçer.

## F1 — CLI↔app BYOK validation parite boşluğu  [A, düşük risk → Döngü 8'de uygula]
- **Gözlem:** App tarafı `apikey::validate_key` (Döngü 3) bozuk anahtarı (içinde boşluk/satır) reddeder; ama `aura-cli/aura` `cmd_key set` yalnız boş-değil kontrolü yapar.
- **Risk:** Terminal kullanıcısı `aura key set "Bearer sk-…"` ya da çok-satırlı paste yaparsa sessizce bozuk anahtar saklanır → claude auth hatası (sebebi belirsiz).
- **Öneri:** Aynı tek-token kuralını CLI'a ekle (parite). Shell ile test edilebilir.

## F2 — `vault_epoch` ölü (her zaman "0")
- **Gözlem:** `commands/ai.rs` cache_key'e `vault_epoch` katıyor ama meta'da hiç set edilmiyor → sabit.
- **Risk:** Yok (sabit, zararsız); yalnız okuyan için kafa karıştırıcı. Yorumla netleştirildi.
- **Analiz:** Doğruluk zaten İKİ katmanla sağlanıyor — (1) retrieval-fingerprint cache_key'de (yeni dosya retrieval'a girince MISS), (2) `cache_get_valid` dep content-hash. `tests/cache_invalidation.rs` kanıtlıyor. Global epoch bump'ı EKLEMEK hit-oranını gereksiz düşürürdü (tek not düzenlemesi tüm cache'i çöpler).
- **Öneri:** Şimdilik bırak (dokümante). İleride istenirse epoch'u **yalnız schema/model-ver değişiminde** bump et (global purge için), per-edit DEĞİL.

## F3 — Vektör arama O(N) brute-force  [J, yüksek risk]
- **Gözlem:** `db::vec_search` tüm `vec_chunks`'ı tarar (partial top-k ile bellek korumalı). <~50k chunk'ta sorun yok.
- **Risk:** Büyük vault'ta arama gecikmesi lineer büyür.
- **Deney:** rusqlite + sqlite-vec (gerçek ANN) flag arkasında A/B; `BENCHMARKS.md`'de gecikme önce→sonra ölç. Dep + FFI değişimi → kapılarla koru. (IDEAS [J])

## F4 — Cache fingerprint note_path+heading_path (content_hash değil)
- **Gözlem:** `retrieval_fingerprint` içerik-hash içermez; içerik değişimi `cache_get_valid` dep-hash'iyle yakalanır.
- **Sonuç:** Katmanlı tasarım doğru ve test-kanıtlı; değişiklik gerekmiyor. (Sağlam.)

## F5 — BYOK anahtarı düz-metin 0600 dosya
- **Gözlem:** `~/.aura/anthropic_api_key` (0600) — kardeş CLI'larla (codex auth.json, gemini oauth_creds.json) tutarlı; app+CLI'ı tek kaynakla birleştirir.
- **Risk:** Keychain "rest'te şifreli" daha güçlü olurdu; ama dosya app↔CLI paritesini sağlıyor (kabul edilen tradeoff, dokümante).
- **Öneri:** İsteğe bağlı gelecekte Keychain backend (yalnız app), dosyayı fallback bırak. Düşük öncelik.

## Sıradaki deneyler (IDEAS'a beslenir)
- [C] Semantic-yakınlık cache + eval fixture (anayasa: sıfır yanlış-cevap → threshold + eval şart).
- [D] RRF/graph-retrieval ağırlık taraması: graph sinyalinin cevap kalitesine katkısını ölç.
- [I] `elapsed_ms`'i UI'da göster (index toast/LiveActivity).
