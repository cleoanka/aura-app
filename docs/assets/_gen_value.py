#!/usr/bin/env python3
"""Value-proposition visuals for AURA Desktop (tiers / token economy / aura mode).
Personal-data-free; uses the app's real theme palette."""

BG, BG2, BG3 = "#17171c", "#202027", "#2a2a33"
TEXT, MUTED, BORDER = "#ececf1", "#a7a7b4", "#343440"
VIOLET, BLUE, TEAL, AMBER, GRAY = "#8f8cf5", "#4ea1ff", "#3fcbb0", "#e8bf69", "#8b8f9a"
GREEN, RED = "#7ad17a", "#e0726a"
FONT = "-apple-system,BlinkMacSystemFont,'Segoe UI',Roboto,sans-serif"

def esc(s): return s.replace("&", "&amp;").replace("<", "&lt;").replace(">", "&gt;")

class S:
    def __init__(self, w, h):
        self.w, self.h, self.o = w, h, []
        self.o.append(f'<svg xmlns="http://www.w3.org/2000/svg" width="{w}" height="{h}" '
                      f'viewBox="0 0 {w} {h}" font-family="{FONT}">')
    def rect(self, x, y, w, h, rx, fill, stroke=None, sw=1, op=None):
        s = f'<rect x="{x}" y="{y}" width="{w}" height="{h}" rx="{rx}" fill="{fill}"'
        if stroke: s += f' stroke="{stroke}" stroke-width="{sw}"'
        if op is not None: s += f' opacity="{op}"'
        self.o.append(s + '/>')
    def text(self, x, y, s, fill=TEXT, size=13, w="400", anchor="start", ls=None, op=None):
        a = f'<text x="{x}" y="{y}" fill="{fill}" font-size="{size}" font-weight="{w}" text-anchor="{anchor}"'
        if ls: a += f' letter-spacing="{ls}"'
        if op is not None: a += f' opacity="{op}"'
        self.o.append(a + f'>{esc(s)}</text>')
    def line(self, x1, y1, x2, y2, stroke=BORDER, sw=1, op=None):
        s = f'<line x1="{x1}" y1="{y1}" x2="{x2}" y2="{y2}" stroke="{stroke}" stroke-width="{sw}"'
        if op is not None: s += f' opacity="{op}"'
        self.o.append(s + '/>')
    def circle(self, cx, cy, r, fill, stroke=None, sw=1, op=None):
        s = f'<circle cx="{cx}" cy="{cy}" r="{r}" fill="{fill}"'
        if stroke: s += f' stroke="{stroke}" stroke-width="{sw}"'
        if op is not None: s += f' opacity="{op}"'
        self.o.append(s + '/>')
    def chip(self, x, y, label, color):
        w = 13 + len(label) * 6.6
        self.rect(x, y, w, 20, 6, "color-mix(in srgb,%s 16%%,#17171c)" % color, color, 1)
        self.text(x + w/2, y + 14, label, color, 11, "600", "middle")
        return x + w + 8
    def save(self, path):
        self.o.append('</svg>')
        open(path, "w").write("\n".join(self.o))
        print("wrote", path)

def frame(s):
    s.rect(0, 0, s.w, s.h, 14, BG)
    s.o.append(f'<rect x="0.5" y="0.5" width="{s.w-1}" height="{s.h-1}" rx="14" fill="none" stroke="{BORDER}"/>')

def header(s, eyebrow, title, sub):
    s.text(40, 50, eyebrow, MUTED, 12, "700", ls=2)
    s.text(40, 84, title, TEXT, 26, "800")
    s.text(40, 110, sub, MUTED, 14, "400")

# ============================================================ 1) TIERS
def gen_tiers():
    s = S(1200, 690); frame(s)
    header(s, "WORKS FOR EVERYONE", "Useful at every level",
           "From zero AI to a full agent stack — AURA pays off either way. Each level builds on the one below.")
    tiers = [
        (BLUE,   "4", "Full stack", "Claude + Antigravity + Codex",
         ["Antigravity adds live research", "Codex implements changes", "Consensus → Claude synthesizes"],
         ["3 agents", "max power"]),
        (VIOLET, "3", "Just Claude", "One subscription, far more mileage",
         ["Cache: repeats cost 0 tokens", "Retrieval sends only relevant chunks", "Plan / review / fix — spend only when sure"],
         ["1 subscription", "huge token savings"]),
        (AMBER,  "2", "Local AI (Ollama)", "Lane 0 — generation on your machine",
         ["Ask your notes with a local model", "No account, no quota, no egress"],
         ["private", "offline", "free"]),
        (TEAL,   "1", "No AI at all", "A first-class Markdown second brain",
         ["Knowledge graph + hybrid search (keyword + on-device vectors)", "CodeMirror editor, [[wikilinks]], instant retrieval"],
         ["$0", "100% offline", "foundation"]),
    ]
    y = 150; rh = 122; gap = 10
    # "more power" arrow on the right
    s.o.append(f'<defs><linearGradient id="pw" x1="0" y1="1" x2="0" y2="0">'
               f'<stop offset="0%" stop-color="{TEAL}"/><stop offset="100%" stop-color="{BLUE}"/></linearGradient></defs>')
    s.rect(1158, y, 8, rh*4 + gap*3, 4, "url(#pw)", op=0.55)
    s.text(1152, y - 6, "▲ more power", MUTED, 10.5, "600", "end")
    for color, num, title, sub, bullets, chips in tiers:
        s.rect(40, y, 1100, rh, 12, BG2, BORDER, 1)
        s.rect(40, y, 6, rh, 12, color)              # left rail
        s.rect(46, y, 6, rh, 0, color)               # square the rail's right edge
        # number medallion
        s.circle(96, y + rh/2, 26, "color-mix(in srgb,%s 18%%,#202027)" % color, color, 1.5)
        s.text(96, y + rh/2 + 9, num, color, 26, "800", "middle")
        # title + sub
        s.text(146, y + 42, title, TEXT, 19, "800")
        s.text(146, y + 64, sub, MUTED, 13, "400")
        cx = 146
        for c in chips:
            cx = s.chip(cx, y + 80, c, color)
        # bullets (right)
        by = y + 34
        for b in bullets:
            s.circle(636, by - 4, 3, color)
            s.text(650, by, b, "#c8c8d2", 13, "400")
            by += 26
        y += rh + gap
    s.save("docs/assets/tiers.svg")

