// Normalized PNG anchors. Game coordinates remain the capture-point coordinates.
export const BOAT_HATCH = { x: 0.5, y: 0.83 } as const;
export const CLAW_GRAB = { x: 0.5, y: 0.71 } as const;
export const CLAW_RING = { x: 0.5, y: 0.04 } as const;
export const REST_LENGTH = 0.07;

function image(file: string): HTMLImageElement {
  const result = new Image();
  result.src = `/art/hunt/${file}`;
  return result;
}
export const huntArt = {
  boat: image("boat.png"), claw: image("claw.png"),
  shells: [image("shell_0.png"), image("shell_1.png"), image("shell_2.png")],
};
export function ready(image: HTMLImageElement): boolean {
  return image.complete && image.naturalWidth > 0;
}
export function anchoredRect(x: number, y: number, width: number, height: number, anchor: { x: number; y: number }) {
  return { x: x - width * anchor.x, y: y - height * anchor.y, width, height };
}
