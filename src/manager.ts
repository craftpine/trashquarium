import "./style.css";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { open } from "@tauri-apps/plugin-dialog";
import { api, asFailure, type BellyEntry, type FeedReport, type Inspection, type Species, type StateView } from "./api";
import { attentionNote, categoryName, formatDate, formatSize, reason, stageName } from "./i18n";

type Tab = "shop" | "feed" | "belly" | "tank" | "settings" | "hunt";
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
  for (const c of children) if (c !== null && c !== undefined && c !== false) el.append(c);
  return el;
}

// ---------- feedback ----------

let toastTimer = 0;
function toast(text: string, error = false) {
  const t = $("toast");
  t.textContent = text;
  t.className = `show${error ? " error" : ""}`;
  clearTimeout(toastTimer);
  toastTimer = window.setTimeout(() => (t.className = ""), error ? 7000 : 4500);
}

function fail(e: unknown) {
  const f = asFailure(e);
  toast(f.undetermined ? "Chưa xác định — đang kiểm tra Bụng cá…" : reason(f.code), true);
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
      toast(`Chỉ xem trước ${state.max_preview_files} file đầu tiên mỗi lượt.`);
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
      toast("Có file chưa xác định — đang kiểm tra Bụng cá…", true);
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
    settings: renderSettings,
    hunt: renderHunt,
  };
  panel.replaceChildren(views[tab]());
}

function renderStats() {
  $("stats").replaceChildren(
    h("span", { class: "pill shells", title: "Vỏ sò là điểm trong game, không phải tiền thật" }, h("b", {}, String(state.shells)), "Vỏ sò"),
    h("span", { class: "pill" }, h("b", {}, `${state.fish.length}/${state.capacity}`), "cá trong bể"),
    h("span", { class: "pill", title: "Hạn mức Vỏ sò nhận từ cho cá ăn mỗi ngày" }, "Hôm nay", h("b", {}, `${state.daily.shells}/${state.daily.shell_cap}`)),
  );
}

