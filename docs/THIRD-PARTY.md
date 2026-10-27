# Third-party software in Courtside's installers

Courtside is licensed under the GNU AGPL v3 ([LICENSE](../LICENSE)). Its installers also carry software by others, under its own licences. Each installer includes a list of that software, the licence texts, and where the source is:

- **macOS:** `Courtside.app/Contents/Resources/third-party/` (right-click the app → Show Package Contents).
- **Windows:** the `third-party` folder in the folder Courtside is installed in.

## Video playback: mpv and FFmpeg (GPL)

Courtside plays video with [libmpv](https://mpv.io), which uses [FFmpeg](https://ffmpeg.org) and other libraries to decode video. These are free software under the GPL (mpv: GPL-2.0-or-later; FFmpeg as built: GPL-3.0-or-later), with LGPL, BSD and MIT parts. Courtside doesn't modify them. Combining them with Courtside is allowed: the AGPL v3 and GPL v3 permit it (section 13 of each).

- **macOS:** the libraries are [Homebrew](https://brew.sh)'s builds, bundled by `tools/bundle/macos.sh`. `THIRD-PARTY-NOTICES.txt` lists each one with its version, licence and source tarball.
- **Windows:** `libmpv-2.dll` is a build from [shinchiro/mpv-winbuild-cmake](https://github.com/shinchiro/mpv-winbuild-cmake), downloaded by the `Desktop app` workflow. `THIRD-PARTY-NOTICES.txt` names the release and the mpv commit; that release's build recipe fixes the version of every library inside.

### Written offer

For three years from the release of any Courtside installer, the Courtside project will provide the complete corresponding source code of the GPL software in that installer, on request, at no more than the cost of providing it. Write to **cillianbc@gmail.com** with the Courtside version and your system.

## Courtside's own dependencies

The Rust crates and JavaScript packages Courtside is built from are under permissive licences (MIT, Apache-2.0, BSD and similar) or the MPL-2.0. They're listed in `Cargo.lock` and `apps/desktop/pnpm-lock.yaml`. The fonts (IBM Plex, Barlow Condensed) are under the SIL Open Font License.
