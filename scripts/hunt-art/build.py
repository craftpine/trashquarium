#!/usr/bin/env python3
"""Draws the shell-hunt sprites (boat, claw, three shells) as SVG and renders
them to transparent PNGs in public/art/hunt.

    python3 scripts/hunt-art/build.py

Needs `pip install playwright` plus a Chromium (only used to rasterise SVG).
The SVG sources are written next to this script so they can be tweaked by hand.
"""
import math
import pathlib

HERE = pathlib.Path(__file__).resolve().parent
OUT = HERE.parent.parent / "public" / "art" / "hunt"

# ---------------------------------------------------------------- shells ---
SHELLS = {
    # name: (base, light, dark, outline, glow)
    "shell_0": ("#f4dfb6", "#fff6dc", "#d8b685", "#a9825a", "#ffe9b0"),  # cream scallop
    "shell_1": ("#f1b39b", "#ffdcca", "#d4806a", "#9f5645", "#ffc9a8"),  # coral scallop
    "shell_2": ("#d9d0ee", "#f8f3ff", "#a597c9", "#74659a", "#cdeaff"),  # pearl scallop
}


def pt(cx, cy, r, deg):
    a = math.radians(deg)
    return cx + r * math.sin(a), cy - r * math.cos(a)


def shell_svg(name, base, light, dark, outline, glow):
    cx, cy, R = 128, 206, 138
    ribs = 9
    span = 128.0  # total fan angle
    a0 = -span / 2
    step = span / ribs
    wedges, seams, edge = [], [], []
    start = pt(cx, cy, R, a0)
    edge.append(f"M{cx},{cy} L{start[0]:.1f},{start[1]:.1f}")
    for i in range(ribs):
        s, e, m = a0 + i * step, a0 + (i + 1) * step, a0 + (i + 0.5) * step
        ps, pe, pm = pt(cx, cy, R, s), pt(cx, cy, R, e), pt(cx, cy, R * 1.13, m)
        fill = "url(#ribA)" if i % 2 == 0 else "url(#ribB)"
        wedges.append(
            f'<path d="M{cx},{cy} L{ps[0]:.1f},{ps[1]:.1f} Q{pm[0]:.1f},{pm[1]:.1f} {pe[0]:.1f},{pe[1]:.1f} Z" fill="{fill}"/>'
        )
        seams.append(f'<path d="M{cx},{cy} L{pe[0]:.1f},{pe[1]:.1f}" />')
        edge.append(f"Q{pm[0]:.1f},{pm[1]:.1f} {pe[0]:.1f},{pe[1]:.1f}")
    edge.append("Z")
    fan = " ".join(edge)
    rings = "".join(
        f'<path d="M{pt(cx, cy, R * k, a0 + 4)[0]:.1f},{pt(cx, cy, R * k, a0 + 4)[1]:.1f} A{R * k:.1f},{R * k:.1f} 0 0 1 {pt(cx, cy, R * k, -a0 - 4)[0]:.1f},{pt(cx, cy, R * k, -a0 - 4)[1]:.1f}"/>'
        for k in (0.30, 0.48, 0.66, 0.82, 0.94)
    )
    return f'''<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 256 256" width="256" height="256">
<defs>
  <linearGradient id="ribA" x1="0" y1="1" x2="0" y2="0"><stop offset="0" stop-color="{dark}"/><stop offset=".55" stop-color="{base}"/><stop offset="1" stop-color="{light}"/></linearGradient>
  <linearGradient id="ribB" x1="0" y1="1" x2="0" y2="0"><stop offset="0" stop-color="{dark}"/><stop offset=".5" stop-color="{base}"/><stop offset="1" stop-color="{base}"/></linearGradient>
  <radialGradient id="gloss" cx=".42" cy=".28" r=".7"><stop offset="0" stop-color="#fff" stop-opacity=".75"/><stop offset=".45" stop-color="#fff" stop-opacity=".12"/><stop offset="1" stop-color="#000" stop-opacity=".18"/></radialGradient>
  <linearGradient id="hinge" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="{base}"/><stop offset="1" stop-color="{dark}"/></linearGradient>
  <clipPath id="fan"><path d="{fan}"/></clipPath>
  <filter id="halo" x="-30%" y="-30%" width="160%" height="160%"><feGaussianBlur stdDeviation="9"/></filter>
  <filter id="grain" x="0" y="0" width="100%" height="100%"><feTurbulence type="fractalNoise" baseFrequency=".85" numOctaves="2" seed="7" result="n"/><feColorMatrix in="n" type="matrix" values="0 0 0 0 .25  0 0 0 0 .15  0 0 0 0 .08  1.6 0 0 0 -.55"/></filter>
  <filter id="drop" x="-20%" y="-20%" width="140%" height="150%"><feDropShadow dx="0" dy="5" stdDeviation="4" flood-color="#04222a" flood-opacity=".5"/></filter>
</defs>
<path d="{fan}" fill="{glow}" opacity=".55" filter="url(#halo)"/>
<g filter="url(#drop)">
  <path d="M{cx - 40},{cy + 8} L{cx - 26},{cy - 22} L{cx + 26},{cy - 22} L{cx + 40},{cy + 8} Q{cx},{cy + 24} {cx - 40},{cy + 8} Z" fill="url(#hinge)" stroke="{outline}" stroke-width="3" stroke-linejoin="round"/>
  <g clip-path="url(#fan)">{''.join(wedges)}
    <g fill="none" stroke="{outline}" stroke-opacity=".28" stroke-width="2">{rings}</g>
    <g stroke="{outline}" stroke-opacity=".55" stroke-width="2.4" fill="none">{''.join(seams)}</g>
    <rect width="256" height="256" fill="url(#gloss)"/>
    <rect width="256" height="256" filter="url(#grain)" opacity=".28"/>
  </g>
  <path d="{fan}" fill="none" stroke="{outline}" stroke-width="4" stroke-linejoin="round"/>
</g>
<ellipse cx="102" cy="92" rx="20" ry="9" fill="#fff" opacity=".55" transform="rotate(-28 102 92)"/>
<circle cx="164" cy="120" r="3.5" fill="#fff" opacity=".7"/>
</svg>'''