function renderBanners() {
  const banners: Node[] = [];
  if (state.read_only) {
    banners.push(h("div", { class: "banner danger" }, h("span", {}, reason(state.read_only.code), " Thư mục dữ liệu: ", h("code", {}, state.data_dir))));
  }
  if (state.belly_error) {
    banners.push(
      h("div", { class: "banner danger" },
        h("span", {}, "Bụng cá tạm khóa: ", reason(state.belly_error.code)),
        h("button", { class: "small ghost", onclick: () => api.recover().then(refresh, fail) }, "Thử lại"),
      ),
    );
  }
  if (state.recovered_from_backup) {
    banners.push(h("div", { class: "banner info" }, "Save chính không đọc được nên game đã mở bản sao lưu gần nhất."));
  }
  if (state.attention.length > 0) {
    banners.push(
      h("div", { class: "banner warn" },
        h("span", {}, `${state.attention.length} giao dịch chưa xác định cần bạn kiểm tra. Không file nào bị xóa.`),
        h("button", { class: "small ghost", onclick: () => switchTab("belly") }, "Xem"),
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
    ["shop", "Cửa hàng"],
    ["feed", "Cho cá ăn"],
    ["belly", "Bụng cá", state.held_count],
    ["tank", "Bể của tôi"],
    ["hunt", "Trục vớt Vỏ sò"],
    ["settings", "Cài đặt"],
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

function fishArt(species: Species | undefined) {
  return h("div", { class: "art" }, species ? h("img", { src: `/${species.sprite}`, alt: species.name, loading: "lazy" }) : "🐟");
}

function renderShop(): Node {
  const full = state.fish.length >= state.capacity;
  const cards = state.species.map((s) => {
    const short = s.price - state.shells;
    const buy = h("button", {
      class: "primary small",
      disabled: working || full || short > 0 || !!state.read_only,
      title: full ? "Bể đã đầy" : short > 0 ? `Còn thiếu ${short} Vỏ sò` : undefined,
      onclick: () => confirmBuy(s),
    }, full ? "Bể đầy" : short > 0 ? `Thiếu ${short}` : "Đổi");
    return h("article", { class: "card" },
      fishArt(s),
      h("div", { class: "body" },
        h("h3", {}, s.name),
        h("span", { class: "sci" }, s.scientific_name),
        h("div", { class: "tags" },
          h("span", { class: "tag" }, "Có thật"),
          h("span", { class: "tag" }, s.reproduction === "live_birth" ? "Đẻ con" : "Đẻ trứng"),
          state.dex[s.id]?.owned ? h("span", { class: "tag" }, "Đã có") : null,
        ),
        h("div", { class: "actions" },
          h("span", { class: "price" }, `${s.price} Vỏ sò`),
          h("span", {},
            h("button", { class: "ghost small", onclick: () => showSpecies(s) }, "Chi tiết"), " ",
            buy,
          ),
        ),
      ),
    );
  });
  return h("section", {},
    h("div", { class: "section-head" }, h("h2", {}, "Cửa hàng · 10 loài cá thật")),
    h("p", { class: "lead" }, "Dọn file an toàn → nhận Vỏ sò → chọn cá. Vỏ sò là điểm trong game, không mua bằng tiền thật. Bể chung chỉ là không gian game, không phải hướng dẫn nuôi chung các loài ngoài đời."),
    h("div", { class: "grid" }, ...cards),
    h("p", { class: "fine", style: "margin-top:16px" }, state.disclaimer),
  );
}

function showSpecies(s: Species) {
  modal(
    h("div", {},
      h("div", { class: "modal-art" }, h("img", { src: `/${s.sprite}`, alt: s.name })),
      h("h2", { style: "margin-top:12px" }, s.name),
      h("p", { class: "sci" }, s.scientific_name),
      s.editorial_status !== "released" ? h("p", { class: "tag draft", style: "display:inline-block;margin:6px 0" }, "Nội dung nháp — chưa được biên tập khoa học duyệt") : null,
      h("p", { style: "margin:8px 0" }, s.fact),
      h("dl", { class: "kv" },
        h("dt", {}, "Nhãn"), h("dd", {}, "Có thật"),
        h("dt", {}, "Sinh sản"), h("dd", {}, s.reproduction === "live_birth" ? "Đẻ con (không có trứng ngoài bụng)" : "Đẻ trứng"),
        h("dt", {}, "Khi mua"), h("dd", {}, stageName[s.stage_on_purchase]),
        h("dt", {}, "Ghép cặp"), h("dd", {}, "Chỉ cùng loài — tính năng gia đình cá sẽ có ở bản sau"),
        h("dt", {}, "Nguồn"), h("dd", {}, h("code", {}, s.source)),
      ),
    ),
    [{ label: "Đóng", kind: "ghost" }],
  );
}

function confirmBuy(s: Species) {
  modal(
    h("div", {},
      h("h2", {}, `Đổi ${s.price} Vỏ sò lấy ${s.name}?`),
      h("p", { style: "margin-top:8px" }, `Sau khi đổi còn ${state.shells - s.price} Vỏ sò · bể ${state.fish.length + 1}/${state.capacity}.`),
      h("p", { class: "fine" }, "Vỏ sò là điểm trong game, không dùng tiền thật."),
    ),
    [
      { label: "Để sau", kind: "ghost" },
      {
        label: "Đổi",
        kind: "primary",
        run: async () => {
          if (working) return;
          working = true;
          render();
          try {
            const fish = await api.buy(s.id, s.price);
            toast(`Đã đón ${fish.name} vào bể!`);
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

function renderFeed(): Node {
  const okCount = preview.filter((p) => p.ok).length;
  const choose = async () => {
    const picked = await open({ multiple: true, directory: false, title: "Chọn file không cần nữa" });
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
      h("span", { class: "mark", "aria-label": p.ok ? "Hợp lệ" : "Bị từ chối" }, p.ok ? "✓" : "✕"),
      h("div", {},
        h("div", { class: "name" }, p.name || p.path),
        h("div", { class: "path" }, p.path),
        h("div", { class: "why" }, reason(p.code)),
      ),
      h("div", { class: "meta" }, p.size ? formatSize(p.size) : "", h("br"), p.category ? categoryName[p.category] : ""),
    ),
  );
  const fishSelect = h("select", {
    "aria-label": "Cá được cho ăn",
    onchange: (e: Event) => (selectedFish = (e.target as HTMLSelectElement).value),
  }, ...state.fish.map((f) => h("option", { value: f.id, selected: f.id === selectedFish }, `${f.name} · ${stageName[f.stage]} · ${f.exp} EXP`)));
  return h("section", {},
    h("div", { class: "section-head" }, h("h2", {}, "Cho cá ăn")),
    h("p", { class: "lead" }, "Chọn file bạn không cần nữa. File được chuyển vào Bụng cá và có thể nhả lại bất cứ lúc nào — game không bao giờ xóa file. Chuyển trong cùng ổ đĩa không làm tăng dung lượng trống."),
    h("div", { class: "feed-layout" },
      h("div", {},
        h("div", { class: "dropzone" },
          h("strong", {}, "Thả file vào cửa sổ này hoặc chọn file"),
          h("span", { class: "fine" }, `Tối đa ${state.max_preview_files} file mỗi lượt. Xem trước không di chuyển file nào.`),
          h("div", { class: "row" },
            h("button", { onclick: choose, disabled: working }, "Chọn file…"),
            h("button", { class: "ghost", onclick: sample, disabled: working }, "Tạo file mẫu để thử"),
          ),
        ),
        preview.length ? h("div", { class: "list" }, ...rows) : null,
        feedReport ? renderFeedReport(feedReport) : null,
      ),
      h("aside", { class: "side" },
        h("h3", {}, "Xác nhận"),
        h("label", { class: "fine" }, "Cá nhận thức ăn"),
        fishSelect,
        h("button", {
          class: "primary",
          disabled: working || okCount === 0 || !selectedFish || !!state.read_only || !!state.belly_error,
          onclick: feed,
        }, working ? "Đang xử lý…" : okCount ? `Cho cá ăn ${okCount} file` : "Chưa có file hợp lệ"),
        h("ul", {},
          h("li", {}, "Mỗi file được kiểm tra lại ngay trước khi chuyển."),
          h("li", {}, "Chỉ nhận file cùng ổ đĩa với Bụng cá; không bao giờ sao chép rồi xóa."),
          h("li", {}, "Chống thưởng trùng bằng dấu vân tay đọc tối đa 1 MiB đầu + kích thước, xử lý trên máy. Đây không phải checksum toàn file: hai file khác nhau đôi khi bị coi là trùng."),
          h("li", {}, "Nhả file ra không thu hồi Vỏ sò đã nhận; cho ăn lại cùng file không được thưởng thêm."),
        ),
      ),
    ),
  );
}

function renderFeedReport(r: FeedReport): Node {
  const moved = r.files.filter((f) => f.ok).length;
  const refused = r.files.filter((f) => !f.ok);
  return h("div", { class: "result" },
    h("strong", {}, `Đã đưa ${moved} file vào Bụng cá.`),
    r.reward
      ? h("div", {},
          `+${r.reward.shells} Vỏ sò · +${r.reward.exp} EXP`,
          r.reward.duplicates ? ` · ${r.reward.duplicates} file trùng không tính thưởng` : "",
          r.reward.capped ? " · đã chạm hạn mức hôm nay" : "",
        )
      : null,
    r.reward_error ? h("div", {}, "File đã an toàn trong Bụng cá nhưng chưa lưu được thưởng; game sẽ đối soát lại một lần khi lưu được.") : null,
    ...refused.map((f) => h("div", { class: "fine" }, `✕ ${f.name}: ${f.undetermined ? "Chưa xác định, đang kiểm tra Bụng cá" : reason(f.code)}`)),
  );
}

function renderBelly(): Node {
  const fishName = (id: string) => state.fish.find((f) => f.id === id)?.name ?? "—";
  const restore = async (entry: BellyEntry) => {
    if (working) return;
    working = true;
    render();
    try {
      const out = await api.restore(entry.id);
      toast(out.renamed ? `Đã nhả ra với tên mới vì trùng tên: ${out.path}` : `Đã nhả về ${out.path}`);
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
        h("div", { class: "why" }, attentionNote[a.note] ?? a.note),
        a.from ? h("div", { class: "path" }, "Từ: ", a.from) : null,
        a.to ? h("div", { class: "path" }, "Đến: ", a.to) : null,
      ),
      h("div", { class: "meta" }, a.op === "restore" ? "Nhả ra" : "Cho ăn"),
    ),
  );
  const rows = bellyEntries.map((e) =>
    h("div", { class: "row-item ok" },
      h("span", { class: "mark" }, "◍"),
      h("div", {},
        h("div", { class: "name" }, e.name),
        h("div", { class: "path" }, "Vị trí gốc: ", e.original_path),
        h("div", { class: "fine" }, `${formatSize(e.size)} · ${categoryName[e.category]} · ${formatDate(e.eaten_at)} · ${fishName(e.fish_id)}`),
      ),
      h("button", { class: "ghost small", disabled: working, onclick: () => restore(e) }, "Nhả ra"),
    ),
  );
  return h("section", {},
    h("div", { class: "section-head" },
      h("h2", {}, "Bụng cá"),
      h("button", { class: "ghost small", disabled: working, onclick: () => api.recover().then(refresh, fail) }, "Kiểm tra giao dịch dang dở"),
    ),
    h("p", { class: "lead" }, "File được giữ nguyên byte cho tới khi bạn nhả ra; không tự xóa sau bất kỳ thời hạn nào. Nhả về đúng chỗ cũ; nếu đã có file trùng tên thì đặt tên “(restored N)” thay vì ghi đè."),
    attention.length ? h("div", { class: "list" }, h("h3", {}, "Cần bạn kiểm tra"), ...attention) : null,
    rows.length ? h("div", { class: "list" }, ...rows) : h("div", { class: "empty" }, "Bụng cá đang trống."),
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
        h("h3", {}, f.name),
        h("span", { class: "sci" }, s ? s.scientific_name : f.species_id),
        h("div", { class: "tags" },
          h("span", { class: "tag" }, stageName[f.stage]),
          h("span", { class: "tag" }, f.origin === "starter" ? "Cá khởi đầu" : "Từ cửa hàng"),
        ),
        h("div", { class: "bar", role: "progressbar", "aria-valuenow": String(pct), "aria-valuemin": "0", "aria-valuemax": "100" }, h("i", { style: `width:${pct}%` })),
        h("span", { class: "fine" }, next ? `${f.exp} / ${next} EXP tới ${stageName[f.stage === "fry" ? "juvenile" : "adult"]}` : `${f.exp} EXP · đã trưởng thành`),
      ),
    );
  });
  return h("section", {},
    h("div", { class: "section-head" },
      h("h2", {}, "Bể của tôi"),
      h("span", { class: "fine" }, `Fishdex: đã sở hữu ${owned}/${state.species.length} loài`),
    ),
    h("p", { class: "lead" }, "Cá không bao giờ chết hay bỏ đi khi bạn nghỉ chơi. Cho cá ăn để cá lớn từ cá bột → cá non → trưởng thành."),
    h("div", { class: "grid" }, ...cards),
  );
}

function renderHunt(): Node {
  const remaining = state.hunt.batch?.shells.filter((s) => !s.collected).length ?? 0;
  return h("section", { class: "settings" },
    h("h2", {}, "Trục vớt Vỏ sò"),
    h("p", { class: "lead" }, "Gọi thuyền ra thẳng bể cá trên desktop, canh móc đung đưa rồi thả để gắp sò. Kéo về thuyền mới nhận Vỏ sò."),
    h("div", { class: "setting" }, h("div", {},
      h("h3", {}, remaining ? `${remaining} sò đang chờ dưới đáy` : "Chưa thấy sò mới"),
      h("p", {}, "Sò xuất hiện từng đợt 1–10, thời gian không cố định. Sò được giữ lại khi bạn bận. Lần đầu có 3 sò hướng dẫn.")),
      state.hunt.session
        ? h("button", { onclick: () => api.huntAction(state.hunt.session!.id, Date.now(), "leave").then(refresh, fail) }, "Rời thuyền")
        : h("button", { class: "primary", disabled: !!state.read_only || state.settings.meeting_mode || state.hunt.earned >= state.hunt.daily_cap,
            onclick: () => api.startHunt().then(refresh, fail) }, "Gọi thuyền")),
    h("p", {}, `Đã nhặt hôm nay: ${state.hunt.earned}/${state.hunt.daily_cap}. Hạn mức riêng với cho cá ăn; không thưởng EXP.`),
    h("p", { class: "fine" }, "Thả móc bằng Ctrl+Alt+Space hoặc nút nhanh ở góc phải dưới (hiện khi bạn ở màn hình desktop, Win+D). Rời khỏi desktop sẽ tạm dừng. Trượt không mất điểm; rời thuyền giữ sò chưa nhặt."),
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
    h("h2", {}, "Cài đặt"),
    h("div", { class: "setting" },
      h("div", {}, h("h3", {}, "Mở khi đăng nhập Windows"), h("p", {}, "Khởi động xuống khay hệ thống, dùng lựa chọn bể lần trước. Mặc định tắt. Chỉ bật từ bản đã cài.")),
      autostart === null ? h("span", { class: "fine" }, "Không đọc được trạng thái") : toggle(autostart, "Mở cùng hệ thống", async (v) => { autostart = await api.setAutostart(v); })),
    h("div", { class: "setting" },
      h("div", {}, h("h3", {}, "Nút nhanh trên desktop"), h("p", {}, "Cho ăn, gọi thuyền, mua cá và các thao tác khác. Windows: chỉ hiện khi bể bật và desktop đang dùng.")),
      toggle(state.settings.quick_dock_enabled, "Nút nhanh", (v) => api.setDock(v))),
    h("div", { class: "setting" },
      h("div", {},
        h("h3", {}, "Bể cá desktop"),
        h("p", {}, "Đại dương nằm dưới biểu tượng desktop, không che ứng dụng đang dùng và không nhận chuột. Tắt lúc nào cũng được. Không đổi hình nền hay theme hệ thống."),
      ),
      toggle(state.settings.tank_enabled, "Bể cá desktop", (v) => api.setTank(v)),
    ),
    h("div", { class: "setting" },
      h("div", {},
        h("h3", {}, "Chế độ họp"),
        h("p", {}, "Giảm chuyển động và khung hình, không bật thông báo tự động."),
      ),
      toggle(state.settings.meeting_mode, "Chế độ họp", (v) => api.setMeetingMode(v)),
    ),
    h("div", { class: "setting" },
      h("div", {},
        h("h3", {}, "Dữ liệu trên máy"),
        h("p", {}, "Save, Bụng cá và sổ giao dịch nằm tại ", h("code", {}, state.data_dir), ". Gỡ cài đặt không xóa thư mục này. Không mạng, không tài khoản, không telemetry."),
      ),
    ),
    h("div", { class: "setting" },
      h("div", {}, h("h3", {}, "Thoát hẳn"), h("p", {}, "Đóng cửa sổ chỉ ẩn game xuống khay/menu bar. Thoát hẳn sẽ tắt cả bể desktop.")),
      h("button", { class: "ghost", onclick: () => api.quit() }, "Thoát"),
    ),
  );
}

// ---------- first run & window lifecycle ----------

function onboarding() {
  const starter = state.fish[0];
  modal(
    h("div", {},
      h("h2", {}, "Chào mừng tới TrashQuarium"),
      h("p", { style: "margin-top:8px" }, `Bạn được tặng ${starter ? starter.name : "một chú cá"} và ${state.shells} Vỏ sò để mở cửa hàng.`),
      h("p", {}, "• Bể cá desktop (tùy chọn) bơi dưới biểu tượng desktop, không che ứng dụng. Bật trong Cài đặt."),
      h("p", {}, "• Cho cá ăn bằng file bạn không cần nữa: file vào Bụng cá, luôn nhả lại được, không bao giờ bị xóa."),
      h("p", { class: "fine" }, "Không cần dùng file thật — bạn có thể thử bằng file mẫu, hoặc chỉ chăm cá và mua cá bằng quà tặng."),
    ),
    [
      { label: "Thử với file mẫu", kind: "ghost", run: async () => {
        await api.finishOnboarding().catch(fail);
        tab = "feed";
        const path = await api.createSample().catch((e) => (fail(e), null));
        await refresh();
        if (path) await previewPaths([path]);
      } },
      { label: "Bắt đầu", kind: "primary", run: async () => {
        await api.finishOnboarding().catch(fail);
        await refresh();
      } },
    ],
  );
}

function askClose() {
  modal(
    h("div", {},
      h("h2", {}, "Ẩn TrashQuarium?"),
      h("p", { style: "margin-top:8px" }, "Game vẫn chạy ở khay hệ thống (Windows) hoặc menu bar (macOS) — bấm biểu tượng cá để mở lại. Bể desktop vẫn bơi nếu đang bật."),
    ),
    [
      { label: "Thoát hẳn", kind: "ghost", run: () => api.quit() },
      { label: "Ẩn cửa sổ", kind: "primary", run: () => getCurrentWindow().hide() },
    ],
  );
}

async function main() {
  autostart = await api.autostartStatus().catch(() => null);
  await refresh();
  if (!state) return;
  await listen("state-changed", refresh);
  const route = async () => {
    const next = await api.takeRoute();
    if (next && ["shop", "feed", "belly", "tank", "settings", "hunt"].includes(next)) {
      if (working || preview.length > 0) { toast("Hoàn thành hoặc bỏ lượt xem trước trước khi chuyển màn."); return; }
      switchTab(next as Tab);
    }
  };
  await listen("manager-route", route);
  await route();
  await getCurrentWindow().onFocusChanged(async ({ payload }) => {
    if (payload) { autostart = await api.autostartStatus().catch(() => null); render(); }
  });
  await listen<string>("tank-error", (e) =>
    toast(`Không gắn được bể vào desktop (${reason(e.payload)}). Bể đã tắt; game không chuyển sang cửa sổ phủ toàn màn hình.`, true),
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
  if (!state.settings.onboarding_done) onboarding();
}

main();
