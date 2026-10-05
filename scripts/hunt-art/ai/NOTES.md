# AI art — shell hunt

Boat, claw and three shells in `public/art/hunt/` are AI-generated (built-in image generation) and only resized / re-canvased to the sizes below; they are real transparent RGBA PNGs. Prompts and source references: `hunt-art-prompts.json`. Automated checks (size, alpha, margins, SHA-256): `hunt-art-qa.json`.

| File | Size |
|---|---|
| boat.png | 720×540 |
| claw.png | 256×352 |
| shell_0.png, shell_1.png, shell_2.png | 512×512 |

Anchor constants used by `src/tank.ts`: boat rope outlet at (50 %, 83 %) of the boat canvas, claw grab point at 71 % of its height. The vector set in `../vector-fallback/` uses the same anchors and can be swapped back in by copying the PNGs.

Not hand-tested on real Windows yet: dropping/catching at different resolutions and DPI.
