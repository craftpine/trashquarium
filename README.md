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

## Not done yet

- Milestones C–E: breeding, ancestry, eggs, trait layers, birth cards, full
  Fishdex, Ancient/Mythic lines, Museum/events.
- Taskbar-overlay mode and Windows themes (wallpaper/cursor/colours).
- Fish rename, sounds, English localisation, in-game non-file shell source.
- Windows GUI QA: desktop attachment, DPI, multi-monitor and the installer
  were type-checked from macOS but never run on Windows.
- Species facts are `draft`, pending editorial and scientific review.
