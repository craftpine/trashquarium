import "./style.css";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { open } from "@tauri-apps/plugin-dialog";
import { api, asFailure, type BellyEntry, type SavingsBook, type FeedReport, type Fish, type Inspection, type Species, type StateView } from "./api";
import { attentionNote, categoryName, formatDate, formatSize, getLang, onLangChange, LANGS, locale, reason, setLang, speciesFact, speciesName, stageName, t, type Lang } from "./i18n";
import { collectionPanel } from "./shell-collection";
import { coinText } from "./coin";
import { formatCountdown } from "./egg-den";
import { BANKER, SHARK_IMG, progress, savingsInterest, splitCountdown, termPercent } from "./savings";

type Tab = "shop" | "feed" | "belly" | "tank" | "den" | "savings" | "settings" | "hunt";
type Child = Node | string | null | undefined | false;

let state: StateView;
let tab: Tab = "shop";
let preview: Inspection[] = [];
let feedReport: FeedReport | null = null;
let bellyEntries: BellyEntry[] = [];
let selectedFish = "";
let working = false;
let autostart: boolean | null = null;

const $ = (id: string) => document.getElementById(id)!;

/** Small DOM builder: `on*` props become listeners, the rest attributes/properties. */
function h<K extends keyof HTMLElementTagNameMap>(
  tag: K,
  props: Record<string, unknown> = {},
  ...children: Child[]
): HTMLElementTagNameMap[K] {
  const el = document.createElement(tag);
  for (const [key, value] of Object.entries(props)) {
    if (value === undefined || value === false) continue;
    if (key.startsWith("on") && typeof value === "function") {
      el.addEventListener(key.slice(2).toLowerCase(), value as EventListener);
    } else if (key === "class") {
      el.className = String(value);
    } else if (key in el && typeof value !== "string") {
      (el as unknown as Record<string, unknown>)[key] = value;
    } else {
      el.setAttribute(key, value === true ? "" : String(value));
    }
  }
  for (const c of children) {
    if (c === null || c === undefined || c === false) continue;
    if (typeof c === "string") el.append(...coinText(c)); else el.append(c);
  }
  return el;
}

// ---------- feedback ----------

let toastTimer = 0;
function toast(text: string, error = false) {
  const t = $("toast");
  t.replaceChildren(...coinText(text));
  t.className = `show${error ? " error" : ""}`;
  clearTimeout(toastTimer);
  toastTimer = window.setTimeout(() => (t.className = ""), error ? 7000 : 4500);
}

function fail(e: unknown) {
  const f = asFailure(e);
  toast(f.undetermined ? t("Chưa xác định — đang kiểm tra Bụng cá…") : reason(f.code), true);
}

interface Action {
  label: string;
  kind?: "primary" | "ghost";
  run?: () => unknown | Promise<unknown>;
}

function modal(body: Node, actions: Action[]) {
  const dialog = $("modal") as HTMLDialogElement;
  const buttons = actions.map((a) =>
    h("button", {
      class: a.kind ?? "",
      onclick: async (ev: Event) => {
        (ev.currentTarget as HTMLButtonElement).disabled = true;
        dialog.close();
        await a.run?.();
      },
    }, a.label),
  );
  dialog.replaceChildren(h("div", { class: "modal-body" }, body), h("div", { class: "modal-actions" }, ...buttons));
  dialog.showModal();
}

// ---------- data ----------

async function refresh() {
  try {
    state = await api.state();
  } catch (e) {
    fail(e);
    return;
  }
  if (!state.fish.some((f) => f.id === selectedFish)) selectedFish = state.fish[0]?.id ?? "";
  announceHatches();
  if (tab === "belly") bellyEntries = await api.bellyList().catch(() => []);
  render();
}

async function previewPaths(paths: string[]) {
  if (working || paths.length === 0) return;
  working = true;
  tab = "feed";
  feedReport = null;
  try {
    preview = await api.preview(paths);
    if (paths.length > state.max_preview_files) {
      toast(t("Chỉ xem trước {n} file đầu tiên mỗi lượt.", { n: state.max_preview_files }));
    }
  } catch (e) {
    fail(e);
  }
  working = false;
  render();
}

async function feed() {
  const items = preview.filter((p) => p.ok);
  if (working || items.length === 0 || !selectedFish) return;
  working = true;
  render();
  try {
    feedReport = await api.feed(items, selectedFish);
    preview = [];
    if (feedReport.files.some((f) => f.undetermined)) {
      toast(t("Có file chưa xác định — đang kiểm tra Bụng cá…"), true);
      await api.recover().catch(fail);
    }
  } catch (e) {
    fail(e);
  }
  working = false;
  await refresh();
}

// ---------- rendering ----------

function render() {
  renderStats();
  renderBanners();
  renderTabs();
  const panel = $("panel");
  const views: Record<Tab, () => Node> = {
    shop: renderShop,
    feed: renderFeed,
    belly: renderBelly,
    tank: renderTank,
    den: renderDen,
    savings: renderSavings,
    settings: renderSettings,
    hunt: renderHunt,
  };
  panel.replaceChildren(views[tab]());
}

function showGuide() {
  const item = (icon: string, title: string, body: string) =>
    h("div", { class: "guide-item" }, h("span", { class: "guide-icon", "aria-hidden": "true" }, icon), h("div", {}, h("h3", {}, title), h("p", {}, body)));
  modal(
    h("div", { class: "guide" },
      h("h2", {}, t("Cách chơi trong 60 giây")),
      h("p", { class: "lead" }, t("Bể cá trên desktop của bạn, nhưng thức ăn là… file rác.")),
      item("🗑️", t("Cho cá ăn rác (an toàn!)"), t("Thả file bạn không cần nữa vào game. File chui vào Bụng cá — không bị xóa, nhả ra lúc nào cũng được — còn cá thì nhận EXP. Lỡ cho ăn nhầm? Cá chỉ hơi tiếc thôi.")),
      item("🆙", t("Lớn nhanh như thổi"), t("10 EXP = 1 level, một file nhỏ đã đủ 2 level. Cứ 5 level cá no căng bụng và ngủ trưa 2 tiếng. Lv.100 là trưởng thành: hết lớn, bắt đầu nghĩ đến chuyện đời.")),
      item("🐚", t("Gọi thuyền, gắp sò, ra tiền"), t("Gọi thuyền, canh cái móc đung đưa rồi bấm Space. Sò trắng 1 CBCoin, đỏ 10, tím 100 — sò tím là trúng số.")),
      item("🛒", t("Shopping cho bể"), t("Cá rẻ thì bé xíu, cá đắt thì to bự: cá mập voi (300 CBCoin) to gần gấp 4 lần cá bảy màu (20 CBCoin). Bể có 20 chỗ: cá dưới 100 CBCoin chiếm 1 chỗ, 100–199 chiếm 2, từ 200 trở lên chiếm 3 — cá mập to thì phải nhường chỗ chứ!")),
      item("⛵", t("Bán hay cho đẻ?"), t("Cá Lv.100 béo múp hiện chữ “bắt điii!” — gọi thuyền gắp lên rồi chọn bán luôn (giá mua ×100) hay nuôi thêm (lỡ mua nhầm thì cá chưa lớn bán lại được 1/2 giá mua). Hoặc ghép hai cá cùng loài cho sinh sản: mỗi trứng trừ giá bán của cả hai rồi vào Hang trứng ấp 2–3 tiếng, tỷ lệ nở 5–20% (cá càng đắt càng khó nở). Cá con ra đời là Lv.0 và lại bắt đầu từ đầu.")),
      item("🦈", t("Gửi tiết kiệm"), t("Dư CBCoin thì gửi Cá Mập: lãi 9% mỗi ngày, cộng dồn theo kỳ hạn 1–30 ngày (7 ngày +63%). Đáo hạn mới có lãi, rút sớm chỉ nhận lại gốc.")),
      item("😌", t("Yên tâm"), t("Không tiền thật, không tài khoản, không mạng. Cá không bao giờ chết, chỉ đôi khi hơi lười.")),
    ),
    [{ label: t("Đã hiểu!"), kind: "primary" }],
  );
}

