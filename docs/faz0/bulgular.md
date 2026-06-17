# AURA Desktop — Faz 0 Bulguları

## T0.0 — Ortam preflight
```
rustc: rustc 1.93.0 (254b59607 2026-01-19) (Homebrew)
cargo: cargo 1.93.0 (Homebrew)
node:  v24.15.0  npm: 11.12.1
codesign: codesign: unrecognized option `--version'
brew: Homebrew 5.1.15
tauri-cli: cargo-tauri not found
tauri not found
YOK (kurulacak)
aura:   /Users/<user>/.local/bin/aura
claude: /Users/<user>/.local/bin/claude
gemini: /Users/<user>/.npm-global/bin/gemini
codex:  /Users/<user>/.local/bin/codex
```

## T0.1 — Token konumu tespiti
```
--- dosya tabanlı config dizinleri ---
bazıları yok

--- claude creds ---
-rw-r--r--@   1 <user>  staff     161 16 Haz 23:30 .last-update-result.json
-rw-------@   1 <user>  staff  130872 17 Haz 18:41 history.jsonl
-rw-r--r--@   1 <user>  staff      89 17 Haz 15:50 mcp-needs-auth-cache.json
-rw-r--r--@   1 <user>  staff     396 17 Haz 18:30 settings.json
-rw-r--r--@   1 <user>  staff     607 17 Haz 16:39 settings.local.json
--- gemini ---
total 56
drwxr-xr-x@  11 <user>  staff   352 17 Haz 18:07 .
drwxr-x---+ 151 <user>  staff  4832 17 Haz 18:41 ..
-rw-r--r--@   1 <user>  staff    57 16 Haz 22:40 google_accounts.json
drwxr-xr-x@   6 <user>  staff   192 16 Haz 23:24 history
-rw-r--r--@   1 <user>  staff    36 11 Mar 10:47 installation_id
-rw-------@   1 <user>  staff  1823 17 Haz 17:38 oauth_creds.json
-rw-r--r--@   1 <user>  staff   274 16 Haz 23:24 projects.json
-rw-r--r--@   1 <user>  staff   126 17 Haz 15:46 settings.json
-rw-r--r--@   1 <user>  staff   187 17 Haz 15:51 state.json
--- codex ---
-rw-r--r--@   1 <user>  staff      4227 17 Haz 16:41 .codex-global-state.json
-rw-r--r--@   1 <user>  staff      4227 17 Haz 16:41 .codex-global-state.json.bak
-rw-------@   1 <user>  staff      4501 17 Haz 16:07 auth.json
-rw-r--r--@   1 <user>  staff      1703 17 Haz 16:10 chrome-native-hosts-v2.json
-rw-r--r--@   1 <user>  staff     10266 17 Haz 16:08 claude-cowork-import-history.json
-rw-r--r--@   1 <user>  staff     17390 17 Haz 16:08 external_agent_session_imports.json
-rw-------@   1 <user>  staff     14995 17 Haz 16:38 history.jsonl
-rw-r--r--@   1 <user>  staff    147165 17 Haz 18:29 models_cache.json
-rw-r--r--@   1 <user>  staff      1293 17 Haz 16:10 session_index.jsonl
-rw-r--r--@   1 <user>  staff       102 17 Haz 16:11 version.json
```

## T0.1b — claude token konumu (kesin)
```
--- gizli creds dosyası? ---
~/.claude/.credentials.json YOK
--- keychain (claude/anthropic) ---
    "acct"<blob>="<user>"
    "svce"<blob>="Claude Code-credentials"
    0x00000007 <blob>="Claude Safe Storage"
    "acct"<blob>="Claude Key"
    "svce"<blob>="Claude Safe Storage"
```
SONUÇ: gemini=dosya(~/.gemini/oauth_creds.json), codex=dosya(~/.codex/auth.json), claude=keychain (dosya yok)

## T0.5 — cold-start (Python startup overhead, LLM'siz; daemon SADECE bunu kazandırır)
```
aura --version median=35ms p-max=40ms
zsh -lc true  median=5ms p-max=6ms
EK-MEDIAN (aura startup overhead) = 30ms  | eşik=1500ms
KARAR: daemon GEREKSİZ → per-job onaylandı
```

## T0.6 — environment resolver (zsh -lc vs zsh -c farkı)
```
login-shell (zsh -lc) PATH:
/Library/Frameworks/Python.framework/Versions/3.13/bin
/opt/local/bin
/opt/local/sbin
/Library/Frameworks/Python.framework/Versions/3.13/bin
/Library/Frameworks/Python.framework/Versions/3.14/bin
/usr/local/bin
...non-login (zsh -c) PATH:
/Users/<user>/.npm-global/bin
/Users/<user>/.local/bin
/Library/Frameworks/Python.framework/Versions/3.13/bin
/opt/local/bin
/opt/local/sbin
/Library/Frameworks/Python.framework/Versions/3.13/bin
login-shell which:
/Users/<user>/.local/bin/aura
/Users/<user>/.local/bin/claude
/Users/<user>/.npm-global/bin/gemini
/Users/<user>/.local/bin/codex
non-login which:
/Users/<user>/.local/bin/aura
/Users/<user>/.local/bin/claude
/Users/<user>/.npm-global/bin/gemini
/Users/<user>/.local/bin/codex
```

## T0.2 — local-hardened-smoke: PASS (codesign --verify valid, sadece inherit, unsigned-mem=0)
## T0.3 — notary-smoke: ATLANDI (Apple Developer ID kimliği yok; release paketlemede T4.6'da yapılacak). GO'yu engellemez.
## T0.4 — quarantine-spawn: KISMİ — bundle exec'ten 'aura' çalışıyor (çıktı: aura 0.4.0). Tam Gatekeeper-quarantine kanıtı notarization gerektirir (T0.3 atlandı) → paketlemeye ertelendi. NOT: app NON-SANDBOXED Developer ID olacağı için child kendi keychain(claude)/dosya(gemini,codex) auth'una erişir (claude/gemini/codex zaten kullanıcı shell'inden çalışıyor).

## T0.7 — FAZ 0 KARAR KAPISI
| Karar | Sonuç |
|---|---|
| token konumu | claude=keychain('Claude Code-credentials'), gemini=dosya(~/.gemini/oauth_creds.json), codex=dosya(~/.codex/auth.json) |
| local-hardened | PASS (codesign --verify valid) |
| notary-smoke | ATLANDI (Apple ID yok) — bloker değil |
| quarantine-spawn | KISMİ PASS (bundle child aura çalışıyor); tam kanıt notarization'a ertelendi |
| cold-start | ek-median **30ms** « 1.5s → **per-job spawn; daemon YOK** |
| env-resolver | zsh -lc & zsh -c ikisi de 4 binary'yi buluyor → snapshot nice-to-have, bloker değil (yine de Faz1'de eklenecek) |
| entitlement | **NON-SANDBOXED Developer ID + hardened runtime + inherit**; keychain-access-groups YOK, allow-unsigned-executable-memory YOK |

**OPUS KARARI: ✅ GO.** Mimari sağlam: per-job spawn (daemon yok), non-sandboxed+inherit, dosya/keychain auth child'da erişilebilir. Tek açık uç: gerçek notarize+Gatekeeper kanıtı kullanıcının Apple Developer ID'siyle T4.6'da yapılacak (dev build'leri lokal imzasız çalışır, bloker değil). **Faz 1 BAŞLAYABİLİR.**
