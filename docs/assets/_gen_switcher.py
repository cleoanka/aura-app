#!/usr/bin/env python3
"""Workspace (repo) semantiği görseli + animasyonlu GIF.

Statik kart: aktif workspace + son-kullanılanlar menüsü (unut ✕ dahil).
GIF: menü kapalı → açık → başka repo'ya geçiş → liste yeni repo'yla tazelenir.
Kişisel-veri içermez; app'in gerçek tema token'ları ve i18n metinleri kullanılır.

Üretim: python3 docs/assets/_gen_switcher.py  (repo kökünden)
PNG/GIF, rsvg-convert (librsvg) ile render edilir.
"""

import subprocess
import tempfile
from pathlib import Path

BG, BG2, BG3 = "#17171c", "#202027", "#2a2a33"
TEXT, MUTED, BORDER = "#ececf1", "#a7a7b4", "#343440"
ACCENT, ACC_SOFT = "#8f8cf5", "rgba(143,140,245,0.16)"
GOOD, RED = "#7ad17a", "#ef6b73"

OUT = Path("docs/assets")


class Scene:
    def __init__(self, w, h):
        self.w, self.h = w, h
        self.o = [
            f'<svg xmlns="http://www.w3.org/2000/svg" width="{w}" height="{h}" '
            f'viewBox="0 0 {w} {h}" '
            "font-family=\"-apple-system,BlinkMacSystemFont,'Segoe UI',Roboto,sans-serif\">"
        ]

    def r(self, x, y, w, h, rx, fill, stroke=None, sw=1, op=None):
        s = f'<rect x="{x}" y="{y}" width="{w}" height="{h}" rx="{rx}" fill="{fill}"'
        if stroke:
            s += f' stroke="{stroke}" stroke-width="{sw}"'
        if op is not None:
            s += f' opacity="{op}"'
        self.o.append(s + "/>")

    def t(self, x, y, s, fill=TEXT, size=13, w="400", anchor="start", op=None):
        s = s.replace("&", "&amp;").replace("<", "&lt;").replace(">", "&gt;")
        a = (
            f'<text x="{x}" y="{y}" fill="{fill}" font-size="{size}" '
            f'font-weight="{w}" text-anchor="{anchor}"'
        )
        if op is not None:
            a += f' opacity="{op}"'
        self.o.append(a + f">{s}</text>")

    def line(self, x1, y1, x2, y2, stroke=BORDER, sw=1):
        self.o.append(
            f'<line x1="{x1}" y1="{y1}" x2="{x2}" y2="{y2}" stroke="{stroke}" stroke-width="{sw}"/>'
        )

    def svg(self):
        return "\n".join(self.o + ["</svg>"])


REPOS = ["notes-vault", "trading-research", "teknofest-aura"]


