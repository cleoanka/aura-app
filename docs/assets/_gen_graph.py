#!/usr/bin/env python3
"""Generate a clean, personal-data-free Knowledge-Graph mockup SVG that mirrors
AURA Desktop's real GraphView (palette, control panel, legend). Deterministic."""
import math, random

random.seed(7)

# Real GraphView palette (app/src/components/Graph/GraphView.tsx)
TYPE_COLOR = {
    "markdown": "#8f8cf5",
    "code":     "#4ea1ff",
    "config":   "#3fcbb0",
    "binary":   "#8b8f9a",
    "external": "#e8bf69",
    "dangling": "#6b7280",
}
LINK_COLOR = {
    "wikilink": "rgba(143,140,245,0.40)",
    "import":   "rgba(78,161,255,0.34)",
    "include":  "rgba(63,203,176,0.34)",
    "mention":  "rgba(167,167,180,0.22)",
}

# A small, synthetic second-brain (no personal data).
nodes = [
    ("index",        "index.md",         "markdown"),
    ("architecture", "Architecture.md",  "markdown"),
    ("roadmap",      "Roadmap.md",       "markdown"),
    ("ideas",        "Ideas.md",         "markdown"),
    ("research",     "Research.md",      "markdown"),
    ("embeddings",   "Embeddings.md",    "markdown"),
    ("consensus_n",  "Consensus.md",     "markdown"),
    ("search_n",     "Hybrid Search.md", "markdown"),
    ("graph_n",      "Knowledge.md",     "markdown"),
    ("privacy",      "Privacy.md",       "markdown"),
    ("daily",        "Daily.md",         "markdown"),
    ("meeting",      "Meeting.md",       "markdown"),
    ("vault_n",      "Vault.md",         "markdown"),
    ("lib",          "lib.rs",           "code"),
    ("indexer",      "indexer.rs",       "code"),
    ("search_rs",    "search.rs",        "code"),
    ("consensus_rs", "consensus.rs",     "code"),
    ("exec",         "exec.rs",          "code"),
    ("db",           "db.rs",            "code"),
    ("ai",           "ai.rs",            "code"),
    ("app_tsx",      "App.tsx",          "code"),
    ("graphview",    "GraphView.tsx",    "code"),
    ("askpanel",     "AskPanel.tsx",     "code"),
    ("cargo",        "Cargo.toml",       "config"),
    ("pkg",          "package.json",     "config"),
    ("tauri",        "tauri.conf.json",  "config"),
    ("settings",     "settings.json",    "config"),
    ("logo",         "logo.png",         "binary"),
    ("model",        "model.safetensors","binary"),
    ("gh",           "github.com",       "external"),
    ("anthropic",    "anthropic.com",    "external"),
    ("todo",         "TODO",             "dangling"),
    ("someday",      "Someday",          "dangling"),
    ("backlog",      "Backlog",          "dangling"),
]

edges = [
    ("index","architecture","wikilink"),("index","roadmap","wikilink"),
    ("index","ideas","wikilink"),("index","research","wikilink"),
    ("index","privacy","wikilink"),("index","vault_n","wikilink"),
    ("architecture","embeddings","wikilink"),("architecture","consensus_n","wikilink"),
    ("architecture","search_n","wikilink"),("architecture","graph_n","wikilink"),
    ("research","embeddings","wikilink"),("research","ideas","wikilink"),
    ("ideas","someday","wikilink"),("roadmap","todo","wikilink"),
    ("roadmap","backlog","wikilink"),("daily","meeting","wikilink"),
    ("daily","index","wikilink"),("meeting","roadmap","wikilink"),
    ("vault_n","search_n","wikilink"),("consensus_n","anthropic","mention"),
    ("research","gh","mention"),("architecture","gh","mention"),
    ("lib","indexer","import"),("lib","search_rs","import"),("lib","exec","import"),
    ("lib","db","import"),("lib","ai","import"),("lib","consensus_rs","import"),
    ("indexer","db","import"),("search_rs","db","import"),("search_rs","indexer","import"),
    ("ai","exec","import"),("ai","search_rs","import"),("ai","consensus_rs","import"),
    ("consensus_rs","exec","import"),("app_tsx","graphview","import"),
    ("app_tsx","askpanel","import"),("graphview","app_tsx","import"),
    ("askpanel","ai","mention"),("graphview","graph_n","mention"),
    ("cargo","lib","include"),("pkg","app_tsx","include"),
    ("tauri","lib","include"),("settings","ai","mention"),
    ("embeddings","model","mention"),("index","logo","mention"),
    ("search_n","search_rs","mention"),("consensus_n","consensus_rs","mention"),
    ("architecture","lib","mention"),("privacy","exec","mention"),
    ("graph_n","graphview","mention"),
]

