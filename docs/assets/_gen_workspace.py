#!/usr/bin/env python3
"""AURA Desktop main-window mockup (rail + explorer + editor + Ask panel).
Personal-data-free; uses the app's real theme tokens and i18n strings."""

BG, BG2, BG3 = "#17171c", "#202027", "#2a2a33"
TEXT, MUTED, BORDER = "#ececf1", "#a7a7b4", "#343440"
ACCENT, ACC_SOFT = "#8f8cf5", "rgba(143,140,245,0.16)"
MD, CODE, CFG = "#8f8cf5", "#4ea1ff", "#3fcbb0"
GOOD = "#7ad17a"
o = []
def r(x, y, w, h, rx, fill, stroke=None, sw=1, op=None):
    s = f'<rect x="{x}" y="{y}" width="{w}" height="{h}" rx="{rx}" fill="{fill}"'
    if stroke: s += f' stroke="{stroke}" stroke-width="{sw}"'
    if op is not None: s += f' opacity="{op}"'
    o.append(s + '/>')
def t(x, y, s, fill=TEXT, size=13, w="400", anchor="start", ls=None, op=None):
    s = s.replace("&", "&amp;").replace("<", "&lt;").replace(">", "&gt;")
    a = f'<text x="{x}" y="{y}" fill="{fill}" font-size="{size}" font-weight="{w}" text-anchor="{anchor}"'
    if ls: a += f' letter-spacing="{ls}"'
    if op is not None: a += f' opacity="{op}"'
    o.append(a + f'>{s}</text>')
def line(x1, y1, x2, y2, stroke=BORDER, sw=1, op=None):
    s = f'<line x1="{x1}" y1="{y1}" x2="{x2}" y2="{y2}" stroke="{stroke}" stroke-width="{sw}"'
    if op is not None: s += f' opacity="{op}"'
    o.append(s + '/>')

W, H = 1200, 780
o.append(f'<svg xmlns="http://www.w3.org/2000/svg" width="{W}" height="{H}" viewBox="0 0 {W} {H}" '
         f'font-family="-apple-system,BlinkMacSystemFont,\'Segoe UI\',Roboto,sans-serif">')
o.append('<defs><linearGradient id="edBg" x1="0" y1="0" x2="0" y2="1">'
         f'<stop offset="0%" stop-color="#191920"/><stop offset="100%" stop-color="{BG}"/></linearGradient></defs>')
# window
r(0, 0, W, H, 14, BG)
o.append(f'<rect x="0.5" y="0.5" width="{W-1}" height="{H-1}" rx="14" fill="none" stroke="{BORDER}"/>')

# ---- title bar ----
r(0, 0, W, 38, 14, BG2)
r(0, 24, W, 14, 0, BG2)  # square off bottom of titlebar
for i, c in enumerate(["#ff5f57", "#febc2e", "#28c840"]):
    o.append(f'<circle cx="{20+i*18}" cy="19" r="6" fill="{c}"/>')
t(W/2, 23, "AURA Desktop", MUTED, 12.5, "600", "middle")
line(0, 38, W, 38, BORDER)

RAIL = 58
EXPL = 300
EDIT = 824
BODYT, BODYB = 38, 742

# ---- icon rail ----
r(0, BODYT, RAIL, BODYB-BODYT, 0, "#1b1b21")
line(RAIL, BODYT, RAIL, BODYB, BORDER)
cx = RAIL/2
# brand mark
o.append(f'<circle cx="{cx}" cy="66" r="14" fill="{ACCENT}"/>')
o.append(f'<circle cx="{cx}" cy="66" r="4.5" fill="#ffffff"/>')
o.append(f'<circle cx="{cx}" cy="66" r="9" fill="none" stroke="#ffffff" stroke-width="1.4" opacity="0.85"/>')
o.append(f'<circle cx="{cx+9}" cy="66" r="2.2" fill="#ffffff"/>')