def panel(active_repo, menu_open, notes, highlight=None, w=560, h=520):
    """VaultExplorer panelinin sol üst köşesi: başlık + switcher + dosya listesi."""
    s = Scene(w, h)
    s.r(0, 0, w, h, 14, BG)
    s.o.append(
        f'<rect x="0.5" y="0.5" width="{w-1}" height="{h-1}" rx="14" fill="none" stroke="{BORDER}"/>'
    )

    # panel header
    s.t(24, 40, "ÇALIŞMA ALANI", ACCENT, 11.5, "700")
    s.t(24, 68, active_repo, TEXT, 21, "700")
    s.r(w - 168, 40, 144, 34, 8, ACC_SOFT, ACCENT, 1)
    s.t(w - 96, 62, "Proje Klasörü Aç", TEXT, 12, "600", "middle")

    # switcher toggle
    ty = 92
    s.r(24, ty, w - 48, 34, 8, BG2, BORDER, 1)
    s.t(38, ty + 22, "Son çalışma alanları", MUTED, 12, "500")
    s.t(w - 40, ty + 22, "▾" if not menu_open else "▴", MUTED, 12, "500", "middle")

    list_y = ty + 46
    if menu_open:
        mh = len(REPOS) * 38 + 8
        s.r(24, list_y, w - 48, mh, 8, BG2, BORDER, 1)
        for i, name in enumerate(REPOS):
            iy = list_y + 6 + i * 38
            is_active = name == active_repo
            is_hl = name == highlight
            if is_active or is_hl:
                s.r(30, iy, w - 60, 32, 6, ACC_SOFT if is_active else BG3)
            s.t(44, iy + 21, name, TEXT if (is_active or is_hl) else MUTED, 13, "500")
            if is_active:
                s.r(w - 128, iy + 7, 52, 18, 9, "none", ACCENT, 1)
                s.t(w - 102, iy + 20, "aktif", ACCENT, 10, "600", "middle")
            s.t(w - 48, iy + 21, "✕", RED if is_hl else MUTED, 12, "500", "middle")
        list_y += mh + 12

    # note list
    s.t(24, list_y + 18, "DOSYALAR", MUTED, 10.5, "700")
    for i, (name, kind) in enumerate(notes):
        iy = list_y + 30 + i * 34
        if iy + 30 > h - 16:
            break
        s.r(24, iy, w - 48, 28, 6, BG2 if i == 0 else "none")
        dot = {"md": ACCENT, "code": "#4ea1ff", "cfg": "#3fcbb0"}[kind]
        s.o.append(f'<circle cx="38" cy="{iy+14}" r="3.5" fill="{dot}"/>')
        s.t(52, iy + 19, name, TEXT if i == 0 else MUTED, 12.5, "400")
    return s.svg()


NOTES_A = [
    ("Günlük notlar.md", "md"),
    ("Fikir defteri.md", "md"),
    ("Okuma listesi.md", "md"),
    ("Toplantı 2026-07.md", "md"),
    ("Arşiv taslağı.md", "md"),
]
NOTES_B = [
    ("strategy-notes.md", "md"),
    ("lob_features.py", "code"),
    ("backtest.rs", "code"),
    ("config.toml", "cfg"),
    ("REGIME.md", "md"),
]


def render_png(svg_text, out_path, zoom=2):
    with tempfile.NamedTemporaryFile("w", suffix=".svg", delete=False) as f:
        f.write(svg_text)
        tmp = f.name
    subprocess.run(
        ["rsvg-convert", "-z", str(zoom), "-o", str(out_path), tmp], check=True
    )
    Path(tmp).unlink()


def main():
    # Statik kart (README için): menü açık, aktif repo + unut ✕ görünür.
    static = panel("notes-vault", True, NOTES_A)
    (OUT / "workspace-switcher.svg").write_text(static)
    render_png(static, OUT / "workspace-switcher.png")
    print("wrote workspace-switcher.svg/png")

    # GIF: kapalı → açık → hedef vurgulu → yeni repo aktif (liste değişti) → menü kapalı
    frames_spec = [
        (panel("notes-vault", False, NOTES_A), 120),
        (panel("notes-vault", True, NOTES_A), 140),
        (panel("notes-vault", True, NOTES_A, highlight="trading-research"), 110),
        (panel("trading-research", True, NOTES_B), 130),
        (panel("trading-research", False, NOTES_B), 200),
    ]
    from PIL import Image

    frames = []
    durations = []
    for i, (svg_text, dur) in enumerate(frames_spec):
        p = OUT / f"_frame_{i}.png"
        render_png(svg_text, p, zoom=1.4)
        frames.append(Image.open(p).convert("P", palette=Image.ADAPTIVE))
        durations.append(dur * 10)  # csantiye → ms (yavaş, okunabilir tempo)
        p.unlink()
    frames[0].save(
        OUT / "workspace-switch.gif",
        save_all=True,
        append_images=frames[1:],
        duration=durations,
        loop=0,
    )
    print("wrote workspace-switch.gif")


if __name__ == "__main__":
    main()
