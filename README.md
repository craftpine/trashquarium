# TrashQuarium 0.3 (Tauri rewrite)

Cozy desktop aquarium: pick files you no longer need, they go into the fish's
**Belly** (always restorable, never deleted), the fish gain EXP and grow. You
earn **CBCoin** (in-game points) by hunting shells with the boat and buy real
fish species from a 64-species shop. See `docs/economy-growth.md`.

Reimplementation of `../specs/TrashQuarium-Source-2026-09-28` against
`../specs/TRASHQUARIUM-MASTER-SPEC-V4.md`, milestones **A** (file safety,
Belly, recovery, save, desktop lifecycle) and **B** (shop, wallet, feeding).
Runs on macOS and Windows.

## Stack

- `src-tauri/src/engine/` — Rust core, no Tauri dependency, fully unit tested:
  - `guard.rs` FileGuard (fail-closed intake checks, full-content SHA-256 for new rewards)
  - `belly.rs` journaled Belly vault (`prepared → file_moved → indexed → committed`), crash recovery, restore without overwrite
  - `save.rs` atomic JSON with rotating backups
  - `game.rs` CBCoin wallet, fish levels/rest, shop, selling, shell collection, exactly-once reward receipts
  - `catalog.rs` bundled species/balance data + validator (`config/*.json`)
  - `app.rs` facade used by the Tauri commands
- `src-tauri/src/desktop.rs` — desktop tank window below the icons (macOS window level / Windows WorkerW), idle detection
- `src-tauri/src/lib.rs` — commands, tray/menu-bar, single instance
- `src/` — TypeScript front end: `manager.ts` (shop, feeding, Belly, tank, settings), `tank.ts` (canvas ocean, 30 FPS / 10 FPS idle)

## Run

Needs Node 20+, Rust 1.89+ (Windows: WebView2 + MSVC build tools).

```sh
npm install
npm run tauri dev          # run
npm test                   # Rust core tests
npm run tauri build        # .app/.dmg on macOS, per-user NSIS installer on Windows
```

Use a throwaway profile for QA so real data is untouched:

```sh
TRASHQUARIUM_DATA_DIR=/tmp/tq-qa npm run tauri dev
```

Fill a QA profile with fake fish (never the real profile):

```sh
npm run seed -- --dir /tmp/tq-qa --count 30 --shells 5000 --seed 1
```

Default data folder: `%LOCALAPPDATA%\TrashQuarium` (Windows),
`~/Library/Application Support/TrashQuarium` (macOS). Uninstalling does not
remove it.

## Safety contract (P0)

- No code path deletes a user file. Files are only renamed, same volume, with
  no-replace semantics (`renamex_np RENAME_EXCL` / `MoveFileExW` without
  `REPLACE_EXISTING` or `COPY_ALLOWED`). Cross-volume files are refused.
- Refused: relative/UNC/device/ADS paths, links and reparse points (file or any
  parent), folders, drive roots, system and app-data folders, app/library
  packages (e.g. `.photoslibrary`), repositories, hidden/system/cloud-only
  files, non-allowlisted extensions, > 4 GiB, locked files. Installers only
  from the OS Downloads folder. Anything that can't be checked is refused.
- Preview never moves anything; each file is re-inspected right before the move.
- Journal is written before the move. On start, recovery settles unfinished
  transactions from what is on disk; ambiguous cases are parked as
  "needs attention" with both copies kept.
- Rewards (schema 4): Belly receipt → one atomic save of EXP + ledger (feeding gives EXP only; CBCoin comes from shells). A failed
  save leaves the receipt pending and it is paid once later. Recovery-completed
  moves are not rewarded. Duplicate content pays nothing (new rewards hash the whole
  file; older ledger entries keep the legacy size + first 1 MiB fingerprint).
- The 60-shell daily hunt cap resets only when the local date moves forward.

## Shell hunt, quick dock and startup