ids = [n[0] for n in nodes]
label = {n[0]: n[1] for n in nodes}
ntype = {n[0]: n[2] for n in nodes}

deg = {i: 0 for i in ids}
for a, b, _ in edges:
    deg[a] += 1; deg[b] += 1

# --- tiny force-directed layout (deterministic) -------------------------------
pos = {i: (random.uniform(-1, 1), random.uniform(-1, 1)) for i in ids}
W, H = 900.0, 640.0
area = W * H
k = math.sqrt(area / len(ids)) * 0.42
adj = {i: set() for i in ids}
for a, b, _ in edges:
    adj[a].add(b); adj[b].add(a)

for it in range(420):
    disp = {i: [0.0, 0.0] for i in ids}
    for i in range(len(ids)):
        for j in range(i + 1, len(ids)):
            a, b = ids[i], ids[j]
            dx = pos[a][0] - pos[b][0]; dy = pos[a][1] - pos[b][1]
            d = math.hypot(dx, dy) or 0.01
            f = (k * k) / d
            ux, uy = dx / d, dy / d
            disp[a][0] += ux * f; disp[a][1] += uy * f
            disp[b][0] -= ux * f; disp[b][1] -= uy * f
    for a, b, _ in edges:
        dx = pos[a][0] - pos[b][0]; dy = pos[a][1] - pos[b][1]
        d = math.hypot(dx, dy) or 0.01
        f = (d * d) / k
        ux, uy = dx / d, dy / d
        disp[a][0] -= ux * f; disp[a][1] -= uy * f
        disp[b][0] += ux * f; disp[b][1] += uy * f
    # gentle centering
    for i in ids:
        disp[i][0] -= pos[i][0] * 0.012
        disp[i][1] -= pos[i][1] * 0.012
    t = 0.10 * (1 - it / 420) + 0.012
    for i in ids:
        dx, dy = disp[i]
        d = math.hypot(dx, dy) or 0.01
        pos[i] = (pos[i][0] + (dx / d) * min(d, t * 60),
                  pos[i][1] + (dy / d) * min(d, t * 60))

xs = [p[0] for p in pos.values()]; ys = [p[1] for p in pos.values()]
minx, maxx = min(xs), max(xs); miny, maxy = min(ys), max(ys)
# stage box (leave room on the right for the floating controls panel)
SX0, SX1, SY0, SY1 = 60, 905, 150, 730
def mapx(x): return SX0 + (x - minx) / (maxx - minx) * (SX1 - SX0)
def mapy(y): return SY0 + (y - miny) / (maxy - miny) * (SY1 - SY0)
P = {i: (mapx(pos[i][0]), mapy(pos[i][1])) for i in ids}

def radius(i): return 3.2 + math.sqrt(deg[i]) * 3.0

# --- emit SVG -----------------------------------------------------------------
out = []
out.append('<svg xmlns="http://www.w3.org/2000/svg" width="1200" height="780" '
           'viewBox="0 0 1200 780" font-family="-apple-system,BlinkMacSystemFont,\'Segoe UI\',Roboto,sans-serif">')
out.append('<defs>'
           '<radialGradient id="bgGlow" cx="42%" cy="46%" r="62%">'
           '<stop offset="0%" stop-color="#23222e"/><stop offset="100%" stop-color="#17171c"/>'
           '</radialGradient>'
           '<filter id="soft" x="-60%" y="-60%" width="220%" height="220%">'
           '<feGaussianBlur stdDeviation="6"/></filter></defs>')
