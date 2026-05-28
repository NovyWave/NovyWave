#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

VERSION="${1:-${TAURI_BUILD_VERSION:-}}"
if [ -z "$VERSION" ]; then
  echo "ERROR: version is required: scripts/package_chrome_appimage.sh <version>" >&2
  exit 1
fi

APPIMAGETOOL="${APPIMAGETOOL:-target/release/appimagetool-x86_64.AppImage}"
APPDIR="${APPDIR:-target/release/chrome-appimage/NovyWaveChrome.AppDir}"
OUTPUT="${OUTPUT:-target/release/NovyWave_${VERSION}_chrome-linux-x64.AppImage}"
PAYLOAD_DIR="${APPDIR}/usr/lib/novywave"

require_path() {
  local path="$1"
  if [ ! -e "$path" ]; then
    echo "ERROR: required path missing: $path" >&2
    exit 1
  fi
}

require_path target/release/novywave-chrome
require_path target/release/backend
require_path frontend_dist
require_path public
require_path frontend_dist/_api
require_path frontend_dist/_api/pkg
require_path frontend_dist/_api/pkg/build_id
require_path plugins/dist

if grep -R "window.__TAURI__" frontend_dist/_api/pkg >/dev/null 2>&1; then
  echo "ERROR: Chrome AppImage requires a WEB frontend, but frontend_dist contains Tauri bindings." >&2
  echo "Run: makers build && makers chrome-appimage" >&2
  exit 1
fi

if [ ! -x "$APPIMAGETOOL" ]; then
  mkdir -p "$(dirname "$APPIMAGETOOL")"
  curl -L \
    https://github.com/AppImage/AppImageKit/releases/download/continuous/appimagetool-x86_64.AppImage \
    -o "$APPIMAGETOOL"
  chmod +x "$APPIMAGETOOL"
fi

rm -rf "$APPDIR"
mkdir -p \
  "$PAYLOAD_DIR/frontend" \
  "$PAYLOAD_DIR/plugins" \
  "$APPDIR/usr/share/applications" \
  "$APPDIR/usr/share/metainfo" \
  "$APPDIR/usr/share/icons/hicolor/128x128/apps"

cp target/release/novywave-chrome "$PAYLOAD_DIR/novywave-chrome-linux-x64"
cp target/release/backend "$PAYLOAD_DIR/backend"
cp -r frontend_dist "$PAYLOAD_DIR/frontend_dist"
cp -r public "$PAYLOAD_DIR/public"
cp -r frontend_dist/_api "$PAYLOAD_DIR/_api"
cp -r frontend_dist/_api/pkg "$PAYLOAD_DIR/frontend/pkg"
cp -r plugins/dist "$PAYLOAD_DIR/plugins/dist"
chmod +x "$PAYLOAD_DIR/novywave-chrome-linux-x64" "$PAYLOAD_DIR/backend"

cp src-tauri/icons/128x128.png "$APPDIR/novywave-chrome.png"
cp src-tauri/icons/128x128.png "$APPDIR/.DirIcon"
cp src-tauri/icons/128x128.png \
  "$APPDIR/usr/share/icons/hicolor/128x128/apps/novywave-chrome.png"

cat > "$APPDIR/AppRun" <<'APPRUN'
#!/usr/bin/env bash
set -euo pipefail

HERE="$(dirname "$(readlink -f "$0")")"
cd "$HERE/usr/lib/novywave"
exec ./novywave-chrome-linux-x64 "$@"
APPRUN
chmod +x "$APPDIR/AppRun"

cat > "$APPDIR/com.novywave.chrome.desktop" <<'DESKTOP'
[Desktop Entry]
Name=NovyWave (Chrome)
Comment=Waveform viewer using Chrome/Chromium app mode
Exec=novywave-chrome-linux-x64
Icon=novywave-chrome
Type=Application
Categories=Development;Electronics;
Terminal=false
DESKTOP
cp "$APPDIR/com.novywave.chrome.desktop" \
  "$APPDIR/usr/share/applications/com.novywave.chrome.desktop"

cat > "$APPDIR/usr/share/metainfo/com.novywave.chrome.appdata.xml" <<'APPDATA'
<?xml version="1.0" encoding="UTF-8"?>
<component type="desktop-application">
  <id>com.novywave.chrome</id>
  <metadata_license>MIT</metadata_license>
  <project_license>MIT</project_license>
  <name>NovyWave (Chrome)</name>
  <summary>Waveform viewer using Chrome or Chromium app mode</summary>
  <description>
    <p>NovyWave opens digital waveform files. This package uses an installed Chromium-based browser for rendering.</p>
  </description>
  <developer id="com.novywave">
    <name>NovyWave</name>
  </developer>
  <url type="homepage">https://github.com/NovyWave/NovyWave</url>
  <launchable type="desktop-id">com.novywave.chrome.desktop</launchable>
  <content_rating type="oars-1.1" />
</component>
APPDATA

ARCH=x86_64 APPIMAGE_EXTRACT_AND_RUN=1 "$APPIMAGETOOL" "$APPDIR" "$OUTPUT"
chmod +x "$OUTPUT"

echo "[OK] Built Chrome AppImage: $OUTPUT"
