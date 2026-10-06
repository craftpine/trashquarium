// How a still fish sprite is bent each frame so it swims like a fish: a travelling body wave
// (small at the head, big at the tail), fins that flutter, a tail fin that fans open as it
// sweeps, and bursts of fast tail beats followed by glides. Pure functions: tank.ts draws.

export type SwimStyle = "fish" | "tuna" | "eel" | "ray";

const STYLE: Record<string, SwimStyle> = {
  // Rays fly with their wings instead of beating a tail.
  mobula_birostris: "ray", aetobatus_narinari: "ray", rhinoptera_bonasus: "ray",
  // Eels ripple the whole body.
  rhinomuraena_quaesita: "eel", gymnothorax_favagineus: "eel", chlamydoselachus_anguineus: "eel",
  // Fast open-ocean swimmers keep the body stiff and drive with the tail.
  thunnus_alalunga: "tuna", thunnus_albacares: "tuna", thunnus_thynnus: "tuna", katsuwonus_pelamis: "tuna",
  xiphias_gladius: "tuna", makaira_nigricans: "tuna", istiophorus_platypterus: "tuna", coryphaena_hippurus: "tuna",
  carcharodon_carcharias: "tuna", lamna_nasus: "tuna", prionace_glauca: "tuna", carcharhinus_longimanus: "tuna",
  galeocerdo_cuvier: "tuna", sphyrna_lewini: "tuna", cetorhinus_maximus: "tuna", megachasma_pelagios: "tuna",
  rhincodon_typus: "tuna", stegostoma_tigrinum: "tuna",
};

export const swimStyle = (speciesId: string): SwimStyle => STYLE[speciesId] ?? "fish";

/** Sideways (on screen: vertical) body offset, as a fraction of body length, at position
 * u along the body (0 = tail tip, 1 = snout). `beat` is the tail-beat phase in radians. */
export function bodyWave(style: SwimStyle, u: number, beat: number): number {
  const back = 1 - u;
  switch (style) {
    case "eel": // about one and a half waves along the whole body
      return 0.045 * (0.35 + 0.65 * back) * Math.sin(beat - back * 7);
    case "tuna": // stiff body, strong tail
      return 0.055 * Math.pow(back, 3) * Math.sin(beat - back * 2.2) - 0.006 * Math.pow(u, 3) * Math.sin(beat);
    case "ray": // the body stays level; the wings do the work (finStretch)
      return 0.012 * Math.sin(beat * 0.5 - back * 2);
    default: // most fish: wave grows toward the tail, the head yaws a little the other way
      return 0.055 * (0.06 + 0.94 * back * back) * Math.sin(beat - back * 3.6) - 0.012 * Math.pow(u, 3) * Math.sin(beat);
  }
}

/** Vertical stretch of the slice at u (1 = unchanged): pectoral/dorsal fins flutter in the
 * middle of the body and the tail fin fans open and closed with each beat. Rays flap. */
export function finStretch(style: SwimStyle, u: number, beat: number, fin: number): number {
  if (style === "ray") return 1 + 0.2 * Math.sin(beat * 0.5 - u * 1.4); // wings up and down
  const middle = Math.exp(-(((u - 0.58) / 0.17) ** 2));
  const tail = u < 0.22 ? 1 - u / 0.22 : 0;
  const flutter = style === "eel" ? 0.025 : 0.09;
  return 1 + flutter * middle * Math.sin(fin + u * 4) + 0.14 * tail * Math.sin(beat + 0.6);
}

/** Burst-and-glide: returns the new burst level (0..1). Bursts start at random or on a turn. */
export function nextBurst(burst: number, dt: number, roll: number, turned: boolean): number {
  if (turned || roll < dt * 0.18) return 1;
  return Math.max(0, burst - dt * 0.9);
}
