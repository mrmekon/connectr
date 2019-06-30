#!/bin/bash

DST="target/"
APPDIR="Connectr.app"

echo "Building OS X app..."

rm -rf "$DST/$APPDIR"
mkdir "$DST/$APPDIR/"
mkdir "$DST/$APPDIR/Contents/"
mkdir "$DST/$APPDIR/Contents/Resources/"
mkdir "$DST/$APPDIR/Contents/MacOS/"

cp -a target/debug/connectr "$DST/$APPDIR/Contents/MacOS/"
cp -a connectr_80px_300dpi.png "$DST/$APPDIR/Contents/Resources/"
cp -a connectr.ini.in "$DST/$APPDIR/Contents/Resources/connectr.ini"
cp -a clientid_prompt.sh "$DST/$APPDIR/Contents/Resources/clientid_prompt.sh"
chmod a+x "$DST/$APPDIR/Contents/Resources/clientid_prompt.sh"
cp -a LICENSE "$DST/$APPDIR/Contents/Resources/LICENSE.txt"

/usr/bin/strip -u -r "$DST/$APPDIR/Contents/MacOS/connectr"

cat > "$DST/$APPDIR/Contents/Info.plist" << EOF
{
   CFBundleName = connectr;
   CFBundleDisplayName = Connectr;
   CFBundleIdentifier = "com.trevorbentley.connectr";
   CFBundleExecutable = connectr;
   CFBundleIconFile = "connectr.icns";

   CFBundleVersion = "0.0.2";
   CFBundleInfoDictionaryVersion = "6.0";
   CFBundlePackageType = APPL;
   CFBundleSignature = xxxx;

   LSMinimumSystemVersion = "10.10.0";
}
EOF
echo "Done!"
