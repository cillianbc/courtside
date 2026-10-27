# Courtside

Volleyball scouting and video analysis for Windows, macOS and Linux.

Courtside is a desktop app for coding volleyball matches live or from video. It reads and writes DataVolley `.dvw` files, so the files it makes open in DataVolley 4 and VolleyStation Pro, and your existing matches open in Courtside. It's free software under the [GNU AGPL v3](LICENSE).

> **Status: early.** The coding console is being tested by scouts in real matches. There's no public app release yet. The command-line tool is released and stable.

**User guide:** [courtside-2.gitbook.io/courtside-docs](https://courtside-2.gitbook.io/courtside-docs): installing, coding a match, video and clips, files, and the command-line tool.

## What it does

- **Live coding from the keyboard.** Type codes the way DataVolley or VolleyStation scouts do: shorthand and dot codes are filled in, the serve comes from the rotation, the setter's set is added for you, and every code is read back in plain words as you type.
- **Checks while you code.** Score, rotation, substitutions, liberos, back-row play and how each rally's touches fit together are checked live, with the first real fault shown and the issues that follow from it grouped under it.
- **Video.** Frame-accurate playback (built on [mpv](https://mpv.io)) with codes stamped to the video, a scrub bar showing every rally and code, sync to the first serve, and clip playlists filtered by player, skill, evaluation, rotation or phase.
- **Search and fix.** Find codes across the match (⌘F / Ctrl+F), step through them on video, and modify, add or remove codes. The score and rotations that follow are rewritten.
- **Files.** Import and export `.dvw`, read VolleyStation `.vsm`, and import or export team rosters (`.sq`, `.vst`).

## Download

**Desktop app.** Preview builds will be published on this repository's [Releases page](https://github.com/cillianbc/courtside/releases): a `.dmg` for macOS (Apple silicon, macOS 14 or later) and an installer for Windows, with video playback included and nothing else to install. [Installing](https://courtside-2.gitbook.io/courtside-docs/getting-started/install) covers the one-time security prompt on each system.

**Command-line tool** (`courtside validate | convert | stats`). It checks `.dvw` and `.vsm` files for errors, converts matches to JSON for R, Python or spreadsheets, and prints match statistics. [Latest release](https://github.com/cillianbc/courtside-cli/releases/latest).

```sh
brew install cillianbc/tap/courtside                      # macOS
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/cillianbc/courtside-cli/releases/latest/download/courtside-cli-installer.sh | sh   # macOS / Linux
```

Windows (PowerShell):

```powershell
powershell -ExecutionPolicy Bypass -c "irm https://github.com/cillianbc/courtside-cli/releases/latest/download/courtside-cli-installer.ps1 | iex"
```

## Build from source

You need Rust (stable), Node 22 with pnpm 10, and mpv's library: `brew install mpv` on macOS, `libmpv-dev` on Debian or Ubuntu. On Windows, set `MPV_LIB_DIR` to a folder with `mpv.lib` and `libmpv-2.dll`.

```sh
cargo test --workspace                  # the Rust crates
cd apps/desktop && pnpm install
pnpm tauri dev                          # run the app
pnpm dev                                # the layout alone in a browser, with made-up data
```

`tools/bundle/macos.sh` builds a self-contained `.dmg`.

## How it's built

A Rust core shared by every surface, with a [Tauri](https://tauri.app) and Svelte desktop app on top.

| Folder | What it is |
|---|---|
| `crates/dvw-codec` | `.dvw` and `.vsm` reading and writing, lossless, every encoding and dialect seen in real files |
| `crates/vb-grammar` | the scouting-code grammar and shorthand expansion |
| `crates/vb-engine` | event log, match state, validation and statistics |
| `crates/courtside-store` | the season database (SQLite) |
| `crates/courtside-console` | the coding console's logic, independent of any UI |
| `crates/courtside-cli` | the command-line tool |
| `apps/desktop` | the desktop app |

Design decisions are in [`docs/adr`](docs/adr). The file-format findings, each checked against real match files, are in [`docs/FORMAT-NOTES.md`](docs/FORMAT-NOTES.md).

## Feedback and contributing

Issues are welcome: [open one](https://github.com/cillianbc/courtside/issues). Match files contain real players' names, so please don't attach them to public issues. Describe the problem, or use [Report a problem](https://courtside-2.gitbook.io/courtside-docs/troubleshooting/reporting-a-problem) in the app, which emails the Courtside team directly.

## Licence

Courtside is licensed under the [GNU Affero General Public License v3.0 only](LICENSE).

DataVolley and VolleyStation are trademarks of their owners. Courtside is an independent project, not affiliated with or endorsed by them; it reads and writes their file formats for interoperability.