# ------------------------------------------------------------------ boat ---
BOAT = '''<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 480 360" width="480" height="360">
<defs>
  <linearGradient id="hull" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#f2b46c"/><stop offset=".55" stop-color="#d98a4a"/><stop offset="1" stop-color="#a95f33"/></linearGradient>
  <linearGradient id="teal" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#3ba39b"/><stop offset="1" stop-color="#1f6f6e"/></linearGradient>
  <linearGradient id="sail" x1="0" y1="0" x2="1" y2="0"><stop offset="0" stop-color="#f6ecd2"/><stop offset=".6" stop-color="#fffaf0"/><stop offset="1" stop-color="#e6d8b6"/></linearGradient>
  <linearGradient id="mast" x1="0" y1="0" x2="1" y2="0"><stop offset="0" stop-color="#7b4c2b"/><stop offset=".5" stop-color="#b57a49"/><stop offset="1" stop-color="#6d4325"/></linearGradient>
  <linearGradient id="flag" x1="0" y1="0" x2="1" y2="0"><stop offset="0" stop-color="#f06a58"/><stop offset="1" stop-color="#ff9d73"/></linearGradient>
  <radialGradient id="glass" cx=".35" cy=".3" r=".8"><stop offset="0" stop-color="#d8f6f2"/><stop offset="1" stop-color="#4c9ea0"/></radialGradient>
  <radialGradient id="foam" cx=".5" cy=".5" r=".5"><stop offset="0" stop-color="#fff" stop-opacity=".75"/><stop offset=".6" stop-color="#eafcff" stop-opacity=".25"/><stop offset="1" stop-color="#eafcff" stop-opacity="0"/></radialGradient>
  <clipPath id="hullClip"><path id="hullShape" d="M38,168 C70,192 140,198 240,198 C340,198 410,192 446,158 C446,236 384,302 240,306 C96,302 38,236 38,168 Z"/></clipPath>
  <filter id="grain" x="0" y="0" width="100%" height="100%"><feTurbulence type="fractalNoise" baseFrequency=".7" numOctaves="2" seed="11" result="n"/><feColorMatrix in="n" type="matrix" values="0 0 0 0 .22  0 0 0 0 .12  0 0 0 0 .05  1.6 0 0 0 -.6"/></filter>
  <filter id="drop" x="-15%" y="-15%" width="130%" height="140%"><feDropShadow dx="0" dy="6" stdDeviation="5" flood-color="#04222a" flood-opacity=".45"/></filter>
</defs>
<ellipse cx="240" cy="304" rx="205" ry="20" fill="url(#foam)"/>
<g filter="url(#drop)">
  <!-- mast -->
  <path d="M236,204 L236,44 L246,44 L246,204 Z" fill="url(#mast)"/>
  <path d="M241,44 L241,204" stroke="#4a2c17" stroke-opacity=".25" stroke-width="1.5"/>
  <!-- main sail -->
  <path d="M232,52 C170,86 122,148 100,198 L232,198 Z" fill="url(#sail)" stroke="#cdb98d" stroke-width="2.5" stroke-linejoin="round"/>
  <path d="M232,52 C198,98 176,146 168,198 M232,52 C214,108 204,150 202,198" fill="none" stroke="#cdb98d" stroke-opacity=".55" stroke-width="1.6"/>
  <path d="M232,52 C176,86 128,146 106,194" fill="none" stroke="#fff" stroke-opacity=".7" stroke-width="3"/>
  <!-- jib -->
  <path d="M252,66 C300,104 340,152 358,198 L252,198 Z" fill="url(#sail)" stroke="#cdb98d" stroke-width="2.5" stroke-linejoin="round"/>
  <path d="M252,66 C282,110 296,152 298,198" fill="none" stroke="#cdb98d" stroke-opacity=".55" stroke-width="1.6"/>
  <!-- pennant -->
  <path d="M246,46 C266,36 288,50 312,40 C300,54 300,62 312,72 C290,64 268,76 246,64 Z" fill="url(#flag)" stroke="#c9503f" stroke-width="2" stroke-linejoin="round"/>
  <circle cx="241" cy="42" r="6" fill="#f3d27a" stroke="#a97e2b" stroke-width="2"/>
  <!-- hull -->
  <use href="#hullShape" fill="url(#hull)"/>
  <g clip-path="url(#hullClip)">
    <g fill="none" stroke="#7a4524" stroke-opacity=".34" stroke-width="2.4" stroke-linecap="round">
      <path d="M40,226 C120,246 360,246 446,224"/>
      <path d="M60,252 C130,272 350,272 424,250"/>
      <path d="M96,278 C170,294 310,294 384,278"/>
    </g>
    <path d="M30,196 C130,222 350,222 456,190 L456,212 C350,246 130,246 30,218 Z" fill="url(#teal)"/>
    <path d="M30,196 C130,222 350,222 456,190" fill="none" stroke="#a8e0d8" stroke-opacity=".55" stroke-width="2.5"/>
    <rect width="480" height="360" filter="url(#grain)" opacity=".30"/>
  </g>
  <use href="#hullShape" fill="none" stroke="#7a4524" stroke-width="4" stroke-linejoin="round"/>
  <!-- sheer (gunwale) strip -->
  <path d="M38,168 C70,192 140,198 240,198 C340,198 410,192 446,158" fill="none" stroke="#b79a62" stroke-width="16" stroke-linecap="round"/>
  <path d="M38,168 C70,192 140,198 240,198 C340,198 410,192 446,158" fill="none" stroke="#fff1cd" stroke-width="11" stroke-linecap="round"/>
  <!-- portholes -->
  <circle cx="170" cy="262" r="14" fill="url(#glass)" stroke="#8a5428" stroke-width="4"/>
  <circle cx="310" cy="262" r="14" fill="url(#glass)" stroke="#8a5428" stroke-width="4"/>
  <ellipse cx="165" cy="256" rx="5" ry="3" fill="#fff" opacity=".8"/>
  <ellipse cx="305" cy="256" rx="5" ry="3" fill="#fff" opacity=".8"/>
  <!-- winch hatch where the rope leaves the hull -->
  <circle cx="240" cy="299" r="9" fill="#5a3519" stroke="#3b2210" stroke-width="2.5"/>
  <circle cx="240" cy="299" r="3.5" fill="#d8c7a0"/>
</g>
</svg>'''


