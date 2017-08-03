#!/bin/bash

APP="connectr"
DST="target/"
APPDIR="Connectr.app"
RELEASE=`git describe --abbrev=0`

echo "Building OS X app $RELEASE..."

cargo run --release && pkill "$APP"

strip "$DST/$APPDIR/Contents/MacOS/$APP"
(cd "$DST" && zip -r9 "$RELEASE.zip" "$APPDIR" && md5 "$RELEASE.zip" > "$RELEASE.md5")

echo "Done!"
