# Araştırma — semantic-yakınlık cache (2026-06-23)

## Hedef
Exact-match cache yalnız birebir aynı (normalize) soruya hit verir. Anlamca aynı
("X nedir?" ↔ "X'i açıkla") sorular cache'i ıskalıyor. Semantic cache hit oranını
artırabilir — **ama anayasa Madde 9 (sıfır yanlış-cevap) pazarlığa kapalı.**

## Risk
Yanlış-pozitif (alakasız soruyu "yeterince benzer" sayıp eski cevabı sunmak) →
anayasa ihlali. Bu yüzden **eval fixture olmadan default'a alınmaz.** Zaten
`Settings.advanced_retrieval.semantic_cache_enabled` default `false` + `semantic_cache_threshold` (96) mevcut.

## Tasarım (güvenli, kapılı)
1. **Anahtar:** soru embedding'i (e5) → en yakın cache girdisi; cosine ≥ threshold (örn. 0.96) ise aday.
2. **Çift kapı:** aday bulunsa bile mevcut **dep-hash kontrolü** (cache_get_valid) aynen uygulanır → kaynak değişmişse yine miss.
3. **Sadece opt-in:** `semantic_cache_enabled` açıkken; kapalıyken birebir bugünkü davranış.
4. **Threshold ayarlanabilir** (mevcut alan); muhafazakâr yüksek başla.

## Eval fixture (uygulamadan ÖNCE şart)
`app/src-tauri/tests/fixtures/semantic_cache/` altında:
- `pairs.jsonl`: {q1, q2, same_answer: bool} — paraphrase (true) + tuzak/yakın-ama-farklı (false) çiftleri.
- Test: threshold'ta **false-positive = 0** (tuzak çiftler hit vermemeli), true-pair recall raporlanır.
- Kapı: false-positive > 0 → semantic cache **default kapalı kalır** ve LAND edilmez.

## Plan (sonraki döngüler)
- D(araştırma, bu) → G: eval fixture + harness (runtime LLM YOK; sadece embedding benzerliği deterministik) → C: semantic lookup'ı `ask` yoluna opt-in ekle, eval geçerse.
- **İlk somut adım:** embedding benzerliği saf fonksiyonu + fixture iskeleti (LLM'siz, test-edilebilir).

> Anayasa: eval'de tek bir false-positive bile varsa bu özellik default açılmaz.