function renderStats() {
  $("stats").replaceChildren(
    h("span", { class: "pill shells", title: t("CBCoin là điểm trong game, không phải tiền thật") }, h("b", {}, String(state.cbcoins)), "CBCoin"),
    h("span", { class: "pill", title: t("Cá dưới 100 CBCoin chiếm 1 chỗ, 100–199 chiếm 2 chỗ, từ 200 trở lên chiếm 3 chỗ.") }, h("b", {}, `${state.used_slots}/${state.capacity}`), t("chỗ · {n} cá", { n: state.fish.length })),
    h("span", { class: "pill" }, t("EXP hôm nay"), h("b", {}, String(state.daily.exp))),
    h("label", { class: "pill lang", title: t("Ngôn ngữ") },
      "🌐",
      h("select", { "aria-label": t("Ngôn ngữ"), onchange: (e: Event) => setLang((e.currentTarget as HTMLSelectElement).value as Lang) },
        ...LANGS.map((l) => h("option", { value: l.id, selected: getLang() === l.id }, l.label)))),
    h("button", { class: "pill help", title: t("Cách chơi"), "aria-label": t("Cách chơi"), onclick: () => showIntro() }, "?"),
  );
}

function renderBanners() {
  const banners: Node[] = [];
  if (state.read_only) {
    banners.push(h("div", { class: "banner danger" }, h("span", {}, reason(state.read_only.code), t(" Thư mục dữ liệu: "), h("code", {}, state.data_dir))));
  }
  if (state.belly_error) {
    banners.push(
      h("div", { class: "banner danger" },
        h("span", {}, t("Bụng cá tạm khóa: "), reason(state.belly_error.code)),
        h("button", { class: "small ghost", onclick: () => api.recover().then(refresh, fail) }, t("Thử lại")),
      ),
    );
  }
  if (state.recovered_from_backup) {
    banners.push(h("div", { class: "banner info" }, t("Save chính không đọc được nên game đã mở bản sao lưu gần nhất.")));
  }
  if (state.attention.length > 0) {
    banners.push(
      h("div", { class: "banner warn" },
        h("span", {}, t("{n} giao dịch chưa xác định cần bạn kiểm tra. Không file nào bị xóa.", { n: state.attention.length })),
        h("button", { class: "small ghost", onclick: () => switchTab("belly") }, t("Xem")),
      ),
    );
  }
  $("banners").replaceChildren(...banners);
}

function switchTab(next: Tab) {
  tab = next;
  if (next === "belly") {
    api.bellyList().then((b) => {
      bellyEntries = b;
      render();
    }, fail);
  }
  render();
}

function renderTabs() {
  const tabs: [Tab, string, number?][] = [
    ["shop", t("Cửa hàng")],
    ["feed", t("Cho cá ăn")],
    ["belly", t("Bụng cá"), state.held_count],
    ["tank", t("Bể của tôi")],
    ["den", t("Hang trứng"), state.eggs.length],
    ["savings", t("Gửi tiết kiệm"), state.savings.filter((b) => b.matures_at <= Date.now() / 1000).length],
    ["hunt", t("Trục vớt Vỏ sò")],
    ["settings", t("Cài đặt")],
  ];
  $("tabs").replaceChildren(
    ...tabs.map(([id, label, count]) =>
      h("button", { role: "tab", "aria-selected": String(tab === id), onclick: () => switchTab(id) },
        label,
        count ? h("span", { class: "count" }, String(count)) : null,
      ),
    ),
  );
}

function speciesOf(id: string): Species | undefined {
  return state.species.find((s) => s.id === id);
}

/** Fish keep the species name they were bought under; show it in the current language. */
const fishLabel = (f: Fish) => { const sp = speciesOf(f.species_id); return sp ? speciesName(sp) : f.name; };

function fishArt(species: Species | undefined) {
  return h("div", { class: "art" }, species ? h("img", { src: `/${species.sprite}`, alt: speciesName(species), loading: "lazy" }) : "🐟");
}

function renderShop(): Node {
  const cards = state.species.map((s) => {
    const need = slotsOf(s.price);
    const full = need > freeSlots();
    const short = s.price - state.cbcoins;
    const buy = h("button", {
      class: "primary small",
      disabled: working || full || short > 0 || !!state.read_only,
      title: full ? t("Không đủ chỗ: cần {need} chỗ, bể còn {free} chỗ", { need, free: freeSlots() }) : short > 0 ? t("Còn thiếu {n} CBCoin", { n: short }) : undefined,
      onclick: () => confirmBuy(s),
    }, full ? t("Bể đầy") : short > 0 ? t("Thiếu {n}", { n: short }) : t("Đổi"));
    return h("article", { class: "card" },
      fishArt(s),
      h("div", { class: "body" },
        h("h3", {}, speciesName(s)),
        h("span", { class: "sci" }, s.scientific_name),
        h("div", { class: "tags" },
          h("span", { class: "tag" }, s.reproduction === "live_birth" ? t("Đẻ con") : t("Đẻ trứng")),
          h("span", { class: `tag${need > 1 ? " big" : ""}` }, t("Chiếm {n} chỗ", { n: need })),
          state.dex[s.id]?.owned ? h("span", { class: "tag" }, t("Đã có")) : null,
        ),
        h("div", { class: "actions" },
          h("span", { class: "price" }, `${s.price} CBCoin`),
          h("span", {},
            h("button", { class: "ghost small", onclick: () => showSpecies(s) }, t("Chi tiết")), " ",
            buy,
          ),
        ),
      ),
    );
  });
  return h("section", {},
    h("div", { class: "section-head" }, h("h2", {}, t("Cửa hàng"))),
    h("div", { class: "grid" }, ...cards),
    h("p", { class: "fine", style: "margin-top:16px" }, t(state.disclaimer)),
  );
}

function showSpecies(s: Species) {
  modal(
    h("div", {},
      h("div", { class: "modal-art" }, h("img", { src: `/${s.sprite}`, alt: speciesName(s) })),
      h("h2", { style: "margin-top:12px" }, speciesName(s)),
      h("p", { class: "sci" }, s.scientific_name),
      h("p", { style: "margin:8px 0" }, speciesFact(s)),
      h("dl", { class: "kv" },
        h("dt", {}, t("Sinh sản")), h("dd", {}, s.reproduction === "live_birth" ? t("Đẻ con (không có trứng ngoài bụng)") : t("Đẻ trứng")),
        h("dt", {}, t("Khi mua")), h("dd", {}, stageName("fry")),
        h("dt", {}, t("Ghép cặp")), h("dd", {}, t("Chỉ cùng loài, cả hai đã trưởng thành (Lv.100) — dùng nút Sinh sản trên thẻ cá")),
        h("dt", {}, t("Nguồn")), h("dd", {}, h("code", {}, s.source)),
      ),
    ),
    [{ label: t("Đóng"), kind: "ghost" }],
  );
}

function confirmBuy(s: Species) {
  modal(
    h("div", {},
      h("h2", {}, t("Đổi {price} CBCoin lấy {name}?", { price: s.price, name: speciesName(s) })),
      h("p", { style: "margin-top:8px" }, t("Sau khi đổi còn {left} CBCoin · bể dùng {count}/{cap} chỗ.", { left: state.cbcoins - s.price, count: state.used_slots + slotsOf(s.price), cap: state.capacity })),
      h("p", { class: "fine" }, t("CBCoin là điểm trong game, không dùng tiền thật.")),
    ),
    [
      { label: t("Để sau"), kind: "ghost" },
      {
        label: t("Đổi"),
        kind: "primary",
        run: async () => {
          if (working) return;
          working = true;
          render();
          try {
            const fish = await api.buy(s.id, s.price);
            toast(t("Đã đón {name} vào bể!", { name: fishLabel(fish) }));
          } catch (e) {
            fail(e);
          }
          working = false;
          await refresh();
        },
      },
    ],
  );
}

/** Lv.100 is `stage_exp.adult` EXP (10 EXP per level since save schema 5). */
/** Tank slots a species takes: 1 below medium_price, 2 below large_price, else 3 (same rule as the engine). */
const slotsOf = (price: number) => (price >= state.slots.large_price ? 3 : price >= state.slots.medium_price ? 2 : 1);
const freeSlots = () => Math.max(0, state.capacity - state.used_slots);

const levelOf = (f: Fish) => Math.floor((f.exp * 100) / state.stage_exp.adult);

