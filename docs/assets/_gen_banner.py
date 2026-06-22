#!/usr/bin/env python3
"""AURA Desktop README hero banner. Personal-data-free, deterministic."""
import math, random
random.seed(11)
W, H = 1280, 400
ACCENT, BLUE, TEAL, AMBER = "#8f8cf5", "#4ea1ff", "#3fcbb0", "#e8bf69"
o = []
o.append(f'<svg xmlns="http://www.w3.org/2000/svg" width="{W}" height="{H}" viewBox="0 0 {W} {H}" '
         f'font-family="-apple-system,BlinkMacSystemFont,\'Segoe UI\',Roboto,sans-serif">')
o.append('<defs>'
         '<radialGradient id="bg" cx="30%" cy="38%" r="85%">'
         '<stop offset="0%" stop-color="#26243300"/><stop offset="0%" stop-color="#262433"/>'
         '<stop offset="55%" stop-color="#1b1b24"/><stop offset="100%" stop-color="#141419"/>'
         '</radialGradient>'
         '<linearGradient id="wm" x1="0" y1="0" x2="1" y2="1">'
         '<stop offset="0%" stop-color="#b9b7fb"/><stop offset="50%" stop-color="#8f8cf5"/>'
         '<stop offset="100%" stop-color="#6f8cf2"/></linearGradient>'
         '<radialGradient id="halo" cx="50%" cy="50%" r="50%">'
         '<stop offset="0%" stop-color="#8f8cf5" stop-opacity="0.55"/>'
         '<stop offset="100%" stop-color="#8f8cf5" stop-opacity="0"/></radialGradient>'
         '<filter id="b2"><feGaussianBlur stdDeviation="2.2"/></filter>'
         '</defs>')
o.append(f'<rect width="{W}" height="{H}" rx="16" fill="url(#bg)"/>')
o.append(f'<rect x="0.5" y="0.5" width="{W-1}" height="{H-1}" rx="16" fill="none" stroke="#343440"/>')

# faint background constellation
N = 46
pts = []
for i in range(N):
    x = random.uniform(40, W-40); y = random.uniform(36, H-36)
    pts.append((x, y))
cols = [ACCENT, BLUE, TEAL, AMBER]
for i in range(N):
    for j in range(i+1, N):
        dx = pts[i][0]-pts[j][0]; dy = pts[i][1]-pts[j][1]
        d = math.hypot(dx, dy)
        if d < 132:
            op = max(0.0, 0.12*(1-d/132))
            o.append(f'<line x1="{pts[i][0]:.0f}" y1="{pts[i][1]:.0f}" x2="{pts[j][0]:.0f}" y2="{pts[j][1]:.0f}" '
                     f'stroke="#8f8cf5" stroke-width="1" opacity="{op:.3f}"/>')
for i, (x, y) in enumerate(pts):
    r = random.uniform(1.6, 4.2)
    c = cols[i % len(cols)]
    o.append(f'<circle cx="{x:.0f}" cy="{y:.0f}" r="{r:.1f}" fill="{c}" opacity="0.5"/>')

# emblem (orbit mark)
ex, ey = 150, 196
o.append(f'<circle cx="{ex}" cy="{ey}" r="74" fill="url(#halo)"/>')
o.append(f'<circle cx="{ex}" cy="{ey}" r="40" fill="none" stroke="{ACCENT}" stroke-width="3"/>')
o.append(f'<ellipse cx="{ex}" cy="{ey}" rx="40" ry="15" fill="none" stroke="{BLUE}" stroke-width="2.4" '
         f'opacity="0.85" transform="rotate(-28 {ex} {ey})"/>')
o.append(f'<ellipse cx="{ex}" cy="{ey}" rx="40" ry="15" fill="none" stroke="{TEAL}" stroke-width="2.4" '
         f'opacity="0.7" transform="rotate(32 {ex} {ey})"/>')
o.append(f'<circle cx="{ex}" cy="{ey}" r="11" fill="#ffffff"/>')
o.append(f'<circle cx="{ex}" cy="{ey}" r="11" fill="{ACCENT}" opacity="0.35"/>')
# orbiting dots
o.append(f'<circle cx="{ex+40*math.cos(math.radians(-28)):.0f}" cy="{ey-40*0.0:.0f}" r="4.5" fill="{BLUE}"/>')
o.append(f'<circle cx="{ex-35}" cy="{ey+22}" r="4" fill="{AMBER}"/>')
o.append(f'<circle cx="{ex+30}" cy="{ey-28}" r="3.5" fill="{TEAL}"/>')

# wordmark
o.append(f'<text x="252" y="222" fill="url(#wm)" font-size="118" font-weight="800" letter-spacing="2">AURA</text>')
o.append(f'<text x="258" y="262" fill="#a7a7b4" font-size="30" font-weight="600" letter-spacing="13">DESKTOP</text>')
# tagline
o.append(f'<text x="256" y="312" fill="#ececf1" font-size="21" font-weight="500">'
         f'An AI-native, local-first Markdown second brain for macOS.</text>')
o.append(f'<text x="256" y="342" fill="#a7a7b4" font-size="15.5" font-weight="400">'
         f'Claude · Antigravity · Codex — orchestrated by the <tspan fill="#8f8cf5" font-weight="600">aura</tspan> CLI. '
         f'Your notes never leave the device.</text>')

o.append('</svg>')
open("docs/assets/banner.svg", "w").write("\n".join(o))
print("wrote docs/assets/banner.svg")
