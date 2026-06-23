# aura — CLI orchestrator

A small, single-file Python CLI that makes three installed AI agents act as **one tool**:

| Agent | Role |
|---|---|
| **claude** | the brain — plan / review / synthesize |
| **agy** (Antigravity) | the librarian — research |
| **codex** | the hands — implement |

This is the **engine that [AURA Desktop](../README.md) wraps**. The desktop app shells out to this
exact script (`zsh -lc` + `stdin`); it is also useful on its own from a terminal.

> **Mantra:** *plan is the safe default — you must type `--apply` (or use `fix`/`ship`) to change files.*
> `fix` & `ship` write files but **never commit**, and they work outside a git repo too.

---

## How it works

Child agents run through your **`zsh -lc` login shell**, so they inherit your real `PATH` and
auth. Prompts live in a **file** that is redirected to `stdin` (claude/codex) or read into agy's
`-p` argument via a double-quoted `"$(cat file)"` — **one argument, no shell injection**. Python
never interpolates your text into the command string.

It stores nothing personal in the repo: runtime state lives under your home dir (resolved at
runtime via `Path.home()`), never a hardcoded path — so the same script works for **everyone**.

---

## Install

```bash
# from the repo root
install -m 0755 aura-cli/aura ~/.local/bin/aura      # ensure ~/.local/bin is on your PATH
aura doctor                                           # check the three agents are healthy
```

**Requirements:** Python 3, macOS/Linux with a `zsh` login shell, and the agents you want to use
on your `PATH` (`claude`, `agy`, `codex`). Each agent authenticates itself (claude → Keychain;
agy/codex → their own credential files). `aura` never copies or stores your tokens.

---

## Usage

```bash
aura "add retry to upload()"     # plan the task (read-only) — the safe default
aura plan "..." --research       # approach + steps; --research adds an agy research pass
aura fix "..."  [--dry]          # codex makes the change in one step (--dry previews the patch)
aura ship "..."                  # plan → implement → review, in one command
aura review                      # claude reviews your current git diff
aura doctor                      # check the three agents are healthy
aura key set sk-ant-...          # BYOK: run on your own Anthropic API key
aura key status | aura key clear # show (masked) / remove the stored key
aura last  [--open] [--log]      # re-print the previous run
```

**Flags** (work in any position):

| Flag | Effect |
|---|---|
| `--dry` | `fix`/`ship`: preview the patch, write nothing |
| `--apply` | apply the patch from your last `--dry` preview |
| `--research` | add an `agy` research step to `plan` |
| `--deep` / `--fast` | force the strongest / lightest models |
| `--verbose` | show raw agent transcripts |

Simple prompts stay light; complex ones auto-escalate to stronger models.

The desktop app additionally uses the machine-readable surface (`--json-events`,
`--prompt-file`, `--context`, `doctor --json`) — see [`docs/ARCHITECTURE.md`](../docs/ARCHITECTURE.md).

---

## BYOK — bring your own API key

By default `aura` runs the `claude` child on your existing subscription / OAuth session. If you'd
rather pay per token with your own **Anthropic API key**:

```bash
aura key set sk-ant-...     # stored at ~/.aura/anthropic_api_key (chmod 600)
aura key status             # shows a masked preview (e.g. sk-…aB3d) — never the full key
aura key clear              # remove it
```

When a key is stored (or `ANTHROPIC_API_KEY` is already exported), `aura` exports it so the `claude`
child inherits it through the login shell. The same file is what **AURA Desktop** reads, so a single
key drives both. The value is never printed in full and never leaves your machine via `aura`.

---

## Note

`aura-cli/aura` is the current orchestrator (v0.5.0 — adds BYOK `aura key`). Earlier pinned
snapshots the app was built against live in [`../vendor/`](../vendor) (`aura-0.4.0.py`,
`aura-patched.py`).

## License

[MIT](../LICENSE) © 2026 Hikmetullah Çevik ([@cleoanka](https://github.com/cleoanka))