/** Same rule as the engine: adults sell for price ×100 minus eggs laid, younger fish for half the price. */
const isAdult = (f: Fish) => f.exp >= state.stage_exp.adult;
const saleValue = (f: Fish) => (isAdult(f) ? f.purchase_price * Math.max(1, 100 - f.eggs_used) : Math.floor(f.purchase_price / 2));

// ---------- breeding ----------

function confirmBreed(fish: Fish) {
  const partners = state.fish.filter((o) => o.id !== fish.id && o.species_id === fish.species_id && o.stage === "adult" && o.eggs_used < 99);
  const sp = speciesOf(fish.species_id);
  if (fish.eggs_used >= 99 || partners.length === 0) {
    modal(h("div", {},
      h("h2", {}, t("Sinh sản · {name}", { name: fishLabel(fish) })),
      h("p", {}, fish.eggs_used >= 99
        ? t("Giá trị cá này đã về mức giá mua gốc nên không sinh sản thêm được.")
        : t("Cần thêm một con cá cùng loài đã trưởng thành (Lv.100) trong bể để ghép cặp."))
    ), [{ label: t("Đóng"), kind: "primary" }]);
    return;
  }
  let partner = partners[0];
  let eggs = 1;
  const kidSlots = slotsOf(sp?.price ?? fish.purchase_price);
  const denRoom = Math.max(0, state.breeding.den_capacity - state.eggs.length);
  const hours = (sec: number) => (sec / 3600).toLocaleString(locale(), { maximumFractionDigits: 1 });
  const b = state.breeding;
  const rate = (p: number) => p <= b.price_low ? b.hatch_max : p >= b.price_high ? b.hatch_min : b.hatch_max + (b.hatch_min - b.hatch_max) * (p - b.price_low) / (b.price_high - b.price_low); // same formula as the engine
  const maxEggs = () => Math.max(1, Math.min(99 - fish.eggs_used, 99 - partner.eggs_used, denRoom));
  const summary = h("p", { class: "fine", role: "status" });
  const eggInput = h("input", { type: "number", min: "1", value: "1", "aria-label": t("Số trứng") }) as HTMLInputElement;
  const refreshSummary = () => {
    const r = rate(sp?.price ?? fish.purchase_price);
    eggs = Math.max(1, Math.min(maxEggs(), Math.floor(Number(eggInput.value) || 1)));
    eggInput.max = String(maxEggs());
    summary.replaceChildren(...coinText([
      t("Tỷ lệ nở mỗi trứng: {rate}% · dự kiến khoảng {kids} cá con (Lv.0). ", { rate: (r * 100).toFixed(1), kids: (eggs * r).toFixed(1) }),
      t("Mỗi cá bố mẹ mất {cost} CBCoin giá bán: {before} → {after} CBCoin ", { cost: fish.purchase_price * eggs, before: saleValue(fish), after: fish.purchase_price * Math.max(1, 100 - fish.eggs_used - eggs) }),
      t("(mỗi trứng −{price}, tối thiểu {price}). ", { price: fish.purchase_price }),
      t("Trứng vào Hang trứng và ấp {min}–{max} tiếng mới nở. Hang còn {room} chỗ. ", { min: hours(state.breeding.incubate_min_s), max: hours(state.breeding.incubate_max_s), room: denRoom }),
      t("Khi nở, mỗi cá con cần {kid} chỗ trong bể; bể đầy thì trứng chờ trong hang.", { kid: kidSlots }),
    ].join("")));
  };
  const select = h("select", { "aria-label": t("Chọn cá ghép cặp"), onchange: (ev: Event) => {
    partner = partners.find((o) => o.id === (ev.currentTarget as HTMLSelectElement).value) ?? partner;
    refreshSummary();
  } }, ...partners.map((o) => h("option", { value: o.id }, t("{name} · Lv.{lv} · giá bán {price} CBCoin", { name: fishLabel(o), lv: levelOf(o), price: saleValue(o) }))));
  eggInput.addEventListener("input", refreshSummary);
  refreshSummary();
  modal(h("div", { class: "breed-form" },
    h("h2", {}, t("Sinh sản · {name}", { name: fishLabel(fish) })),
    h("p", {}, t("Ghép cặp với cá cùng loài đã trưởng thành. Mỗi trứng trừ giá trị bán của cả hai cá đúng một lần giá mua gốc, nên cá càng sinh sản càng bán được ít.")),
    h("label", {}, t("Cá ghép cặp"), " ", select),
    h("label", {}, t("Số trứng"), " ", eggInput),
    summary,
  ), [{ label: t("Để sau"), kind: "ghost" }, { label: t("🥚 Sinh sản"), kind: "primary", run: async () => {
    if (working) return;
    working = true; render();
    try {
      const out = await api.breed(fish.id, partner.id, eggs);
      toast(t("🥚 Đã đẻ {n} trứng vào Hang trứng! Mỗi trứng nở sau 2–3 tiếng (tỷ lệ {rate}%).", { n: out.laid, rate: (out.hatch_rate * 100).toFixed(1) }));
      tab = "den";
    } catch (e) { fail(e); }
    finally { working = false; await refresh(); }
  }}]);
}

function confirmSell(fish: Fish) {
  modal(h("div", {},
    h("h2", {}, t("Gọi thuyền bán {name}?", { name: fishLabel(fish) })),
    isAdult(fish)
      ? h("p", {}, t("Level 100 · Giá bán: {price} CBCoin (giá mua ×100", { price: saleValue(fish) }) + (fish.eggs_used ? t(", trừ {n} trứng đã đẻ", { n: fish.eggs_used }) : "") + ").")
      : h("p", {}, t("Cá chưa trưởng thành (Lv.{lv}) chỉ bán lại được 1/2 giá mua: {price} CBCoin. Nuôi tới Lv.100 thì bán được giá mua ×100.", { lv: levelOf(fish), price: saleValue(fish) })),
    h("p", { class: "fine" }, t("Thuyền chỉ mang cá trong game đi. Các file trong Bụng cá vẫn được giữ để khôi phục."))
  ), [{ label: t("Để sau"), kind: "ghost" }, { label: t("Gọi thuyền"), kind: "primary", run: async () => {
    if (working) return;
    working = true; render();
    const boat = h("div", { class: "sale-boat", role: "status" },
      h("img", { src: "/art/hunt/boat.png", alt: t("Thuyền đến đón cá trưởng thành") }),
      h("p", {}, t("Thuyền đang đến đón {name}…", { name: fishLabel(fish) }))
    );
    document.body.append(boat);
    try {
      await boat.animate([{ transform: "translateX(-100vw)", opacity: 0 }, { transform: "translateX(0)", opacity: 1 }], {
        duration: matchMedia("(prefers-reduced-motion: reduce)").matches ? 0 : 1200, fill: "forwards"
      }).finished;
      const coins = await api.sell(fish.id);
      toast(t("⛵ Thuyền đã đón {name}! +{coins} CBCoin", { name: fishLabel(fish), coins }));
    } catch (e) { fail(e); }
    finally { boat.remove(); working = false; await refresh(); }
  }}]);
}

