// CBCoin is shown as the CB logo instead of the word wherever the UI has room.
export const COIN_SRC = "/art/cb-coin.png";
const WORD = "CBCoin";

export function coinImg(): HTMLImageElement {
  const img = document.createElement("img");
  img.className = "coin"; img.src = COIN_SRC; img.alt = WORD; img.title = WORD; img.draggable = false;
  return img;
}

/** Splits text on the word "CBCoin" and puts the logo in its place. */
export function coinText(text: string): (Node | string)[] {
  const parts = text.split(WORD);
  const out: (Node | string)[] = [];
  parts.forEach((part, i) => {
    if (i > 0) out.push(coinImg());
    if (part) out.push(part);
  });
  return out;
}
