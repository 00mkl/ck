#!/bin/sh
# ck-web.app: ck-web as macOS runs Chromium, in an app with Chromium's framework and the
# helper apps Chromium starts its other processes from (copies of ck-web, which see they're
# helpers by their `--type` switch). Run from the repository after
# `cargo build --release -p ck-web`; the app lands next to ck-web (another build folder:
# its path, like `target/debug`). Signed ad hoc, as Apple Silicon wants every program.
set -eu
dir=${1:-target/release}
framework=$(find "$dir/build" -maxdepth 5 -type d -name 'Chromium Embedded Framework.framework' | head -n 1)
if [ -z "$framework" ]; then
    echo "Chromium's framework isn't in $dir/build: build ck-web first" >&2
    exit 1
fi
version=$(sed -n 's/^version = "\(.*\)"$/\1/p' ck-web/Cargo.toml | head -n 1)
app="$dir/ck-web.app"
rm -rf "$app"

# make_app NAME FOLDER: NAME.app in FOLDER, running a copy of ck-web called NAME. None of
# them shows in the Dock.
make_app() {
    contents="$2/$1.app/Contents"
    mkdir -p "$contents/MacOS"
    cp "$dir/ck-web" "$contents/MacOS/$1"
    id=$(printf '%s' "$1" | tr -c 'A-Za-z0-9' '-')
    cat > "$contents/Info.plist" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleExecutable</key><string>$1</string>
  <key>CFBundleIdentifier</key><string>io.github.gold-silver-copper.ck.$id</string>
  <key>CFBundleName</key><string>$1</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleVersion</key><string>$version</string>
  <key>CFBundleShortVersionString</key><string>$version</string>
  <key>LSMinimumSystemVersion</key><string>11.0</string>
  <key>LSUIElement</key><true/>
  <key>LSEnvironment</key><dict><key>MallocNanoZone</key><string>0</string></dict>
</dict>
</plist>
EOF
}

make_app ck-web "$dir"
mkdir -p "$app/Contents/Frameworks"
cp -R "$framework" "$app/Contents/Frameworks/"
for kind in "" " (GPU)" " (Renderer)" " (Plugin)" " (Alerts)"; do
    make_app "ck-web Helper$kind" "$app/Contents/Frameworks"
done
codesign --force --deep --sign - "$app"
echo "$app"