function renderFeed(): Node {
  const okCount = preview.filter((p) => p.ok).length;
  const choose = async () => {
    const picked = await open({ multiple: true, directory: false, title: t("Chọn file không cần nữa") });
    if (picked) previewPaths(Array.isArray(picked) ? picked : [picked]);
  };
  const sample = async () => {
    try {
      const path = await api.createSample();
      await previewPaths([path]);
    } catch (e) {
      fail(e);
    }
  };
  const rows = preview.map((p) =>
    h("div", { class: `row-item ${p.ok ? "ok" : "bad"}` },
      h("span", { class: "mark", "aria-label": p.ok ? t("Hợp lệ") : t("Bị từ chối") }, p.ok ? "✓" : "✕"),
      h("div", {},
        h("div", { class: "name" }, p.name || p.path),
        h("div", { class: "path" }, p.path),
        h("div", { class: "why" }, reason(p.code)),
      ),
      h("div", { class: "meta" }, p.size ? formatSize(p.size) : "", h("br"), p.category ? categoryName(p.category) : ""),
    ),
  );
  const fishSelect = h("select", {
    "aria-label": t("Cá được cho ăn"),
    onchange: (e: Event) => { selectedFish = (e.target as HTMLSelectElement).value; render(); },
  }, ...state.fish.map((f) => h("option", { value: f.id, selected: f.id === selectedFish }, `${fishLabel(f)} · Lv.${levelOf(f)} · ${stageName(f.stage)}${f.resting_until > Date.now()/1000 ? t(" · Đang no") : ""}`)));
  return h("section", {},
    h("div", { class: "section-head" }, h("h2", {}, t("Cho cá ăn"))),
    h("p", { class: "lead" }, t("Chọn file bạn không cần nữa. File được chuyển vào Bụng cá và có thể nhả lại bất cứ lúc nào — game không bao giờ xóa file. Chuyển trong cùng ổ đĩa không làm tăng dung lượng trống.")),
    h("div", { class: "feed-layout" },
      h("div", {},
        h("div", { class: "dropzone" },
          h("strong", {}, t("Thả file vào cửa sổ này hoặc chọn file")),
          h("span", { class: "fine" }, t("Tối đa {n} file mỗi lượt. Xem trước không di chuyển file nào.", { n: state.max_preview_files })),
          h("div", { class: "row" },
            h("button", { onclick: choose, disabled: working }, t("Chọn file…")),
            h("button", { class: "ghost", onclick: sample, disabled: working }, t("Tạo file mẫu để thử")),
          ),
        ),
        preview.length ? h("div", { class: "list" }, ...rows) : null,
        feedReport ? renderFeedReport(feedReport) : null,
      ),
      h("aside", { class: "side" },
        h("h3", {}, t("Xác nhận")),
        h("label", { class: "fine" }, t("Cá nhận thức ăn")),
        fishSelect,
        h("button", {
          class: "primary",
          disabled: working || okCount === 0 || !selectedFish || !!state.read_only || !!state.belly_error || !!state.fish.find(f => f.id === selectedFish && (f.exp >= state.stage_exp.adult || f.resting_until > Date.now()/1000)),
          onclick: feed,
        }, working ? t("Đang xử lý…") : okCount ? t("Cho cá ăn {n} file", { n: okCount }) : t("Chưa có file hợp lệ")),
        h("ul", {},
          h("li", {}, t("Mỗi file được kiểm tra lại ngay trước khi chuyển.")),
          h("li", {}, t("Chỉ nhận file cùng ổ đĩa với Bụng cá; không bao giờ sao chép rồi xóa.")),
          h("li", {}, t("File mới được so toàn bộ nội dung bằng SHA-256 trên máy. Copy hoặc đổi tên không được EXP lần nữa. Lịch sử cũ dùng dấu vân tay đầu file để chặn thưởng lặp.")),
          h("li", {}, t("Nhả file ra không thu hồi EXP đã nhận. Mỗi 20 MB là một bậc 20 EXP; 10 EXP = 1 level. Mỗi 5 level nghỉ 2 tiếng, EXP dư được giữ để tiêu hóa sau.")),
        ),
      ),
    ),
  );
}

function renderFeedReport(r: FeedReport): Node {
  const moved = r.files.filter((f) => f.ok).length;
  const refused = r.files.filter((f) => !f.ok);
  return h("div", { class: "result" },
    h("strong", {}, t("Đã đưa {n} file vào Bụng cá.", { n: moved })),
    r.reward
      ? h("div", {},
          t("+{exp} EXP · {dup} file trùng không ghi nhận", { exp: r.reward.exp, dup: r.reward.duplicates }),
          r.reward.capped ? t(" · đã chạm hạn mức hôm nay") : "",
        )
      : null,
    r.reward_error ? h("div", {}, t("File đã an toàn trong Bụng cá nhưng chưa lưu được thưởng; game sẽ đối soát lại một lần khi lưu được.")) : null,
    ...refused.map((f) => h("div", { class: "fine" }, `✕ ${f.name}: ${f.undetermined ? t("Chưa xác định, đang kiểm tra Bụng cá") : reason(f.code)}`)),
  );
}

function renderBelly(): Node {
  const fishName = (id: string) => { const f = state.fish.find((x) => x.id === id); return f ? fishLabel(f) : "—"; };
  const restore = async (entry: BellyEntry) => {
    if (working) return;
    working = true;
    render();
    try {
      const out = await api.restore(entry.id);
      toast(out.renamed ? t("Đã nhả ra với tên mới vì trùng tên: {path}", { path: out.path }) : t("Đã nhả về {path}", { path: out.path }));
    } catch (e) {
      fail(e);
    }
    working = false;
    await refresh();
  };
  const attention = state.attention.map((a) =>
    h("div", { class: "row-item bad" },
      h("span", { class: "mark" }, "?"),
      h("div", {},
        h("div", { class: "name" }, a.name || a.transaction_id),
        h("div", { class: "why" }, attentionNote(a.note)),
        a.from ? h("div", { class: "path" }, t("Từ: "), a.from) : null,
        a.to ? h("div", { class: "path" }, t("Đến: "), a.to) : null,
      ),
      h("div", { class: "meta" }, a.op === "restore" ? t("Nhả ra") : t("Cho ăn")),
    ),
  );
  const rows = bellyEntries.map((e) =>
    h("div", { class: "row-item ok" },
      h("span", { class: "mark" }, "◍"),
      h("div", {},
        h("div", { class: "name" }, e.name),
        h("div", { class: "path" }, t("Vị trí gốc: "), e.original_path),
        h("div", { class: "fine" }, `${formatSize(e.size)} · ${categoryName(e.category)} · ${formatDate(e.eaten_at)} · ${fishName(e.fish_id)}`),
      ),
      h("button", { class: "ghost small", disabled: working, onclick: () => restore(e) }, t("Nhả ra")),
    ),
  );
  return h("section", {},
    h("div", { class: "section-head" },
      h("h2", {}, t("Bụng cá")),
      h("button", { class: "ghost small", disabled: working, onclick: () => api.recover().then(refresh, fail) }, t("Kiểm tra giao dịch dang dở")),
    ),
    h("p", { class: "lead" }, t("File được giữ nguyên byte cho tới khi bạn nhả ra; không tự xóa sau bất kỳ thời hạn nào. Nhả về đúng chỗ cũ; nếu đã có file trùng tên thì đặt tên “(restored N)” thay vì ghi đè.")),
    attention.length ? h("div", { class: "list" }, h("h3", {}, t("Cần bạn kiểm tra")), ...attention) : null,
    rows.length ? h("div", { class: "list" }, ...rows) : h("div", { class: "empty" }, t("Bụng cá đang trống.")),
  );
}

function renderTank(): Node {
  const owned = Object.values(state.dex).filter((d) => d.owned).length;
  const cards = state.fish.map((f) => {
    const s = speciesOf(f.species_id);
    const next = f.stage === "fry" ? state.stage_exp.juvenile : f.stage === "juvenile" ? state.stage_exp.adult : null;
    const pct = next ? Math.min(100, Math.round((f.exp / next) * 100)) : 100;
    return h("article", { class: "card fish-card" },
      fishArt(s),
      h("div", { class: "body" },
        h("h3", {}, `${fishLabel(f)} · Lv.${levelOf(f)}`),
        h("span", { class: "sci" }, s ? s.scientific_name : f.species_id),
        h("div", { class: "tags" },
          h("span", { class: "tag" }, stageName(f.stage)),
          h("span", { class: "tag" }, f.origin === "starter" ? t("Cá khởi đầu") : f.origin === "hatched" ? t("Cá nở trong bể · đời {n}", { n: f.generation }) : t("Từ cửa hàng")),
        ),
        h("div", { class: "bar", role: "progressbar", "aria-valuenow": String(pct), "aria-valuemin": "0", "aria-valuemax": "100" }, h("i", { style: `width:${pct}%` })),
        h("span", { class: "fine" }, next ? t("{exp} / {next} EXP · {pending} EXP đang tiêu hóa", { exp: f.exp, next, pending: f.pending_exp }) : t("Level 100 · đã trưởng thành")),
        f.resting_until > Date.now()/1000 ? h("p", {}, t("🫧 Bụng em thành bóng rồi! Nghỉ tới {time} nhé.", { time: formatDate(f.resting_until) })) : null,
        f.stage === "adult" ? h("div", { class: "tags" },
          h("span", { class: "tag big", title: t("Gọi thuyền ra bể desktop rồi gắp con cá này để bán.") }, t("🎣 Béo rồi · gắp được {price} CBCoin", { price: saleValue(f) })),
          h("button", { class: "small", disabled: working || !!state.read_only, onclick: () => confirmBreed(f) }, t("🥚 Sinh sản")),
        ) : h("div", { class: "tags" },
          h("button", { class: "ghost small", disabled: working || !!state.read_only, title: t("Mua nhầm hoặc bể đầy? Bán lại bằng 1/2 giá mua."), onclick: () => confirmSell(f) }, t("Bán lại · {price} CBCoin", { price: saleValue(f) })),
        ),
        f.eggs_used ? h("span", { class: "fine" }, t("Đã đẻ {n} trứng", { n: f.eggs_used })) : null,
      ),
    );
  });
  return h("section", {},
    h("div", { class: "section-head" },
      h("h2", {}, t("Bể của tôi")),
      h("span", { class: "fine" }, t("Fishdex: đã sở hữu {owned}/{total} loài", { owned, total: state.species.length })),
    ),
    h("p", { class: "lead" }, t("Cá không bao giờ chết hay bỏ đi khi bạn nghỉ chơi. Cho cá ăn để cá lớn từ cá non → thành niên (Lv.50) → trưởng thành (Lv.100).")),
    h("div", { class: "grid" }, ...cards),
  );
}