# window card
out.append('<rect x="0" y="0" width="1200" height="780" rx="14" fill="url(#bgGlow)"/>')
out.append('<rect x="0.5" y="0.5" width="1199" height="779" rx="14" fill="none" stroke="#343440"/>')

# header
out.append('<text x="40" y="52" fill="#a7a7b4" font-size="12" font-weight="700" letter-spacing="2">GRAPH</text>')
out.append('<text x="40" y="84" fill="#ececf1" font-size="26" font-weight="700">Knowledge Graph</text>')
# badge + refresh (right)
out.append('<rect x="852" y="48" width="190" height="30" rx="8" fill="#202027" stroke="#343440"/>')
out.append(f'<text x="947" y="68" fill="#a7a7b4" font-size="13" text-anchor="middle">{len(ids)} Nodes · {len(edges)} Links</text>')
out.append('<rect x="1054" y="48" width="106" height="30" rx="8" fill="#8f8cf5"/>')
out.append('<text x="1107" y="68" fill="#ffffff" font-size="13" font-weight="600" text-anchor="middle">Refresh</text>')

# edges
sel = "index"
neigh = adj[sel] | {sel}
for a, b, kind in edges:
    x1, y1 = P[a]; x2, y2 = P[b]
    col = LINK_COLOR.get(kind, "rgba(167,167,180,0.20)")
    w = 1.3 if kind == "wikilink" else 0.9
    if a in neigh and b in neigh:
        w += 0.6
    out.append(f'<line x1="{x1:.1f}" y1="{y1:.1f}" x2="{x2:.1f}" y2="{y2:.1f}" stroke="{col}" stroke-width="{w:.1f}"/>')

# nodes (glow for hubs first)
for i in ids:
    x, y = P[i]; r = radius(i); c = TYPE_COLOR[ntype[i]]
    if deg[i] >= 5 or i == sel:
        out.append(f'<circle cx="{x:.1f}" cy="{y:.1f}" r="{r*2.4:.1f}" fill="{c}" opacity="0.16" filter="url(#soft)"/>')
for i in ids:
    x, y = P[i]; r = radius(i); c = TYPE_COLOR[ntype[i]]
    out.append(f'<circle cx="{x:.1f}" cy="{y:.1f}" r="{r:.1f}" fill="{c}"/>')
# selected ring
sx, sy = P[sel]
out.append(f'<circle cx="{sx:.1f}" cy="{sy:.1f}" r="{radius(sel)+4:.1f}" fill="none" stroke="#e8bf69" stroke-width="2.2"/>')

# labels for hubs + selected
labeled = sorted(ids, key=lambda i: deg[i], reverse=True)[:11]
if sel not in labeled:
    labeled.append(sel)
for i in labeled:
    x, y = P[i]; r = radius(i)
    fill = "#ececf1" if (i == sel or deg[i] >= 6) else "rgba(236,236,241,0.82)"
    fw = "700" if i == sel else "500"
    out.append(f'<text x="{x:.1f}" y="{y + r + 13:.1f}" fill="{fill}" font-size="11.5" '
               f'font-weight="{fw}" text-anchor="middle">{label[i]}</text>')

# --- floating Controls panel (top-right) --------------------------------------
cx, cy, cw = 944, 150, 222
out.append(f'<rect x="{cx}" y="{cy}" width="{cw}" height="556" rx="9" fill="#23232b" stroke="#343440"/>')
out.append(f'<text x="{cx+14}" y="{cy+26}" fill="#ececf1" font-size="12" font-weight="700" letter-spacing="1">CONTROLS</text>')
out.append(f'<text x="{cx+cw-20}" y="{cy+26}" fill="#a7a7b4" font-size="12">▾</text>')
out.append(f'<line x1="{cx}" y1="{cy+40}" x2="{cx+cw}" y2="{cy+40}" stroke="#343440"/>')
yy = cy + 62
def lbl(txt, y):
    out.append(f'<text x="{cx+14}" y="{y}" fill="#a7a7b4" font-size="11.5" font-weight="600">{txt}</text>')