# ------------------------------------------------------------------ claw ---
CLAW = '''<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 128 176" width="128" height="176">
<defs>
  <linearGradient id="brass" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="#fff2bd"/><stop offset=".45" stop-color="#e3aa45"/><stop offset="1" stop-color="#9a6521"/></linearGradient>
  <linearGradient id="brassDark" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="#f0c766"/><stop offset="1" stop-color="#7d4f18"/></linearGradient>
  <filter id="drop" x="-30%" y="-20%" width="160%" height="150%"><feDropShadow dx="0" dy="4" stdDeviation="3" flood-color="#04222a" flood-opacity=".55"/></filter>
</defs>
<g filter="url(#drop)" fill="none" stroke-linecap="round" stroke-linejoin="round">
  <circle cx="64" cy="14" r="10" stroke="#6b4414" stroke-width="7"/>
  <circle cx="64" cy="14" r="10" stroke="url(#brass)" stroke-width="3.6"/>
  <path d="M64,24 L64,78" stroke="#6b4414" stroke-width="13"/>
  <path d="M64,24 L64,78" stroke="url(#brass)" stroke-width="8"/>
  <path d="M64,80 C46,84 30,100 28,124 C27,140 34,154 48,160" stroke="#6b4414" stroke-width="11"/>
  <path d="M64,80 C82,84 98,100 100,124 C101,140 94,154 80,160" stroke="#6b4414" stroke-width="11"/>
  <path d="M64,80 L64,138" stroke="#6b4414" stroke-width="11"/>
  <path d="M64,80 C46,84 30,100 28,124 C27,140 34,154 48,160" stroke="url(#brassDark)" stroke-width="6.5"/>
  <path d="M64,80 C82,84 98,100 100,124 C101,140 94,154 80,160" stroke="url(#brassDark)" stroke-width="6.5"/>
  <path d="M64,80 L64,138" stroke="url(#brass)" stroke-width="6.5"/>
  <path d="M60,30 L60,72 M36,104 C32,112 31,118 31,126 M92,104 C96,112 97,118 97,126" stroke="#fff" stroke-opacity=".75" stroke-width="2.2"/>
</g>
<circle cx="64" cy="82" r="9" fill="url(#brass)" stroke="#6b4414" stroke-width="3"/>
<circle cx="64" cy="82" r="3" fill="#6b4414" opacity=".55"/>
</svg>'''


def main():
    from playwright.sync_api import sync_playwright

    OUT.mkdir(parents=True, exist_ok=True)
    sources = {name: shell_svg(name, *colors) for name, colors in SHELLS.items()}
    sources["boat"] = BOAT
    sources["claw"] = CLAW
    with sync_playwright() as p:
        browser = p.chromium.launch()
        for name, svg in sources.items():
            (HERE / f"{name}.svg").write_text(svg, encoding="utf-8")
            w, h = (int(x) for x in __import__("re").search(r'width="(\d+)" height="(\d+)"', svg).groups())
            scale = 2 if name != "boat" else 1.5
            page = browser.new_page(viewport={"width": w, "height": h}, device_scale_factor=scale)
            page.set_content(f'<html><body style="margin:0;background:transparent">{svg}</body></html>')
            page.screenshot(path=str(OUT / f"{name}.png"), omit_background=True, clip={"x": 0, "y": 0, "width": w, "height": h})
            page.close()
            print("wrote", OUT / f"{name}.png")
        browser.close()


if __name__ == "__main__":
    main()
