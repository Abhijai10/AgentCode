#!/usr/bin/env bash
# AgentCode macOS bundle: the ONE command that puts the latest app on your Mac.
#
#   ./scripts/bundle-mac.sh          # build + DMG + install to /Applications
#   ./scripts/bundle-mac.sh --no-install   # build + DMG only
#
# Why this script exists: `tauri build` alone stops at the .app because its
# DMG step shells out to the third-party `create-dmg` tool (a fork of
# create-dmg/create-dmg), which is not installed here — every "Bundling
# AgentCode_*.dmg" attempt died with `failed to run bundle_dmg.sh`.  This
# script produces the SAME artifacts with hdiutil (present on every Mac,
# zero dependencies): a release build, the .app bundle with the ac-daemon
# sidecar inside, a compressed DMG, and an /Applications install.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
APP_NAME="AgentCode"
APP_PATH="$ROOT/target/release/bundle/macos/${APP_NAME}.app"
DMG_PATH="$ROOT/target/release/bundle/dmg/${APP_NAME}_0.1.0_aarch64.dmg"
STAGE="$(mktemp -d)"
trap 'rm -rf "$STAGE"' EXIT

step() { printf '\n\033[1;34m▶ %s\033[0m\n' "$1"; }

step "1/5 Release build (daemon + desktop shell)"
cargo build --release -p ac-daemon
(cd "$ROOT/apps/desktop" && pnpm exec tauri build --bundles app)

step "2/5 Bundle the ac-daemon sidecar into the .app"
# The shell looks for ac-daemon BESIDE its own binary first (F7 sidecar);
# copy it in so the packaged app never depends on the workspace target dir.
cp "$ROOT/target/release/ac-daemon" "$APP_PATH/Contents/MacOS/ac-daemon"

step "3/5 Stage the DMG folder (app + /Applications symlink)"
cp -R "$APP_PATH" "$STAGE/"
ln -s /Applications "$STAGE/Applications"

step "4/5 Create the DMG (hdiutil — no create-dmg dependency)"
mkdir -p "$(dirname "$DMG_PATH")"
rm -f "$DMG_PATH"
hdiutil create \
  -volname "$APP_NAME" \
  -srcfolder "$STAGE" \
  -ov \
  -format UDZO \
  "$DMG_PATH"
echo "  DMG: $DMG_PATH ($(du -h "$DMG_PATH" | cut -f1 | tr -d ' '))"

if [[ "${1:-}" == "--no-install" ]]; then
  step "5/5 Skipped install (--no-install)"
  echo "Done. Drag $APP_NAME from the DMG into Applications, or re-run without --no-install."
  exit 0
fi

step "5/5 Install to /Applications (replaces the old version)"
# Close any running instance first so the swap cannot race a live binary.
osascript -e 'quit app "AgentCode"' >/dev/null 2>&1 || true
sleep 1
rm -rf "/Applications/${APP_NAME}.app"
cp -R "$APP_PATH" "/Applications/${APP_NAME}.app"
echo "  Installed: /Applications/${APP_NAME}.app"
echo
echo "Done — the latest AgentCode is on your Mac."
echo "  First launch: right-click the app in /Applications → Open (bypasses the"
echo "  unsigned-developer warning once), or run:  open /Applications/${APP_NAME}.app"
