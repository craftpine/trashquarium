# TrashQuarium 0.3 (Tauri rewrite)

Cozy desktop aquarium: pick files you no longer need, they go into the fish's
**Belly** (always restorable, never deleted), you earn **Vỏ sò** (game shells)
and buy real fish species from a 10-species shop.

Reimplementation of `../specs/TrashQuarium-Source-2026-09-28` against
`../specs/TRASHQUARIUM-MASTER-SPEC-V4.md`, milestones **A** (file safety,
Belly, recovery, save, desktop lifecycle) and **B** (shop, wallet, feeding).
Runs on macOS and Windows.

## Stack

- `src-tauri/src/engine/` — Rust core, no Tauri dependency, fully unit tested:
  - `guard.rs` FileGuard (fail-closed intake checks, 1 MiB fingerprint)
  - `belly.rs` journaled Belly vault (`prepared → file_moved → indexed → committed`), crash recovery, restore without overwrite
  - `save.rs` atomic JSON with rotating backups
  - `game.rs` wallet, fish, shop, exactly-once reward receipts, daily caps
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
- Rewards: Belly receipt → one atomic save of wallet + EXP + ledger. A failed
  save leaves the receipt pending and it is paid once later. Recovery-completed
  moves are not rewarded. Duplicate fingerprints pay nothing (the fingerprint is
  size + first 1 MiB, not a full checksum; the list keeps the newest 50 000).
- Daily caps reset only when the local date moves forward.

## Shell hunt, quick dock and startup

- Open **Trục vớt Vỏ sò** in the manager, or **Gọi thuyền** from the Windows
  quick dock. The first visit provides three tutorial shells once. Later batches
  contain 1–10 shells after a random 8–22 minutes of running time. Sleep, shutdown
  and Meeting Mode do not advance the clock. Uncollected shells never expire.
- Click the play area or press Space while the hunt window has focus to drop
  the swinging claw. One shell per catch; pulling it to the boat credits one
  game shell. Misses cost nothing. Focus loss pauses the session. Resume with
  **Tiếp tục**; closing returns an unpaid catch to the same batch.
- Hunt rewards have a separate 60-shell daily cap, without fish EXP. Rust owns
  the simulation and commits wallet, daily count and collected flag together.
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
- Tauri save schema 1 is read and migrated to 2 with existing fish, wallet,
  receipts and Belly preserved. Older app builds will open the new save read-only.

### Validation / release limits

The frontend is type-checked and built locally. Rust tests cover reachability,
pause, catches/retraction, replay-safe payout, failed saves and migration; the
Windows workflow runs them on PRs. Windows GUI checks for autostart, dock
focus/Explorer and installer behaviour remain required before release. This
change does not apply a Windows theme or alter file intake safety.

## Still not done

- Milestones C–E: breeding, ancestry, eggs, trait layers, birth cards, full
  Fishdex, Ancient/Mythic lines, Museum/events.
- Taskbar-overlay mode and Windows themes (wallpaper/cursor/colours).
- Fish rename, sounds, English localisation.
- Windows GUI QA: desktop attachment, DPI, multi-monitor and the installer
  were type-checked from macOS but never run on Windows.
- Species facts are `draft`, pending editorial and scientific review.