# ============================================================ 2) SAVINGS
def gen_savings():
    s = S(1200, 600); frame(s)
    header(s, "TOKEN ECONOMY", "Far more from the same Claude",
           "Answering the same questions over your vault — naïve chat vs. AURA's pipeline.")
    bx = 300; bw = 820
    # naive bar
    s.text(60, 168, "Naïve", TEXT, 14, "700")
    s.text(60, 186, "paste vault → chat", MUTED, 11, "400")
    s.rect(bx, 150, bw, 34, 8, "color-mix(in srgb,%s 22%%,#17171c)" % RED, RED, 1)
    s.rect(bx, 150, bw, 34, 8, RED, op=0.85)
    s.text(bx + bw - 14, 172, "100% tokens · every time", "#1a0c0a", 12.5, "700", "end")
    # aura bar
    aw = int(bw * 0.18)
    s.text(60, 236, "AURA", TEXT, 14, "700")
    s.text(60, 254, "cache + retrieval + lanes", MUTED, 11, "400")
    s.o.append(f'<defs><linearGradient id="sv" x1="0" y1="0" x2="1" y2="0">'
               f'<stop offset="0%" stop-color="{TEAL}"/><stop offset="100%" stop-color="{VIOLET}"/></linearGradient></defs>')
    s.rect(bx, 218, bw, 34, 8, BG2, BORDER, 1)        # ghost track
    s.rect(bx, 218, aw, 34, 8, "url(#sv)")
    s.text(bx + aw + 12, 240, "~18% — and repeats are free", TEAL, 12.5, "700")
    # mechanism cards
    s.line(40, 300, 1160, 300, BORDER, 1, 0.6)
    s.text(40, 336, "WHERE THE SAVINGS COME FROM", MUTED, 12, "700", ls=1.5)
    cards = [
        (TEAL,   "Exact-match cache", "A repeated question returns the stored answer — 0 tokens, instantly."),
        (VIOLET, "Hybrid retrieval", "Only the relevant chunks are sent as context — not your whole vault."),
        (BLUE,   "Lane routing", "Cheap model for simple asks; the strong one only when it's actually needed."),
        (AMBER,  "Plan-first", "Read-only plan before you spend on implementation — no wasted round-trips."),
    ]
    cw = 262; gap = 20; x = 40; y = 360; ch = 196
    for color, title, body in cards:
        s.rect(x, y, cw, ch, 12, BG2, BORDER, 1)
        s.rect(x, y, cw, 5, 12, color)
        s.rect(x, y + 2, cw, 4, 0, color)
        s.circle(x + 26, y + 44, 13, "color-mix(in srgb,%s 20%%,#202027)" % color, color, 1.4)
        s.circle(x + 26, y + 44, 4.5, color)
        s.text(x + 18, y + 86, title, TEXT, 15.5, "700")
        # wrap body
        words = body.split(); lines = []; cur = ""
        for wd in words:
            if len(cur) + len(wd) + 1 > 30:
                lines.append(cur); cur = wd
            else:
                cur = (cur + " " + wd).strip()
        if cur: lines.append(cur)
        ty = y + 112
        for ln in lines:
            s.text(x + 18, ty, ln, "#c2c2cd", 12.5, "400"); ty += 19
        x += cw + gap
    s.save("docs/assets/savings.svg")

# ============================================================ 3) MODES
def gen_modes():
    s = S(1200, 430); frame(s)
    header(s, "AURA MODE", "plan · review · fix · ship",
           "Plan is the safe default — you must opt in to change files, and it never commits.")
    cards = [
        (VIOLET, "Plan",   "Approach + steps for any task.", "read-only · safe default"),
        (BLUE,   "Review", "Claude critiques your git diff.", "no file pasting"),
        (TEAL,   "Fix",    "Make the change in one step.", "--dry previews · never commits"),
        (AMBER,  "Ship",   "Plan → implement → review.", "one command"),
    ]
    cw = 265; gap = 20; x = 40; y = 150; ch = 230
    for color, title, body, chip in cards:
        s.rect(x, y, cw, ch, 12, BG2, BORDER, 1)
        s.rect(x, y, cw, 5, 12, color)
        s.rect(x, y + 2, cw, 4, 0, color)
        # glyph medallion
        s.circle(x + cw/2, y + 64, 30, "color-mix(in srgb,%s 16%%,#202027)" % color, color, 1.5)
        s.text(x + cw/2, y + 73, title[0], color, 28, "800", "middle")
        s.text(x + cw/2, y + 132, title, TEXT, 19, "800", "middle")
        s.text(x + cw/2, y + 158, body, "#c2c2cd", 12.5, "400", "middle")
        # chip centered
        wch = 13 + len(chip) * 6.2
        s.rect(x + (cw - wch)/2, y + 180, wch, 22, 7, "color-mix(in srgb,%s 16%%,#17171c)" % color, color, 1)
        s.text(x + cw/2, y + 195, chip, color, 11, "600", "middle")
        x += cw + gap
    s.save("docs/assets/modes.svg")

gen_tiers(); gen_savings(); gen_modes()