def icon(kind, y, active=False):
    col = ACCENT if active else MUTED
    if active:
        r(8, y-16, RAIL-16, 32, 8, ACC_SOFT, ACCENT, 1)
    g = cx
    if kind == "workspace":
        for dx in (-5, 4):
            for dy in (-5, 4):
                r(g+dx-0.5, y+dy-0.5, 6, 6, 1.5, "none", col, 1.6)
    elif kind == "search":
        o.append(f'<circle cx="{g-2}" cy="{y-2}" r="6" fill="none" stroke="{col}" stroke-width="1.6"/>')
        line(g+2.5, y+2.5, g+7, y+7, col, 1.8)
    elif kind == "ask":
        r(g-8, y-7, 16, 12, 4, "none", col, 1.6)
        o.append(f'<path d="M {g-3} {y+5} L {g-6} {y+9} L {g+1} {y+5} Z" fill="{col}"/>')
    elif kind == "aura":
        o.append(f'<path d="M {g} {y-8} L {g+2} {y-2} L {g+8} {y} L {g+2} {y+2} L {g} {y+8} '
                 f'L {g-2} {y+2} L {g-8} {y} L {g-2} {y-2} Z" fill="none" stroke="{col}" stroke-width="1.5"/>')
    elif kind == "graph":
        pts = [(g-6, y+5), (g+6, y+4), (g, y-7)]
        line(pts[0][0], pts[0][1], pts[2][0], pts[2][1], col, 1.4)
        line(pts[1][0], pts[1][1], pts[2][0], pts[2][1], col, 1.4)
        line(pts[0][0], pts[0][1], pts[1][0], pts[1][1], col, 1.4)
        for px, py in pts:
            o.append(f'<circle cx="{px}" cy="{py}" r="3" fill="{col}"/>')
    elif kind == "agents":
        r(g-8, y-6, 16, 13, 4, "none", col, 1.6)
        o.append(f'<circle cx="{g-3}" cy="{y}" r="1.7" fill="{col}"/>')
        o.append(f'<circle cx="{g+3}" cy="{y}" r="1.7" fill="{col}"/>')
        line(g, y-6, g, y-9, col, 1.6)
        o.append(f'<circle cx="{g}" cy="{y-10}" r="1.6" fill="{col}"/>')
    elif kind == "settings":
        o.append(f'<circle cx="{g}" cy="{y}" r="4.5" fill="none" stroke="{col}" stroke-width="1.6"/>')
        import math
        for k in range(8):
            a = k*math.pi/4
            x1 = g+math.cos(a)*6; y1 = y+math.sin(a)*6
            x2 = g+math.cos(a)*8.5; y2 = y+math.sin(a)*8.5
            line(x1, y1, x2, y2, col, 1.6)

nav = [("workspace", True), ("search", False), ("ask", False), ("aura", False),
       ("graph", False), ("agents", False), ("settings", False)]
y0 = 128
for i, (k, act) in enumerate(nav):
    icon(k, y0 + i*52, act)
# EN toggle bottom
r(cx-15, BODYB-44, 30, 24, 7, BG3, BORDER, 1)
t(cx, BODYB-28, "EN", MUTED, 11, "700", "middle")

# ---- explorer ----
r(RAIL, BODYT, EXPL-RAIL, BODYB-BODYT, 0, "#1d1d24")
line(EXPL, BODYT, EXPL, BODYB, BORDER)
ex = RAIL + 18
t(ex, 70, "Project", TEXT, 15, "700")
r(ex, 84, EXPL-RAIL-36, 30, 8, "none", ACCENT, 1.2)
t((RAIL+EXPL)/2, 104, "Open Project Folder", ACCENT, 12, "600", "middle")

tree = [
    ("▾ notes", None, 0, False),
    ("index.md", MD, 1, True),
    ("Architecture.md", MD, 1, False),
    ("Roadmap.md", MD, 1, False),
    ("Research.md", MD, 1, False),
    ("▾ src", None, 0, False),
    ("lib.rs", CODE, 1, False),
    ("indexer.rs", CODE, 1, False),
    ("search.rs", CODE, 1, False),
    ("consensus.rs", CODE, 1, False),
    ("▾ config", None, 0, False),
    ("Cargo.toml", CFG, 1, False),
    ("tauri.conf.json", CFG, 1, False),
    ("README.md", MD, 0, False),
]
ty = 146
for name, dot, indent, active in tree:
    rowy = ty - 13
    if active:
        r(RAIL+8, rowy, EXPL-RAIL-16, 26, 6, ACC_SOFT)
    tx = ex + indent*16
    if dot:
        o.append(f'<circle cx="{tx+4}" cy="{ty-4}" r="3.5" fill="{dot}"/>')
        t(tx+14, ty, name, TEXT if active else MUTED, 12.5, "600" if active else "400")
    else:
        t(tx, ty, name, TEXT, 12.5, "700")
    ty += 30

# index-stats footer (mirrors VaultExplorer: files · chunks · elapsed)
line(RAIL + 14, BODYB - 40, EXPL - 14, BODYB - 40, BORDER, 1, 0.6)
t(ex, BODYB - 22, "37 files · 214 chunks · 812 ms", MUTED, 11, "500")

# ---- editor ----
r(EXPL, BODYT, EDIT-EXPL, BODYB-BODYT, 0, "url(#edBg)")
line(EDIT, BODYT, EDIT, BODYB, BORDER)
edx = EXPL + 34
t(edx, 78, "Architecture.md", TEXT, 20, "700")
t(edx, 100, "notes / Architecture.md", MUTED, 11.5, "400", op=0.8)
line(EXPL+24, 116, EDIT-24, 116, BORDER, 1, 0.6)

# markdown body
def para(x, y, w, op=0.5):
    r(x, y, w, 7, 3, MUTED, op=op)