// ---------- egg den ----------

function renderDen(): Node {
  const now = Date.now() / 1000;
  const full = freeSlots() < 1;
  const eggs = [...state.eggs].sort((a, b) => a.hatch_at - b.hatch_at);
  const card = (e: (typeof eggs)[number]) => {
    const sp = speciesOf(e.species_id);
    const timer = h("span", { class: "egg-timer", "data-hatch": String(e.hatch_at), "data-full": full ? "1" : "" }, "");
    paintTimer(timer);
    return h("article", { class: `egg-card${e.hatch_at - now < 600 ? " soon" : ""}` },
      h("div", { class: "egg-wrap" }, h("div", { class: "egg", style: `--spot:${eggHue(e.species_id)}` }), timer),
      h("strong", {}, sp ? speciesName(sp) : e.species_id),
      h("span", { class: "fine" }, t("Đời {n} · nở lúc {time}", { n: e.generation, time: formatDate(e.hatch_at) })),
    );
  };
  const log = state.hatch_log.slice(0, 12).map((r) => {
    const sp = speciesOf(r.species_id);
    const name = sp ? speciesName(sp) : r.species_id;
    return h("div", { class: `row-item ${r.hatched ? "ok" : "bad"}` },
      h("span", { class: "mark" }, r.hatched ? "🐣" : "💨"),
      h("div", {}, h("div", { class: "name" }, r.hatched ? t("{name} đã nở!", { name }) : t("Một trứng {name} không nở", { name }))),
      h("div", { class: "meta" }, formatDate(r.at)),
    );
  });
  return h("section", {},
    h("div", { class: "section-head" },
      h("h2", {}, t("Hang trứng")),
      h("span", { class: "fine" }, t("{n}/{cap} trứng", { n: eggs.length, cap: state.breeding.den_capacity })),
    ),
    h("p", { class: "lead" }, t("Mọi trứng từ sinh sản đều nằm ở đây. Mỗi trứng ấp 2–3 tiếng rồi mới nở thành cá con Lv.0 (game tắt vẫn tính giờ). Bể đầy thì trứng đến giờ sẽ chờ trong hang đến khi có chỗ.")),
    full && eggs.length ? h("div", { class: "banner warn" }, t("Bể đã đầy — trứng đến giờ sẽ chờ trong hang. Bán bớt cá để trứng nở.")) : null,
    eggs.length ? h("div", { class: "egg-grid" }, ...eggs.map(card)) : h("div", { class: "empty" }, t("Hang trứng đang trống. Cho hai cá cùng loài Lv.100 sinh sản để có trứng.")),
    log.length ? h("div", { class: "list" }, h("h3", {}, t("Vừa nở gần đây")), ...log) : null,
  );
}

/** Same egg, different speckle colour per species. */
function eggHue(id: string) {
  let n = 0;
  for (const c of id) n = (n * 31 + c.charCodeAt(0)) % 360;
  return `hsl(${n} 45% 45%)`;
}

function paintTimer(el: HTMLElement) {
  const left = Number(el.dataset.hatch) - Date.now() / 1000;
  el.textContent = left > 0 ? formatCountdown(left) : el.dataset.full ? t("chờ chỗ") : t("sắp nở!");
  el.classList.toggle("due", left <= 0);
}

// Egg timers tick every second without re-rendering the page.
setInterval(() => document.querySelectorAll<HTMLElement>(".egg-timer").forEach(paintTimer), 1000);

let seenHatch: string | null | undefined;
function announceHatches() {
  const latest = state.hatch_log[0]?.egg_id ?? null;
  if (seenHatch === undefined) { seenHatch = latest; return; }
  if (latest === seenHatch) return;
  const fresh = [];
  for (const r of state.hatch_log) { if (r.egg_id === seenHatch) break; fresh.push(r); }
  seenHatch = latest;
  const born = fresh.filter((r) => r.hatched).length;
  if (born) toast(t("🐣 {n} trứng vừa nở! Ra Bể của tôi xem cá con nhé.", { n: born }));
  else if (fresh.length) toast(t("💨 {n} trứng đã đến giờ nhưng không nở.", { n: fresh.length }));
}

// ---------- savings (Cá Mập's counter) ----------

let saveAmount = "";
let saveTerm = 7;
const money = (n: number) => n.toLocaleString(locale());

function bankerLine(books: SavingsBook[], due: number): string {
  if (due > 0) return t("Sổ tới hạn rồi kìa! Bấm Tất toán nhận lãi liền nha.");
  const rules = state.savings_rules;
  const days = rules.terms_days.includes(7) ? 7 : rules.terms_days[rules.terms_days.length - 1];
  const lines = [
    t("Kỳ hạn {days} ngày lãi {pct}% luôn á, gửi liền đi anh ơi!", { days, pct: termPercent(rules, days) }),
    t("CBCoin nằm im trong ví thì cá cũng không lớn thêm đâu nha."),
    t("Đáo hạn nhớ ghé chị tất toán nha, lãi không tự chạy vào ví đâu."),
  ];
  if (books.length === 0) return t("“Chờ vài hôm nữa anh qua em gửi tiết kiệm nha”… câu này chị nghe hoài rồi đó!");
  return lines[Math.floor(Date.now() / 60_000) % lines.length];
}

