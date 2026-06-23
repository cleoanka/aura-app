# Changelog

Bu projenin tüm dikkate değer değişiklikleri burada belgelenir.
Format [Keep a Changelog](https://keepachangelog.com/), sürümleme [SemVer](https://semver.org/).

## [Unreleased] — branch `a1`
### Added
- `scripts/soul_check.py` — anayasa (gizlilik / "modele doğrudan konuşma yok" /
  shell-injection yok / ağır özellikler default-OFF / cache doğruluğu) otomatik denetimi; CI'ye bağlandı.
- Otonom geliştirme altyapısı: `DEV_JOURNAL.md`, `IDEAS.md`, `BENCHMARKS.md`, `RESEARCH/`.
- Standart repo dosyaları: `CONTRIBUTING.md`, `CITATION.cff`, bu `CHANGELOG.md`.
- `docs/philosophy.md`, `docs/simple.md` (sıfır-jargon), `docs/glossary.md`; README'den linklendi.
- README'de gerçek **CI status rozeti**.
- `IndexStats.elapsed_ms` (indeksleme süresi) — backend + VaultExplorer'da gösterim; `pruned` da gösteriliyor.
- Test kapsamı: indexer `.gitignore`/`snippet`/`IndexStats`, apikey `validate_key` → **cargo test 63 → 72**.
### Changed
- BYOK anahtar doğrulaması: app `validate_key` + CLI `aura key set` (tek-token; boşluk/satır içeren yanlış-yapıştırma reddedilir).
### Fixed
- gemini→agy yeniden adlandırmasından kalma 3 stale test (`settings_robust`/`consensus_degrade`/`pty_argv`) — tam paket yeşil.

## [0.2.0] — 2026-06-23
### Added
- **BYOK** — kendi Anthropic API anahtarınla çalışma (app + `aura key` CLI; `~/.aura/anthropic_api_key`, 0600; default OFF).
- `.gitignore`-duyarlı indeksleme (denylist + vault'un kendi `.gitignore`'u).
- `tests/cache_invalidation.rs` — cache'in dosya-hash'leriyle senkron geçersizleşmesini kanıtlayan regresyon testi.
### Fixed
- CI: `.dmg` paketleme headless runner'da Finder gerektirdiği için patlıyordu → CI artık `--bundles app`.
- gemini→agy yeniden adlandırmasından kalma 3 stale test (`settings_robust`, `consensus_degrade`, `pty_argv`) düzeltildi → tam paket yeşil (63 test).
### Security
- Release binary'leri `--remap-path-prefix` ile derlenir → gömülü kişisel yol/kullanıcı adı yok.

## [0.1.0] — 2026-06-23
### Added
- İlk public sürüm: Tauri v2 + Rust + React/TS ikinci-beyin; Agent Manager, hibrit arama, knowledge graph, Ask (cache→retrieve→lane→stream), aura-mode, consensus, Lane 0.
