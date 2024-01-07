#!/bin/bash

cargo build --release --target x86_64-apple-darwin
cargo build --release --target aarch64-apple-darwin

rm -rf Clicker.app
mkdir -p Clicker.app/Contents/MacOS
mkdir -p Clicker.app/Contents/Resources
mkdir -p Clicker.app/Contents/MacOS/assets
cp assets/*.png Clicker.app/Contents/MacOS/assets

cp assets/Clicker.icns Clicker.app/Contents/Resources
cat > Clicker.app/Contents/Info.plist << EOF
{
   CFBundleName = Clicker;
   CFBundleDisplayName = Clicker;
   CFBundleIdentifier = "org.wubo.clicker";
   CFBundleVersion = "1.0.0";
   CFBundleShortVersionString = "1.0.0";
   CFBundleInfoDictionaryVersion = "6.0";
   CFBundlePackageType = APPL;
   CFBundleSignature = wdld;
   CFBundleExecutable = clicker;
   CFBundleIconFile = "Clicker.icns";
}
EOF

lipo "target/x86_64-apple-darwin/release/bevy_clicker2" \
     "target/aarch64-apple-darwin/release/bevy_clicker2" \
     -create -output Clicker.app/Contents/MacOS/clicker