function renderSavings(): Node {
  const rules = state.savings_rules;
  if (!rules.terms_days.includes(saveTerm)) saveTerm = rules.terms_days[Math.min(2, rules.terms_days.length - 1)];
  const now = Date.now() / 1000;
  const books = [...state.savings].sort((a, b) => a.matures_at - b.matures_at);
  const due = books.filter((b) => b.matures_at <= now).length;
  const full = books.length >= rules.max_books;

  const art = h("div", { class: "banker-art" }, h("img", { src: SHARK_IMG, alt: "" }));
  const banker = h("div", { class: "banker" },
    art,
    h("div", { class: "banker-info" },
      h("div", { class: "bubble" }, bankerLine(books, due)),
      h("h3", {}, t("Cá Mập Tiết Kiệm")),
      h("p", { class: "banker-title" }, BANKER.title),
      h("p", {}, "Contact for work: ", h("b", {}, BANKER.phone)),
      h("p", {}, "TikTok ID: ", h("b", {}, BANKER.tiktok)),
    ),
  );

  const chips = h("div", { class: "term-chips", role: "radiogroup", "aria-label": t("Kỳ hạn") },
    ...rules.terms_days.map((d) =>
      h("button", { class: `term${d === saveTerm ? " on" : ""}`, role: "radio", "aria-checked": String(d === saveTerm), onclick: () => { saveTerm = d; render(); } },
        h("span", {}, t("{d} ngày", { d })), h("b", {}, `+${termPercent(rules, d)}%`))));

  const summary = h("p", { class: "save-summary" });
  const submit = h("button", { class: "primary", onclick: () => deposit() }, t("Gửi tiết kiệm"));
  const input = h("input", {
    type: "number", min: String(rules.min_deposit), max: String(state.cbcoins), step: "1", value: saveAmount,
    placeholder: t("Số CBCoin"), "aria-label": t("Số CBCoin muốn gửi"),
    oninput: (e: Event) => { saveAmount = (e.currentTarget as HTMLInputElement).value; paintSummary(); },
    onkeydown: (e: KeyboardEvent) => { if (e.key === "Enter") deposit(); },
  });
  const quick = (label: string, part: number) => h("button", { class: "small ghost", onclick: () => {
    saveAmount = String(Math.floor(state.cbcoins * part)); input.value = saveAmount; paintSummary();
  } }, label);

  const amountNow = () => Math.floor(Number(saveAmount) || 0);
  function paintSummary() {
    const amount = amountNow();
    const interest = savingsInterest(rules, amount, saveTerm);
    let text: string;
    let ok = false;
    if (full) text = t("Đã đủ {n} sổ. Tất toán bớt một sổ rồi gửi tiếp nha.", { n: rules.max_books });
    else if (amount < rules.min_deposit) text = t("Gửi tối thiểu {n} CBCoin.", { n: rules.min_deposit });
    else if (amount > state.cbcoins) text = t("Ví chỉ có {n} CBCoin.", { n: money(state.cbcoins) });
    else {
      ok = true;
      text = t("Đáo hạn {date}: nhận {total} CBCoin (lãi {interest}).", {
        date: formatDate(now + saveTerm * 86_400), total: money(amount + interest), interest: money(interest) })
        + (interest >= rules.max_interest ? t(" Đã chạm lãi tối đa {n} CBCoin mỗi sổ.", { n: money(rules.max_interest) }) : "");
    }
    summary.replaceChildren(...coinText(text));
    summary.classList.toggle("bad", !ok && amount > 0);
    submit.disabled = !ok || working || !!state.read_only;
  }
  async function deposit() {
    const amount = amountNow();
    if (submit.disabled) return;
    submit.disabled = true;
    try {
      const book = await api.openSavings(amount, saveTerm);
      saveAmount = "";
      toast(t("🦈 Cá Mập: Cảm ơn nha! Sổ {d} ngày đã mở, {date} quay lại nhận {total} CBCoin nha.", {
        d: book.term_days, date: formatDate(book.matures_at), total: money(book.principal + book.interest) }));
    } catch (e) { fail(e); }
    await refresh();
  }
  paintSummary();

  const form = h("div", { class: "save-form" },
    h("h3", {}, t("Mở sổ tiết kiệm")),
    chips,
    h("div", { class: "save-amount" }, input, quick("50%", 0.5), quick(t("Tất cả"), 1)),
    summary,
    submit,
  );

  const bookCard = (b: SavingsBook) => {
    const done = b.matures_at <= now;
    const timer = h("span", { class: "book-timer", "data-due": String(b.matures_at) });
    paintBookTimer(timer);
    const action = done
      ? h("button", { class: "small primary", onclick: () => withdraw(b) }, t("Tất toán +{n}", { n: money(b.principal + b.interest) }))
      : h("button", { class: "small ghost", onclick: () => confirmEarly(b) }, t("Rút trước hạn"));
    return h("article", { class: `book${done ? " due" : ""}` },
      h("div", { class: "book-head" }, h("strong", {}, t("Sổ {d} ngày", { d: b.term_days })), h("span", { class: "fine" }, t("Mở lúc {time}", { time: formatDate(b.opened_at) }))),
      h("div", { class: "book-money" }, `${money(b.principal)} CBCoin`, h("span", { class: "gain" }, `+${money(b.interest)}`)),
      h("div", { class: "book-bar" }, h("i", { style: `width:${(progress(b.opened_at, b.matures_at, now) * 100).toFixed(1)}%` })),
      h("div", { class: "book-foot" }, timer, action),
    );
  };

  const locked = books.reduce((sum, b) => sum + b.principal, 0);
  return h("section", { class: "savings" },
    h("div", { class: "section-head" },
      h("h2", {}, t("Gửi tiết kiệm")),
      h("span", { class: "fine" }, t("{n}/{max} sổ · đang gửi {locked} CBCoin", { n: books.length, max: rules.max_books, locked: money(locked) })),
    ),
    h("p", { class: "lead" }, t("Gửi CBCoin cho Cá Mập theo kỳ hạn: lãi {rate}% mỗi ngày, cộng dồn theo số ngày. Đáo hạn bấm Tất toán để nhận cả gốc lẫn lãi; rút trước hạn chỉ nhận lại gốc. Game tắt vẫn tính ngày.", { rate: termPercent(rules, 1) })),
    h("div", { class: "savings-top" }, banker, form),
    books.length ? h("div", { class: "book-grid" }, ...books.map(bookCard)) : h("div", { class: "empty" }, t("Chưa có sổ nào. Gửi thử một ít CBCoin xem lãi chạy nha!")),
    h("p", { class: "fine" }, t("Tối đa {books} sổ, lãi tối đa {cap} CBCoin mỗi sổ. Lãi suất ở đây là luật chơi, chỉ tính bằng CBCoin trong game — không phải lãi suất hay sản phẩm của ngân hàng thật.", { books: rules.max_books, cap: money(rules.max_interest) })),
  );
}

function paintBookTimer(el: HTMLElement) {
  const left = Number(el.dataset.due) - Date.now() / 1000;
  if (left <= 0) { el.textContent = t("Đã đáo hạn!"); el.classList.add("done"); return; }
  const [days, clock] = splitCountdown(left);
  el.textContent = days > 0 ? t("Còn {d} ngày {time}", { d: days, time: clock }) : t("Còn {time}", { time: clock });
}
setInterval(() => document.querySelectorAll<HTMLElement>(".book-timer").forEach(paintBookTimer), 1000);

async function withdraw(b: SavingsBook) {
  try {
    const p = await api.withdrawSavings(b.id);
    toast(p.early
      ? t("Đã rút {n} CBCoin trước hạn — không có lãi.", { n: money(p.principal) })
      : t("💰 +{total} CBCoin · lãi {interest} từ sổ {d} ngày!", { total: money(p.principal + p.interest), interest: money(p.interest), d: b.term_days }));
  } catch (e) { fail(e); }
  await refresh();
}

function confirmEarly(b: SavingsBook) {
  const [days, clock] = splitCountdown(b.matures_at - Date.now() / 1000);
  const left = days > 0 ? t("{d} ngày {time}", { d: days, time: clock }) : clock;
  modal(
    h("div", {},
      h("h2", {}, t("Rút trước hạn?")),
      h("p", {}, t("Rút bây giờ chỉ nhận lại {principal} CBCoin, mất {interest} CBCoin tiền lãi. Ráng chờ thêm {left} nữa thôi mà!", {
        principal: money(b.principal), interest: money(b.interest), left })),
    ),
    [{ label: t("Thôi, chờ tiếp"), kind: "primary" }, { label: t("Vẫn rút"), kind: "ghost", run: () => withdraw(b) }],
  );
}

