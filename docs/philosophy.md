# Felsefe — AURA neden böyle?

AURA'nın değeri özelliklerinde değil, **inatla savunduğu birkaç ilkede**. Bunlar
ihlal-edilemez; bir değişiklik bir metriği iyileştirse bile bu ilkelerden birini
çiğnerse **geri alınır**. (`scripts/soul_check.py` bunları otomatik denetler.)

## 1. Yerel-öncelik (local-first)
Vault düz dosyalardır — senin diskinde, senin formatında. İndeksleme, embedding,
hibrit arama ve cevap-cache'i **cihazda** çalışır. Veri yalnızca senin bilerek
gönderdiğin prompt ile (giriş yaptığın bir agent'a) dışarı çıkar. Telemetri yok.

## 2. Gizlilik mutlaktır
Repo'da, commit geçmişinde, derlenen binary'de hiçbir kişisel iz olmaz. Release
binary'leri `--remap-path-prefix` ile derlenir ki derleyen kişinin ev yolu/kullanıcı
adı bile sızmasın. BYOK anahtarı `0600` dosyada, maskeli gösterilir, asla yüklenmez.

## 3. App modele doğrudan konuşmaz
Tüm zekâ, kanıtlanmış **`aura` CLI**'dan gelir; o "hangi agent + nasıl auth" tek
doğruluk kaynağıdır. Böylece kırılgan, yeniden-yazılan bir LLM entegrasyonu yoktur —
**Claude ana beyin**, agy/codex yardımcı. App, `aura`'yı `zsh -lc` + dosya→stdin ile sarar.

## 4. Güvenli varsayılanlar
- **Shell-injection yok:** prompt+context dosyaya yazılır, stdin'e verilir; kullanıcı metni komut dizgesine girmez.
- **`Fix` salt-okunur:** diff önizler, asla commit etmez.
- **Ağır özellikler kapalı doğar:** consensus, Lane 0, BYOK, advanced retrieval, semantic search — hepsi opt-in. Varsayılan yol her zaman çalışır.
- **Çökmezlik:** bozuk bir ayar varsayılana düşer, uygulama çökmez.

## 5. En ucuz yol kazanır
Bir soruyu cevaplayabilecek en ucuz yol seçilir: exact-cache → yerel (Lane 0) →
Fast → Deep → Consensus. Cache, dosya-hash'leriyle senkron tutulur → **sıfır yanlış-cevap**.

## 6. Her seviyede işe yarar
Hiç AI'ın olmasa bile (yerel graph + arama + editör), yerel AI'la, tek Claude'la
ya da full stack'le — aynı kurulum değer üretir.

> Özet: **yerel, özel, dürüst, güzel.** Küçük başla, köpek gibi büyüt, hiçbir şeyi kırma, ruhunu bozma.