def seg(y, opts, active):
    n = len(opts); gap = 4; bw = (cw - 28 - gap*(n-1)) / n
    for idx, o in enumerate(opts):
        bx = cx + 14 + idx * (bw + gap)
        fill = "#8f8cf5" if idx == active else "#17171c"
        stroke = "#8f8cf5" if idx == active else "#343440"
        tcol = "#ffffff" if idx == active else "#a7a7b4"
        out.append(f'<rect x="{bx:.1f}" y="{y}" width="{bw:.1f}" height="26" rx="6" fill="{fill}" stroke="{stroke}"/>')
        out.append(f'<text x="{bx+bw/2:.1f}" y="{y+17}" fill="{tcol}" font-size="11" font-weight="600" text-anchor="middle">{o}</text>')
def slider(y, frac):
    tx = cx + 14; tw = cw - 28
    out.append(f'<rect x="{tx}" y="{y}" width="{tw}" height="4" rx="2" fill="#343440"/>')
    out.append(f'<rect x="{tx}" y="{y}" width="{tw*frac:.1f}" height="4" rx="2" fill="#8f8cf5"/>')
    out.append(f'<circle cx="{tx+tw*frac:.1f}" cy="{y+2}" r="6" fill="#8f8cf5"/>')

lbl("Search", yy); yy += 10
out.append(f'<rect x="{cx+14}" y="{yy}" width="{cw-28}" height="28" rx="6" fill="#17171c" stroke="#343440"/>')
out.append(f'<text x="{cx+24}" y="{yy+18}" fill="#6b7280" font-size="11.5">Filter nodes…</text>'); yy += 44
lbl("Color by", yy); yy += 10; seg(yy, ["Type", "Folder"], 0); yy += 40
lbl("Scope", yy); yy += 10; seg(yy, ["Global", "Local"], 0); yy += 42
lbl("Node size", yy); yy += 14; slider(yy, 0.45); yy += 26
lbl("Link distance", yy); yy += 14; slider(yy, 0.35); yy += 26
lbl("Repel force", yy); yy += 14; slider(yy, 0.30); yy += 26
# labels toggle
out.append(f'<text x="{cx+14}" y="{yy+4}" fill="#a7a7b4" font-size="11.5" font-weight="600">Labels</text>')
out.append(f'<rect x="{cx+cw-44}" y="{yy-8}" width="30" height="16" rx="8" fill="#8f8cf5"/>')
out.append(f'<circle cx="{cx+cw-22}" cy="{yy}" r="6" fill="#ffffff"/>'); yy += 22
out.append(f'<rect x="{cx+14}" y="{yy}" width="{cw-28}" height="28" rx="6" fill="#17171c" stroke="#343440"/>')
out.append(f'<text x="{cx+cw/2}" y="{yy+18}" fill="#ececf1" font-size="12" text-anchor="middle">Fit</text>'); yy += 42
out.append(f'<line x1="{cx+14}" y1="{yy-8}" x2="{cx+cw-14}" y2="{yy-8}" stroke="#343440"/>')
# legend
legend = [("Markdown", "markdown"), ("Code", "code"), ("Binary", "binary"), ("Dangling", "dangling")]
cnt = {"markdown":0,"code":0,"config":0,"binary":0,"external":0,"dangling":0}
for i in ids: cnt[ntype[i]] += 1
for name, key in legend:
    out.append(f'<circle cx="{cx+20}" cy="{yy}" r="5" fill="{TYPE_COLOR[key]}"/>')
    out.append(f'<text x="{cx+34}" y="{yy+4}" fill="#a7a7b4" font-size="11.5">{name}</text>')
    out.append(f'<text x="{cx+cw-16}" y="{yy+4}" fill="#ececf1" font-size="11" text-anchor="end" opacity="0.7">{cnt[key]}</text>')
    yy += 22

out.append('</svg>')
open("docs/assets/graph.svg", "w").write("\n".join(out))
print("wrote docs/assets/graph.svg  nodes=%d edges=%d" % (len(ids), len(edges)))