function renderHunt(): Node {
  const remaining = state.hunt.batch?.shells.filter((s) => !s.collected).length ?? 0;
  const fat = state.fish.filter(isAdult).length;
  const session = state.hunt.session;
  const act = (action: "keep" | "sell") => () => api.huntAction(session!.id, Date.now(), action).then(refresh, fail);
  return h("section", { class: "settings" },
    h("h2", {}, t("Trục vớt Vỏ sò")),
    h("p", { class: "lead" }, t("Gọi thuyền ra thẳng bể cá trên desktop, canh móc đung đưa rồi thả để gắp sò. Kéo về thuyền mới nhận CBCoin (sò trắng 1, đỏ 10, tím 100).")),
    h("p", {}, t("Cá Lv.100 hiện chữ \"Cá béo lắm rồi, bắt điii!\" trên desktop. Gắp được cá lên thuyền thì chọn nuôi thêm hoặc bán luôn với giá mua ×100. Cá bơi qua lại nên phải canh đúng lúc.")),
    session?.phase === "deciding" ? h("div", { class: "banner info" },
      h("span", {}, t("🎣 Cá lên thuyền rồi! Nuôi thêm hay bán?")),
      h("button", { class: "small primary", onclick: act("sell") }, t("💰 Yehh nay có cơm ăn rồiiii")),
      h("button", { class: "small ghost", onclick: act("keep") }, t("🐟 Cá gầy quá nuôi thêm chút vậyyyy")),
    ) : null,
    h("div", { class: "setting" }, h("div", {},
      h("h3", {}, (remaining ? t("{n} sò đang chờ dưới đáy", { n: remaining }) : t("Chưa thấy sò mới")) + (fat ? t(" · {n} cá béo chờ gắp", { n: fat }) : "")),
      h("p", {}, t("Sò xuất hiện từng đợt 1–10, thời gian không cố định. Sò được giữ lại khi bạn bận. Lần đầu có 3 sò hướng dẫn."))),
      state.hunt.session
        ? h("button", { onclick: () => api.huntAction(state.hunt.session!.id, Date.now(), "leave").then(refresh, fail) }, t("Rời thuyền"))
        : h("button", { class: "primary", disabled: !!state.read_only || state.settings.meeting_mode || (state.hunt.earned >= state.hunt.daily_cap && fat === 0),
            onclick: () => api.startHunt().then(refresh, fail) }, t("Gọi thuyền"))),
    h("p", {}, t("Đã nhặt hôm nay: {earned}/{cap}. Hạn mức riêng với cho cá ăn; không thưởng EXP.", { earned: state.hunt.earned, cap: state.hunt.daily_cap })),
    h("p", { class: "fine" }, t("Thả móc bằng Space (Windows, khi đang ở màn hình desktop) hoặc Ctrl+Alt+Space, hoặc nút nhanh ở góc phải dưới (hiện khi bạn ở màn hình desktop, Win+D). Rời khỏi desktop sẽ tạm dừng. Trượt không mất điểm; rời thuyền giữ sò chưa nhặt.")),
    collectionPanel(state.hunt),
  );
}

function renderCodeBox(): Node {
  const input = h("input", { type: "text", class: "code-input", placeholder: t("Nhập code…"), "aria-label": t("Nhập code"), autocomplete: "off", spellcheck: false }) as HTMLInputElement;
  const redeem = async () => {
    const code = input.value.trim();
    if (!code || working) return;
    working = true;
    try {
      const out = await api.redeem(code);
      toast(out.kind === "cbcoins"
        ? t("🎉 Code hợp lệ! +{n} CBCoin", { n: out.cbcoins.toLocaleString(locale()) })
        : t("🎉 Code hợp lệ! {n} cá trong bể đã lên Lv.100", { n: out.fish }));
      input.value = "";
    } catch (e) { fail(e); }
    working = false;
    await refresh();
  };
  input.addEventListener("keydown", (e) => { if (e.key === "Enter") void redeem(); });
  return h("div", { class: "setting" },
    h("div", {}, h("h3", {}, t("Nhập code")), h("p", {}, t("Có code quà tặng? Nhập vào đây để nhận thưởng."))),
    h("div", { class: "code-row" }, input, h("button", { class: "primary small", onclick: redeem }, t("Đổi code"))),
  );
}

function renderSettings(): Node {
  const toggle = (checked: boolean, label: string, onFlip: (v: boolean) => Promise<unknown>) =>
    h("button", {
      class: "switch",
      role: "switch",
      "aria-checked": String(checked),
      "aria-label": label,
      onclick: async (e: Event) => {
        (e.currentTarget as HTMLButtonElement).disabled = true;
        try {
          await onFlip(!checked);
        } catch (err) {
          fail(err);
        }
        await refresh();
      },
    });
  return h("section", { class: "settings" },
    h("h2", {}, t("Cài đặt")),
    h("div", { class: "setting" },
      h("div", {}, h("h3", {}, t("Ngôn ngữ")), h("p", {}, t("Chọn ngôn ngữ hiển thị. Áp dụng ngay cho mọi cửa sổ của game."))),
      h("select", { "aria-label": t("Ngôn ngữ"), onchange: (e: Event) => setLang((e.currentTarget as HTMLSelectElement).value as Lang) },
        ...LANGS.map((l) => h("option", { value: l.id, selected: getLang() === l.id }, l.label)))),
    renderCodeBox(),
    h("div", { class: "setting" },
      h("div", {}, h("h3", {}, t("Mở khi đăng nhập Windows")), h("p", {}, t("Khởi động xuống khay hệ thống, dùng lựa chọn bể lần trước. Mặc định tắt. Chỉ bật từ bản đã cài."))),
      autostart === null ? h("span", { class: "fine" }, t("Không đọc được trạng thái")) : toggle(autostart, t("Mở cùng hệ thống"), async (v) => { autostart = await api.setAutostart(v); })),
    h("div", { class: "setting" },
      h("div", {}, h("h3", {}, t("Nút nhanh trên desktop")), h("p", {}, t("Cho ăn, gọi thuyền, mua cá và các thao tác khác. Windows: chỉ hiện khi bể bật và desktop đang dùng."))),
      toggle(state.settings.quick_dock_enabled, t("Nút nhanh"), (v) => api.setDock(v))),
    h("div", { class: "setting" },
      h("div", {},
        h("h3", {}, t("Bể cá desktop")),
        h("p", {}, t("Đại dương nằm dưới biểu tượng desktop, không che ứng dụng đang dùng và không nhận chuột. Tắt lúc nào cũng được. Không đổi hình nền hay theme hệ thống.")),
      ),
      toggle(state.settings.tank_enabled, t("Bể cá desktop"), (v) => api.setTank(v)),
    ),
    h("div", { class: "setting" },
      h("div", {},
        h("h3", {}, t("Chế độ họp")),
        h("p", {}, t("Giảm chuyển động và khung hình, không bật thông báo tự động.")),
      ),
      toggle(state.settings.meeting_mode, t("Chế độ họp"), (v) => api.setMeetingMode(v)),
    ),
    h("div", { class: "setting" },
      h("div", {},
        h("h3", {}, t("Dữ liệu trên máy")),
        h("p", {}, t("Save, Bụng cá và sổ giao dịch nằm tại "), h("code", {}, state.data_dir), t(". Gỡ cài đặt không xóa thư mục này. Không mạng, không tài khoản, không telemetry.")),
      ),
    ),
    h("div", { class: "setting" },
      h("div", {}, h("h3", {}, t("Thoát hẳn")), h("p", {}, t("Đóng cửa sổ chỉ ẩn game xuống khay/menu bar. Thoát hẳn sẽ tắt cả bể desktop."))),
      h("button", { class: "ghost", onclick: () => api.quit() }, t("Thoát")),
    ),
  );
}

// ---------- first run & window lifecycle ----------

// ---------- intro ----------

/** Bump to show the intro again to everyone after big gameplay changes. */
const INTRO_VERSION = "3";
const INTRO_KEY = "tq.intro";

function introSeen(): boolean {
  try { return localStorage.getItem(INTRO_KEY) === INTRO_VERSION; } catch { return false; }
}
function markIntroSeen() {
  try { localStorage.setItem(INTRO_KEY, INTRO_VERSION); } catch { /* storage unavailable */ }
}

/** A little stage of game art for each intro slide (ocean background + sprites). */
function introArt(kind: string): HTMLElement {
  const img = (src: string, cls: string, alt = "") => h("img", { src, class: cls, alt });
  const fish = (id: string) => `/fish/${id}.png`;
  const bubble = (text: string, cls: string) => h("span", { class: `intro-bubble ${cls}` }, text);
  const stage = h("div", { class: `intro-stage ${kind}`, "aria-hidden": "true" });
  const parts: Record<string, () => Node[]> = {
    welcome: () => [img(fish("poecilia_reticulata"), "swim a"), img(fish("amphiprion_ocellaris"), "swim b"), img(fish("pterophyllum_scalare"), "swim c")],
    feed: () => [h("span", { class: "intro-file f1" }, "📄"), h("span", { class: "intro-file f2" }, "🗂️"), h("span", { class: "intro-file f3" }, "📄"), img(fish("carassius_auratus"), "eat")],
    grow: () => [img(fish("poecilia_reticulata"), "g1"), img(fish("poecilia_reticulata"), "g2"), img(fish("poecilia_reticulata"), "g3"),
      h("span", { class: "intro-lv l1" }, "Lv.0"), h("span", { class: "intro-lv l2" }, "Lv.50"), h("span", { class: "intro-lv l3" }, "Lv.100")],
    hunt: () => [img("/art/hunt/boat.png", "boat"), h("span", { class: "rope" }), img("/art/hunt/claw.png", "claw"),
      img("/art/hunt/shell_0.png", "sh s0"), img("/art/hunt/shell_1.png", "sh s1"), img("/art/hunt/shell_2.png", "sh s2")],
    catch: () => [img("/art/hunt/boat.png", "boat"), h("span", { class: "rope long" }), img(fish("carcharodon_carcharias"), "shark"),
      bubble(t("Cá béo lắm rồi, bắt điii! 🎣"), "fat")],
    breed: () => [img(fish("betta_splendens"), "p1"), img(fish("betta_splendens"), "p2"),
      h("span", { class: "intro-egg e1" }), h("span", { class: "intro-egg e2" }), h("span", { class: "intro-egg e3" }), bubble("2:41:08", "timer")],
    ready: () => [img(fish("rhincodon_typus"), "swim big"), img(fish("poecilia_reticulata"), "swim a"), img(fish("thunnus_albacares"), "swim c")],
  };
  stage.append(...(parts[kind] ?? parts.welcome)());
  return stage;
}

