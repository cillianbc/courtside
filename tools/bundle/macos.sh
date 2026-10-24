#!/usr/bin/env bash
# Builds Courtside.app and a .dmg that need nothing installed: libmpv and
# every library it loads are copied into Contents/Frameworks and the app
# points at them there (docs/adr/0004). Ad-hoc signed (no Apple account):
# the first open goes through System Settings → Privacy & Security →
# Open Anyway.
#
# Needs Homebrew's mpv and dylibbundler. The libraries are Homebrew's, so
# the app runs on the macOS they were built for and later: build releases
# on the oldest macOS to support (CI uses macos-14).
#
#   tools/bundle/macos.sh        # → target/release/bundle/dmg/Courtside_<version>_<arch>.dmg
set -euo pipefail

root=$(git -C "$(dirname "$0")" rev-parse --show-toplevel)
cd "$root/apps/desktop"

pnpm tauri build --bundles app
app="$root/target/release/bundle/macos/Courtside.app"
bin="$app/Contents/MacOS/courtside-desktop"
frameworks="$app/Contents/Frameworks"

# Copy libmpv and its dependencies in, with load paths rewritten to the
# bundle (-x the binary, -b its dependencies too, -cd/-of make the folder
# and overwrite).
dylibbundler -cd -of -b -x "$bin" -d "$frameworks" -p @executable_path/../Frameworks/ >/dev/null

# dyld refuses a binary listing the same rpath twice (Tauri adds the
# Frameworks one, dylibbundler adds it again): keep one of each.
for f in "$bin" "$frameworks"/*.dylib; do
  otool -l "$f" | awk '/cmd LC_RPATH/ {getline; getline; print $2}' | sort | uniq -d |
    while read -r dup; do install_name_tool -delete_rpath "$dup" "$f"; done
done

# Nothing may still load from Homebrew.
leaks=$(for f in "$bin" "$frameworks"/*.dylib; do otool -L "$f" | tail -n +2; done |
  grep -E '/opt/homebrew|/usr/local/(opt|Cellar|lib)' || true)
if [ -n "$leaks" ]; then
  echo "still loading from Homebrew:" >&2
  echo "$leaks" >&2
  exit 1
fi

# Where the bundled libraries come from (GPL/LGPL: their source is
# Homebrew's formulae at these versions).
licences="$app/Contents/Resources/third-party"
mkdir -p "$licences"
{
  echo "Courtside bundles these libraries for video playback (libmpv and what it loads)."
  echo "They are built by Homebrew; each one's source and licence is at https://formulae.brew.sh/formula/<name>."
  echo
  for f in "$frameworks"/*.dylib; do
    real=$(readlink -f "$(brew --prefix)/lib/$(basename "$f")" 2>/dev/null || true)
    formula=$(echo "$real" | sed -nE 's|.*/Cellar/([^/]+)/([^/]+)/.*|\1 \2|p')
    echo "$(basename "$f"): ${formula:-unknown}"
  done
} >"$licences/LIBRARIES.txt"
cp "$(brew --prefix mpv)/LICENSE.GPL" "$licences/mpv-LICENSE.GPL"

# Every changed binary needs a signature on Apple silicon: ad-hoc, inside
# out.
for f in "$frameworks"/*.dylib; do codesign --force --sign - "$f"; done
codesign --force --sign - "$bin"
codesign --force --sign - "$app"
codesign --verify --deep --strict "$app"

version=$(sed -nE 's/^version = "(.*)"/\1/p' "$root/Cargo.toml" | head -1)
arch=$(uname -m)
out="$root/target/release/bundle/dmg"
mkdir -p "$out"
stage=$(mktemp -d)
cp -R "$app" "$stage/"
ln -s /Applications "$stage/Applications"
dmg="$out/Courtside_${version}_${arch}.dmg"
rm -f "$dmg"
hdiutil create -quiet -volname Courtside -srcfolder "$stage" -ov -format UDZO "$dmg"
rm -rf "$stage"
echo "$dmg"
