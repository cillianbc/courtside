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

# Third-party notices (GPL/LGPL: their source must be available). Each
# bundled library's Homebrew formula, version, licence and source tarball,
# the licence files the formulae install, and a written offer
# (docs/THIRD-PARTY.md).
licences="$app/Contents/Resources/third-party"
mkdir -p "$licences"
formulas=$(for f in "$frameworks"/*.dylib; do
  readlink -f "$(brew --prefix)/lib/$(basename "$f")" 2>/dev/null |
    sed -nE 's|.*/Cellar/([^/]+)/.*|\1|p'
done | sort -u)
# shellcheck disable=SC2086
brew info --json=v2 --formula $formulas | python3 -I -c '
import json, sys
d = json.load(sys.stdin)
print("Courtside: third-party libraries bundled for video playback")
print("=" * 58)
print()
print("Courtside (AGPL-3.0-only) plays video with libmpv, which loads the libraries")
print("below. They are unmodified Homebrew builds. Each one is listed with its licence")
print("and the exact source it was built from. The licence texts are in this folder.")
print()
print("Written offer: for three years from the release of this copy of Courtside, the")
print("Courtside project will provide the complete corresponding source code of these")
print("libraries on request, at no more than the cost of providing it. Write to the")
print("address in https://github.com/cillianbc/courtside/blob/main/docs/THIRD-PARTY.md.")
print()
for f in sorted(d["formulae"], key=lambda f: f["name"]):
    installed = f.get("installed") or [{}]
    v = installed[0].get("version") or f["versions"]["stable"]
    print("%s %s" % (f["name"], v))
    if v.split("_")[0] != f["versions"]["stable"]:
        # The formula moved on since this was installed: its source URL is
        # for another version. Release builds install fresh (CI).
        sys.stderr.write("notices: %s %s installed, formula at %s: brew upgrade\n"
                         % (f["name"], v, f["versions"]["stable"]))
        print("  note:    built from %s; the source below is for %s"
              % (v, f["versions"]["stable"]))
    print("  licence: %s" % (f.get("license") or "see the source"))
    print("  source:  %s" % f["urls"]["stable"]["url"])
    print("  formula: https://formulae.brew.sh/formula/%s" % f["name"])
' >"$licences/THIRD-PARTY-NOTICES.txt"
for formula in $formulas; do
  for lic in "$(brew --prefix "$formula")"/{COPYING,LICENSE,LICENCE}*; do
    [ -f "$lic" ] && cp "$lic" "$licences/$formula-$(basename "$lic")"
  done
done
cp "$root/LICENSE" "$licences/Courtside-LICENSE"

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
