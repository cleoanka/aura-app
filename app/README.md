# AURA Desktop — `app/` (Tauri project)

This directory is the actual **Tauri v2** application: the React + TypeScript
frontend (`src/`) and the Rust backend crate (`src-tauri/`). For the product
overview, architecture and privacy model, see the [repo README](../README.md).

## Layout

| Path | What lives here |
|---|---|
| `src/` | React 19 + TypeScript frontend — `App.tsx`, `components/`, `hooks/`, `i18n/`, `lib/`, `styles/`. |
| `src-tauri/` | Rust backend crate: Tauri commands, indexer, hybrid search, embedder, `aura` process exec, SQLite (FTS5 + sqlite-vec) data layer. |
| `public/` | Static assets served by Vite. |
| `index.html` | Vite entry HTML. |
| `.env.example` | Optional dev/test env overrides (copy to `.env`). |

## Scripts

```bash
npm install            # install frontend deps

npm run tauri dev      # run the desktop app in dev (opens a window)
npm run tauri build    # release .app + .dmg → src-tauri/target/release/bundle/

npm run dev            # Vite frontend only (no native shell)
npm run build          # tsc typecheck + vite production build
npm test               # frontend unit tests (vitest)
```

Backend tests and lints run from `src-tauri/`:

```bash
cd src-tauri
cargo test --locked                          # 95 tests (aura/network tests are #[ignore])
cargo clippy --all-targets -- -D warnings    # lint gate
cargo fmt --check                            # format gate
```

All three gates are enforced in CI (`.github/workflows/ci.yml`).

## Requirements

macOS (Apple Silicon), Rust 1.93+, Node 24+, Xcode Command-Line Tools, and the
`aura` CLI on `PATH` (ships in [`../aura-cli/`](../aura-cli)). The cloud sub-CLIs
(Claude / Antigravity / Codex) are optional and can be installed from the in-app
Agent Manager — none is required for the local second-brain features.