ey = 150
t(edx, ey, "# AURA Desktop", TEXT, 17, "700"); ey += 30
para(edx, ey, 440); ey += 18; para(edx, ey, 480); ey += 18; para(edx, ey, 360); ey += 34
t(edx, ey, "## Hybrid retrieval", TEXT, 15, "700"); ey += 26
para(edx, ey, 470); ey += 18
# wikilink line
t(edx, ey+6, "Ranked with RRF over", MUTED, 12.5, "400")
r(edx+158, ey-7, 132, 20, 5, ACC_SOFT)
t(edx+164, ey+6, "[[Hybrid Search]]", ACCENT, 12.5, "600")
ey += 22; para(edx, ey, 300, 0.5); ey += 30
# bullets
for b in ["Exact-match cache → zero wrong answers", "FTS5 keyword + vector recall", "Per-job aura spawn, file→stdin"]:
    o.append(f'<circle cx="{edx+4}" cy="{ey-4}" r="2.5" fill="{ACCENT}"/>')
    t(edx+16, ey, b, MUTED, 12.5, "400"); ey += 24
ey += 8
# code block
r(edx, ey, 460, 78, 8, "#14141a", BORDER, 1)
t(edx+14, ey+24, "fn search(q: &str) -> Vec<Hit> {", CODE, 12, "500")
t(edx+28, ey+44, "let k = fts5(q).rrf(vector(q));", "#9aa0aa", 12, "400")
t(edx+14, ey+64, "}", CODE, 12, "500")

# ---- Ask panel ----
ASKX = EDIT
r(ASKX, BODYT, W-ASKX, BODYB-BODYT, 0, "#1b1b21")
ax = ASKX + 22
t(ax, 70, "Ask", TEXT, 15, "700")
t(ax, 90, "Your second brain", MUTED, 11.5, "400", op=0.8)
line(ASKX+16, 104, W-16, 104, BORDER, 1, 0.6)
# question bubble
r(ASKX+60, 124, W-ASKX-82, 46, 10, BG3)
t(ax+44, 144, "How does hybrid search", TEXT, 12.5, "400")
t(ax+44, 161, "rank my notes?", TEXT, 12.5, "400")
# answer header w/ lane badge
o.append(f'<circle cx="{ax+8}" cy="196" r="9" fill="{ACCENT}"/>')
t(ax+8, 200, "A", "#fff", 11, "700", "middle")
r(ax+26, 188, 52, 18, 6, "rgba(143,140,245,0.18)", ACCENT, 1)
t(ax+52, 200, "Deep", ACCENT, 11, "700", "middle")
r(ax+84, 188, 78, 18, 6, BG3)
t(ax+123, 200, "claude · sonnet", MUTED, 10, "500", "middle")
ay = 226
for wd in [330, 348, 360, 320, 352, 300, 340, 240]:
    r(ax, ay, wd, 7, 3, MUTED, op=0.42); ay += 17
ay += 12
t(ax, ay, "SOURCES", MUTED, 10, "700", ls=1.5); ay += 16
for s, c in [("Hybrid Search.md", MD), ("search.rs", CODE), ("Embeddings.md", MD)]:
    wpx = 12 + len(s)*6.6
    r(ax, ay-12, wpx, 22, 7, BG3, BORDER, 1)
    o.append(f'<circle cx="{ax+11}" cy="{ay-1}" r="3" fill="{c}"/>')
    t(ax+20, ay+3, s, MUTED, 11, "500")
    ay += 30
# consensus toggle
ay += 6
t(ax, ay, "Consensus", TEXT, 12, "600")
r(W-22-30, ay-11, 30, 16, 8, BG3, BORDER, 1)
o.append(f'<circle cx="{W-22-22}" cy="{ay-3}" r="6" fill="{MUTED}"/>')
t(ax, ay+16, "all 3 AIs answer → Claude synthesizes", MUTED, 9.5, "400", op=0.8)
# input
r(ASKX+16, BODYB-52, W-ASKX-32, 36, 9, BG, BORDER, 1)
t(ax, BODYB-30, "Ask a question about your notes…", "#6b7280", 11.5, "400")
r(W-72, BODYB-46, 48, 24, 7, ACCENT)
t(W-48, BODYB-30, "Send", "#fff", 11.5, "600", "middle")

# ---- status bar ----
r(0, BODYB, W, H-BODYB, 0, BG2)
r(0, BODYB, W, 14, 0, BG2)
line(0, BODYB, W, BODYB, BORDER)
sy = (BODYB + H)/2 + 4
sx = 20
for name in ["claude", "antigravity", "codex"]:
    o.append(f'<circle cx="{sx+4}" cy="{sy-4}" r="4" fill="{GOOD}"/>')
    t(sx+14, sy, name, MUTED, 11.5, "500"); sx += 26 + len(name)*7
t(W-20, sy, "Project · 37 files", MUTED, 11.5, "500", "end")

o.append('</svg>')
open("docs/assets/workspace.svg", "w").write("\n".join(o))
print("wrote docs/assets/workspace.svg")
