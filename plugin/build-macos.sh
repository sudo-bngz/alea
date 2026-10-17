#!/usr/bin/env bash
# Builds alea's plugin for macOS (Apple silicon and Intel in one bundle) and
# installs it where Ableton Live and other hosts look for plugins.
#
# Needs: the Xcode command line tools (`xcode-select --install`) and Rust
# (https://rustup.rs). Run from anywhere:  ./plugin/build-macos.sh
set -euo pipefail
cd "$(dirname "$0")"

rustup target add aarch64-apple-darwin x86_64-apple-darwin
cargo xtask bundle-universal alea_plugin --profile plugin

install_bundle() {
  local bundle="$1" dest="$2"
  mkdir -p "$dest"
  rm -rf "$dest/$(basename "$bundle")"
  cp -R "$bundle" "$dest/"
  # a local signature, so macOS loads it without complaint
  codesign --force --deep --sign - "$dest/$(basename "$bundle")"
  echo "installed $(basename "$bundle") in $dest"
}
install_bundle ../target/bundled/alea.vst3 "$HOME/Library/Audio/Plug-Ins/VST3"
install_bundle ../target/bundled/alea.clap "$HOME/Library/Audio/Plug-Ins/CLAP"

cat <<'NEXT'

Next, in Ableton Live:
  1. Settings → Plug-Ins: turn on "Use VST3 Plug-in System Folders", then Rescan.
  2. Drop "alea" (Plug-ins → VST3) on a MIDI track; open its window and choose a work.
  3. Make a second MIDI track with an instrument. Set its "MIDI From" to the
     alea track and, below it, to "alea"; set Monitor to "In".
  4. Press play: the work plays in time with Live.
NEXT
