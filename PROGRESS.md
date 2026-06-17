# AURA Desktop — Build Progress

Otonom build (Opus 4.8 orkestratör + Codex implementer + Gemini verify). Ultraplan: `docs/ultraplan-FINAL.md`.

## Faz 0 — Platform ölüm-kalım smoke ✅ GO
- T0.0 ortam: Rust 1.93, Node 24.15, codesign, brew ✓; Tauri CLI yok (npm-local kullanılacak).
- T0.1 token: claude=**keychain**, gemini=dosya, codex=dosya.
- T0.2 local-hardened codesign: **PASS** (sadece `inherit`, unsigned-mem=0).
- T0.3 notary: ATLANDI (Apple ID yok) — bloker değil, T4.6'ya ertelendi.
- T0.4 quarantine-spawn: KISMİ PASS (bundle child aura çalıştı).
- T0.5 cold-start: ek-median **30ms** « 1.5s → **per-job spawn, daemon YOK**.
- T0.6 env-resolver: zsh -lc & -c ikisi de binary buluyor → nice-to-have.
- T0.7 **OPUS GO** ✅. Entitlement: NON-SANDBOXED Developer ID + hardened + inherit.
- Detay: `docs/faz0/bulgular.md`.

## Faz 1 — Agent Manager + sözleşmeler + vault (DEVAM EDİYOR)
- [ ] T1.1 Tauri v2 + plugin-shell + ACL iskeleti + Playwright
- [x] T1.2 aura `--prompt-file/--context/--json-events` + `doctor --json` ✓ (json-events start→chunk→done; doctor --json sözleşme GEÇTİ)
- [x] T1.3 doctor sözleşmesi contracts/ ✓ (Rust testi AM işinde)
- [x] T1.4 env_resolver + ErrorTaxonomy (Rust) ✓
- [x] T1.5 detect/install/doctor + Tauri commands + **React UI kartları** ✓ (npm build PASS); PTY login SIRADA
- [ ] T1.6 vault folder-picker + settings persist
- [ ] T1.7 gerçek .app re-smoke

## Faz 2 — Index + hibrit arama (bekliyor)
## Faz 3 — AI akışı + Lane 0 + Consensus (bekliyor)
## Faz 4 — Cila + paketleme + notarize (bekliyor)
