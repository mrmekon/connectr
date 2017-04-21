#!/bin/bash

DST="target/"
APPDIR="Connectr.app"

echo "Building OS X app..."

rm -rf "$DST/$APPDIR"
mkdir "$DST/$APPDIR/"
mkdir "$DST/$APPDIR/Contents/"
mkdir "$DST/$APPDIR/Contents/Resources/"
mkdir "$DST/$APPDIR/Contents/MacOS/"

cp -a target/release/connectr $DST/Connectr.app/Contents/MacOS/
cp -a spotify.png $DST/Connectr.app/Contents/Resources/
cp -a connectr.ini.in $DST/Connectr.app/Contents/Resources/connectr.ini

strip -u -r $DST/Connectr.app/Contents/MacOS/connectr

cat > "$DST/$APPDIR/Contents/Info.plist" << EOF
{
   CFBundleName = connectr;
   CFBundleDisplayName = Connectr;
   CFBundleIdentifier = "com.trevorbentley.connectr";
   CFBundleExecutable = connectr;
   CFBundleIconFile = "connectr.icns";

   CFBundleVersion = "1.0";
   CFBundleInfoDictionaryVersion = "6.0";
   CFBundlePackageType = APPL;
   CFBundleSignature = xxxx;

   LSMinimumSystemVersion = "10.10.0";
}
EOF
echo "Done!"