- Press **Gọi thuyền** in the manager or the Windows quick dock. The hunt is
  played directly on the desktop tank (no separate window): the boat appears at
  the top of the sea and the shells lie on the seabed. The first visit provides
  three tutorial shells once. Later batches contain 1–10 shells after a random
  8–22 minutes of running time. Sleep, shutdown and Meeting Mode do not advance
  the clock. Uncollected shells never expire.
- The tank is click-through, so controls are the global hotkeys **Space** (Windows
  only, claimed only while a hunt runs and the desktop is in front) and
  **Ctrl+Alt+Space** (registered only during a hunt), and the dock's **Thả móc / Tạm dừng / Rời thuyền**
  buttons. One shell per catch; pulling it to the boat credits CBCoin by colour
  (white 1, red 10, purple 100). About 8 % of shells are rare (gold glow) and a
  rare shell may hold a pearl; each shell type opens a collection card the first
  time (shown in the manager's hunt tab). A catch notice appears under the status
  pill. Misses cost nothing. On Windows the session pauses automatically when the
  desktop is covered by other windows (use Win+D to show it); resume with
  **Tiếp tục**. Leaving returns an unpaid catch to the same batch. macOS has no
  dock yet, so only the hotkey is available there and there is no auto-pause.
  If another program owns a hotkey it is ignored and the dock still works.
  The dock's speaker button toggles optional synthesized hunt sounds (off by
  default, remembered); sounds play from the dock window, so macOS has none.
- Hunt rewards have a separate 60-shell daily cap (counts shells, not CBCoin),
  without fish EXP. Rust owns the simulation and commits wallet, collection,
  daily count and collected flag together.
  Reopening the app neither rerolls the batch nor grants the tutorial again.
- **Mở khi đăng nhập Windows** in Settings registers opt-in autostart. It is
  off by default; startup launches to the tray with the saved tank preference.
  Changing autostart is disabled in development builds to avoid registering
  a temporary executable. The toggle reads the OS registration, not a save flag.
- The quick dock is a small, separately sized input window on Windows. The
  full-screen tank remains click-through. Expand the boat button for feeding,
  shopping, shell hunting and settings/Belly. The dock hides outside the desktop
  and in Meeting Mode; Settings can disable it. Position is currently fixed to
  the monitor's lower-right corner. Dock visibility needs Windows GUI QA.
- Feeding gives EXP only: (floor(bytes / 20 MB) + 1) × 20 per file, 100 EXP = 1
  level, every 5 levels the fish rests 2 hours (leftover EXP is kept), Lv.50 =
  sub-adult, Lv.100 = adult. Adult fish can be sold to the boat for purchase
  price × 100 CBCoin. Resting fish show a speech bubble on the desktop.
- Save schemas 1–3 are read and migrated to 4 with existing fish, wallet
  (`shells` → `cbcoins` 1:1), receipts and Belly preserved. Older app builds
  will open the new save read-only. Full rules: `docs/economy-growth.md`;
  hunt features and QA notes: `docs/hunt-features.md` (written for the earlier
  separate hunt window; the on-desktop renderer is in `src/tank.ts`).

### Validation / release limits

The frontend is type-checked and built locally. Rust tests cover reachability,
pause, catches/retraction, replay-safe payout, failed saves and migration.
Hunt sprites in `public/art/hunt` are AI-generated (prompts and QA in `scripts/hunt-art/ai/`); `scripts/hunt-art/build.py` renders a vector fallback set into `scripts/hunt-art/vector-fallback/`. Windows GUI checks for autostart, dock,
hotkey, on-desktop hunt and focus/Explorer and installer behaviour remain required before release. This
change does not apply a Windows theme or alter file intake safety.

## Still not done

- Milestones C–E: breeding, ancestry, eggs, trait layers, birth cards, full
  Fishdex, Ancient/Mythic lines, Museum/events.
- Taskbar-overlay mode and Windows themes (wallpaper/cursor/colours).
- Fish rename, sounds, English localisation.
- Windows GUI QA: desktop attachment, DPI, multi-monitor and the installer
  were type-checked from macOS but never run on Windows.
- Species facts are `draft`, pending editorial and scientific review.
