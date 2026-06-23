# Katkı rehberi — AURA Desktop

Teşekkürler! AURA'nın **felsefesi pazarlığa kapalıdır**; PR'lar bunu korumak zorundadır.

## Kurulum
```bash
cd app
npm install
npm run tauri dev      # geliştirme penceresi
```
Gereksinim: macOS (Apple Silicon), Rust 1.93+, Node 24+, Xcode CLT.

## Göndermeden önce — yeşil olmalı
```bash
cd app/src-tauri && cargo test --locked      # Rust
cd app && npm run build && npm test           # tsc + vitest
python3 scripts/soul_check.py                 # ANAYASA denetimi
```

## Anayasa (ihlal eden PR reddedilir)
1. **Yerel-öncelik & gizlilik** — veri yalnızca kullanıcının bilerek gönderdiği prompt ile çıkar.
2. **Kişisel veri sızıntısı yok** — repo/commit/binary'de kullanıcı adı / ev yolu / token yok. Release binary'leri `RUSTFLAGS="--remap-path-prefix=$HOME=/build"` ile derlenir.
3. **App modele doğrudan konuşmaz** — tüm AI çağrıları `aura` CLI üzerinden.
4. **Shell-injection yok** — prompt+context `0600` dosya → stdin; asla shell dizgesine interpolate edilmez.
5. **`Fix` salt-okunur** — diff önizler, asla commit etmez.
6. **Ağır özellikler default KAPALI** — consensus / lane0 / BYOK / advanced retrieval / semantic search.

Detay: [`docs/philosophy.md`](docs/philosophy.md) · [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md).

## Stil
- Çevre koddaki yorum yoğunluğu/isimlendirme/idiyomu izle.
- Atomik commit'ler; bozuk commit'leme. Conventional-commit tarzı başlık tercih edilir (`feat:`, `fix:`, `docs:`, `ci:` …).
- Yeni davranış → test. Yeni metrik etkisi → `BENCHMARKS.md`.

## Lisans
Katkın [MIT](LICENSE) altında lisanslanır.
