#!/usr/bin/env python3
"""AURA Desktop — Anayasa (soul) denetimi.

İhlal-edilemez maddeleri grep/dosya kontrolleriyle yakalar. CI ve her agentic
döngüde koşar. Bir ihlal → non-zero exit + hangi madde. Yanlış-pozitif çıkarsa
kuralı SIKILAŞTIR (gevşetme).

Kullanım:  python3 scripts/soul_check.py
"""
from __future__ import annotations

import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
RUST = "app/src-tauri/src"
TS = "app/src"
fails: list[str] = []


def read(rel: str) -> str:
    p = ROOT / rel
    try:
        return p.read_text(encoding="utf-8", errors="ignore")
    except OSError:
        return ""


def grep_tree(pattern: str, *subdirs: str, exts=(".rs", ".ts", ".tsx")) -> list[str]:
    rx = re.compile(pattern)
    hits = []
    for sub in subdirs:
        base = ROOT / sub
        if not base.exists():
            continue
        for p in base.rglob("*"):
            if p.suffix not in exts or not p.is_file():
                continue
            for i, line in enumerate(p.read_text(encoding="utf-8", errors="ignore").splitlines(), 1):
                if rx.search(line):
                    hits.append(f"{p.relative_to(ROOT)}:{i}: {line.strip()}")
    return hits


# ── Madde 2: kişisel veri sızıntısı yok (LICENSE'taki yazar adı hariç) ───────
# Needle'lar parçalı kurulur ki bu dosyanın kendisi yasak token içermesin.
needle_user = "hikmet" + "ulah"
needle_home = "/User" + "s/"
try:
    out = subprocess.run(
        ["git", "-C", str(ROOT), "grep", "-nI", "-e", needle_user, "-e", needle_home],
        capture_output=True, text=True,
    ).stdout
except Exception as e:  # pragma: no cover
    out = ""
    fails.append(f"Madde2: git grep çalıştırılamadı: {e}")
# Belgelenmiş placeholder yolları kişisel veri DEĞİL (example/<user>/runner/builder).
# Gerçek bir kullanıcı adı (örn. /Users/<gerçek-isim>) bu allowlist'e uymaz → yakalanır.
safe_placeholder = re.compile(r"/User" + r"s/(example|<user>|runner|builder)(?![\w-])")
leaks = [
    l for l in out.splitlines()
    if not l.startswith("LICENSE:")
    and "scripts/soul_check.py" not in l  # belt-and-suspenders
    and not safe_placeholder.search(l)
]
if leaks:
    fails.append("Madde2 (kişisel veri sızıntısı):\n  " + "\n  ".join(leaks[:20]))

# ── Madde 3: app modele DOĞRUDAN konuşmaz (aura CLI üzerinden) ───────────────
direct_api = grep_tree(r"api\.anthropic\.com|@anthropic-ai/sdk|anthropic_sdk|reqwest::.*anthropic", RUST, TS)
if direct_api:
    fails.append("Madde3 (doğrudan model API çağrısı):\n  " + "\n  ".join(direct_api))

# ── Madde 4: shell-injection yok (Rust'ta dinamik -c string'i yok) ──────────
shell_inject = grep_tree(r'"-c".*format!|format!\([^)]*"-c"|Command::new\("(ba)?sh"\)', RUST, exts=(".rs",))
if shell_inject:
    fails.append("Madde4 (shell -c'ye dinamik string emaresi):\n  " + "\n  ".join(shell_inject))

# ── Madde 7: bozuk ayar çökmez (load_from default'a düşer) ───────────────────
settings = read(f"{RUST}/settings.rs")
if "Settings::default()" not in settings:
    fails.append("Madde7: settings.rs Settings::default() fallback'i bulunamadı")

# ── Madde 8: ağır özellikler default KAPALI ─────────────────────────────────
def returns_false(fn: str) -> bool:
    m = re.search(fn + r"\s*\([^)]*\)\s*->\s*bool\s*\{\s*false\s*\}", settings)
    return bool(m)

for fn in ("default_consensus_enabled", "default_lane0_enabled"):
    if not returns_false(fn):
        fails.append(f"Madde8: {fn} false döndürmüyor")
if "api_key_enabled: false" not in settings:
    fails.append("Madde8: Settings::default() içinde api_key_enabled: false yok")
if not re.search(r"semantic_search:\s*false", settings):
    fails.append("Madde8: Settings::default() içinde semantic_search: false yok")

# ── Madde 9: cache doğruluğu — invalidation kodu + testi mevcut ──────────────
if "fn cache_get_valid" not in read(f"{RUST}/db/mod.rs"):
    fails.append("Madde9: db::cache_get_valid bulunamadı")
if not (ROOT / "app/src-tauri/tests/cache_invalidation.rs").exists():
    fails.append("Madde9: tests/cache_invalidation.rs regresyon testi yok")

# ── sonuç ───────────────────────────────────────────────────────────────────
if fails:
    print("❌ SOUL CHECK FAILED — anayasa ihlali:\n")
    print("\n\n".join(fails))
    sys.exit(1)
print("✅ soul_check: tüm anayasa maddeleri geçti")
sys.exit(0)
