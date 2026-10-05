import type { HuntView, ShellKind } from "./api";
import "./shell-collection.css";
import { coinText } from "./coin";
import { t } from "./i18n";

export const SHELL_CARDS = [
  { id: "great", sprite: 0, name: "Sò điệp lớn", taxon: "Pecten maximus", fact: "Sò điệp lớn có nhiều mắt nhỏ dọc mép áo. Nó có thể khép vỏ hoặc bơi để phản ứng với chuyển động và bóng tối.", habitat: "Sống ở biển, trên nền cát hoặc sỏi.", source: "https://www.marlin.ac.uk/species/detail/1398" },
  { id: "queen", sprite: 1, name: "Sò điệp queen", taxon: "Aequipecten opercularis", fact: "Cá thể non có thể bám nền bằng sợi tơ chân. Khi lớn, chúng có thể rời điểm bám và bơi tự do.", habitat: "Gặp trên nền cát hoặc sỏi, tới độ sâu khoảng 100 m.", source: "https://www.marlin.ac.uk/species/detail/1997/1000" },
  { id: "variegated", sprite: 2, name: "Sò điệp đa sắc", taxon: "Mimachlamys varia", fact: "Vỏ loài này có nhiều màu tự nhiên, từ trắng, hồng tới tím. Nó có thể sống tự do hoặc bám nền bằng sợi tơ chân.", habitat: "Thường gặp trên nền đá, kể cả trong phần bám của tảo.", source: "https://www.marlin.ac.uk/species/detail/2086" },
] as const;
export function shellSprite(kind: ShellKind): number { return SHELL_CARDS.find(card => card.id === kind)?.sprite ?? 0; }

function node(tag: string, text = "", className = ""): HTMLElement {
  const el = document.createElement(tag); el.append(...coinText(text)); el.className = className; return el;
}
export function collectionPanel(view: HuntView): HTMLElement {
  const panel = node("section", "", "shell-collection");
  const unlocked = SHELL_CARDS.filter(card => (view.collection[card.id]?.count ?? 0) > 0).length;
  panel.append(node("h2", t("Bộ sưu tập vỏ sò · {n}/3", { n: unlocked })));
  panel.append(node("p", t("{n} ngọc trai · Nhặt loại sò lần đầu để mở thẻ tìm hiểu.", { n: view.pearls })));
  const grid = node("div", "", "shell-grid");
  for (const card of SHELL_CARDS) {
    const entry = view.collection[card.id];
    const article = node("article", "", `shell-card${entry ? "" : " locked"}`);
    const img = document.createElement("img"); img.src = `/art/hunt/shell_${card.sprite}.png`; img.alt = entry ? t("Minh họa stylized: {name}", { name: t(card.name) }) : t("Loại sò chưa khám phá");
    article.append(img, node("h3", entry ? t(card.name) : t("Chưa khám phá")));
    article.append(node("p", t("{color} · {coins} CBCoin khi kéo về thuyền", { color: t(["Trắng", "Đỏ", "Tím"][card.sprite]), coins: [1,10,100][card.sprite] }), "shell-meta"));
    if (entry) {
      article.append(node("i", card.taxon), node("p", t(card.fact)), node("p", t(card.habitat)), node("p", t("Đã nhặt {count} · Hiếm {rare} · Lần đầu {first}", { count: entry.count, rare: entry.rare_count, first: entry.first_found }), "shell-meta"));
      const source = document.createElement("a"); source.href = card.source; source.target = "_blank"; source.rel = "noopener noreferrer"; source.textContent = t("Nguồn: MarLIN / Marine Biological Association"); article.append(source);
    } else article.append(node("p", t("Kéo sò về thuyền để mở thẻ kiến thức.")));
    grid.append(article);
  }
  panel.append(grid, node("p", t("Ba thẻ tìm hiểu loài sò biển có thật. Ảnh AI là minh họa game, không dùng để định danh. Nhãn hiếm, tỉ lệ 8% và cơ hội có ngọc 35% trong sò hiếm là luật game, không phải số liệu sinh học. Ngọc hiện được lưu để sưu tầm, chưa có shop tiêu ngọc."), "shell-note"));
  return panel;
}