/** Picture slides introducing the game. Shown on first run, once more after big updates, and from the ? button. */
function showIntro(firstRun = false) {
  const dialog = $("modal") as HTMLDialogElement;
  const starter = state.fish[0];
  const slides: { art: string; title: string; text: string[] }[] = [
    { art: "welcome", title: t("Chào mừng tới TrashQuarium!"), text: [
      t("Bể cá sống ngay trên màn hình nền của bạn — và thức ăn của cá là… những file bạn không cần nữa."),
      firstRun ? t("Bạn được tặng {name} và {coins} CBCoin để bắt đầu.", { name: starter ? fishLabel(starter) : t("một chú cá"), coins: state.cbcoins }) : t("Lướt nhanh qua các tính năng chính nhé."),
    ] },
    { art: "feed", title: t("Cho cá ăn file rác"), text: [
      t("Thả file vào tab Cho cá ăn. File chui vào Bụng cá: không bị xóa, nhả ra lúc nào cũng được."),
      t("Mỗi file cho EXP theo dung lượng; 10 EXP = 1 level."),
    ] },
    { art: "grow", title: t("Cá lớn từng ngày"), text: [
      t("Lv.0 cá non → Lv.50 thành niên → Lv.100 trưởng thành. Cứ 5 level cá no căng bụng và ngủ trưa 2 tiếng."),
      t("Cá không bao giờ chết hay bỏ đi khi bạn nghỉ chơi."),
    ] },
    { art: "hunt", title: t("Gọi thuyền gắp sò"), text: [
      t("Thuyền ra ngay bể desktop. Canh móc đung đưa rồi bấm Space để thả."),
      t("Sò trắng 1, đỏ 10, tím 100 CBCoin. Sò hiếm có thể giấu ngọc trai."),
    ] },
    { art: "catch", title: t("Cá béo thì gắp lên thuyền"), text: [
      t("Cá Lv.100 hiện chữ “Cá béo lắm rồi, bắt điii!”. Gắp lên thuyền rồi chọn bán luôn (giá mua ×100) hay nuôi thêm."),
      t("Cá bơi qua lại, phải canh đúng lúc mới trúng."),
    ] },
    { art: "breed", title: t("Mua cá và cho sinh sản"), text: [
      t("Bể có 20 chỗ, cá to chiếm nhiều chỗ hơn."),
      t("Ghép hai cá Lv.100 cùng loài để đẻ trứng; trứng ấp 2–3 tiếng trong Hang trứng rồi mới nở."),
    ] },
    { art: "ready", title: t("Sẵn sàng rồi!"), text: [
      t("Bật Bể cá desktop trong Cài đặt để cá bơi ngay dưới biểu tượng màn hình nền."),
      t("Bấm nút ? ở góc trên bất cứ lúc nào để xem lại phần giới thiệu này."),
    ] },
  ];
  let i = 0;
  const finish = async (sample: boolean) => {
    dialog.close();
    markIntroSeen();
    if (!state.settings.onboarding_done) await api.finishOnboarding().catch(fail);
    if (sample) {
      tab = "feed";
      const path = await api.createSample().catch((e) => (fail(e), null));
      await refresh();
      if (path) await previewPaths([path]);
    } else await refresh();
  };
  const show = () => {
    const slide = slides[i];
    const last = i === slides.length - 1;
    const dots = h("div", { class: "intro-dots" }, ...slides.map((_, k) => h("button", {
      class: k === i ? "on" : "", "aria-label": t("Trang {n}", { n: k + 1 }), onclick: () => { i = k; show(); },
    })));
    const nav = h("div", { class: "modal-actions" },
      !last ? h("button", { class: "ghost", onclick: () => void finish(false) }, t("Bỏ qua")) : null,
      i > 0 ? h("button", { class: "ghost", onclick: () => { i--; show(); } }, t("← Trước")) : null,
      last && firstRun ? h("button", { class: "ghost", onclick: () => void finish(true) }, t("Thử với file mẫu")) : null,
      last ? h("button", { class: "ghost", onclick: () => { dialog.close(); markIntroSeen(); showGuide(); } }, t("Xem bảng tóm tắt")) : null,
      h("button", { class: "primary", onclick: () => { if (last) void finish(false); else { i++; show(); } } }, last ? t("Bắt đầu chơi") : t("Tiếp →")),
    );
    dialog.replaceChildren(
      h("div", { class: "intro" },
        introArt(slide.art),
        h("div", { class: "modal-body" }, h("h2", {}, slide.title), ...slide.text.map((p) => h("p", {}, p)), dots),
      ),
      nav,
    );
  };
  show();
  if (!dialog.open) dialog.showModal();
}

function askClose() {
  modal(
    h("div", {},
      h("h2", {}, t("Ẩn TrashQuarium?")),
      h("p", { style: "margin-top:8px" }, t("Game vẫn chạy ở khay hệ thống (Windows) hoặc menu bar (macOS) — bấm biểu tượng cá để mở lại. Bể desktop vẫn bơi nếu đang bật.")),
    ),
    [
      { label: t("Thoát hẳn"), kind: "ghost", run: () => api.quit() },
      { label: t("Ẩn cửa sổ"), kind: "primary", run: () => getCurrentWindow().hide() },
    ],
  );
}

function applyStatic() {
  $("drop-hint").textContent = t("Thả file vào đây để xem trước — chưa có file nào bị chuyển");
}

async function main() {
  applyStatic();
  onLangChange(() => { applyStatic(); void api.setTrayLanguage(getLang()).catch(() => {}); if (state) render(); });
  void api.setTrayLanguage(getLang()).catch(() => {});
  autostart = await api.autostartStatus().catch(() => null);
  await refresh();
  if (!state) return;
  await listen("state-changed", refresh);
  setInterval(() => { if (!working && preview.length === 0) void refresh(); }, 15000);
  const route = async () => {
    const next = await api.takeRoute();
    if (next && ["shop", "feed", "belly", "tank", "den", "savings", "settings", "hunt"].includes(next)) {
      if (working || preview.length > 0) { toast(t("Hoàn thành hoặc bỏ lượt xem trước trước khi chuyển màn.")); return; }
      switchTab(next as Tab);
    }
  };
  await listen("manager-route", route);
  await route();
  await getCurrentWindow().onFocusChanged(async ({ payload }) => {
    if (payload) { autostart = await api.autostartStatus().catch(() => null); render(); }
  });
  await listen<string>("tank-error", (e) =>
    toast(t("Không gắn được bể vào desktop ({why}). Bể đã tắt; game không chuyển sang cửa sổ phủ toàn màn hình.", { why: reason(e.payload) }), true),
  );
  await getCurrentWindow().onCloseRequested((event) => {
    event.preventDefault();
    askClose();
  });
  await getCurrentWebview().onDragDropEvent((event) => {
    const hint = $("drop-hint");
    if (event.payload.type === "enter" || event.payload.type === "over") hint.hidden = false;
    else hint.hidden = true;
    if (event.payload.type === "drop") previewPaths(event.payload.paths);
  });
  if (!state.settings.onboarding_done) showIntro(true);
  else if (!introSeen()) showIntro();
}

main();
