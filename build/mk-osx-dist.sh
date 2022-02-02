# http://bazaar.launchpad.net/~widelands-dev/widelands/trunk/view/head:/utils/macos/build_app.sh

V=$(git describe --tags --always --dirty)
DIST=debug
DIR=target/pod-ui-$V-osx
TOOLS_DIR=$(dirname $0)

rm -rf $DIR
rm -rf "target/pod-ui-$V-osx-unsigned.dmg"
rm -rf "target/pod-ui-$V-osx.dmg"

C=$DIR/Pod-UI.app/Contents
mkdir -p $DIR/Pod-UI.app/Contents/{Resources,MacOS}
cp gui/resources/icon.icns $C/Resources/pod-ui.icns
cat >$C/Info.plist <<EOF
{
  CFBundleName = pod-ui;
  CFBundleDisplayName = Pod-UI;
  CFBundleIdentifier = "io.github.arteme.pod-ui";
  CFBundleShortVersionString = "0.1.0";
  CFBundleVersion = "0.1.0.0";
  CFBundleInfoDictionaryVersion = "6.0";
  CFBundlePackageType = APPL;
  CFBundleSignatue = pdui;
  CFBundleExecutable = launcher.sh;
  CFBundleIconFile = pod-ui.icns;
}
EOF

$TOOLS_DIR/collect-gtk.sh $C/Resources ../Resources/
# seems like we must have the Mac keys theme too, otherwise the
# graphics do not load :(
cp -R /usr/local/share/themes/Mac $C/Resources/share/themes/

cp target/$DIST/pod-gui $C/MacOS
cp $TOOLS_DIR/osx/launcher.sh $C/MacOS

# Locate ASAN Library by asking llvm (nice trick by SirVer I suppose)
ASANLIB=$(echo "int main(void){return 0;}" |\
       	  xcrun clang -fsanitize=address -xc -o/dev/null -v - 2>&1 |\
       	  tr ' ' '\n' |\
	  grep libclang_rt.asan_osx_dynamic.dylib)
ASANPATH=`dirname $ASANLIB`

echo "Copying and fixing dynamic libraries... "
dylibbundler --create-dir --bundle-deps \
    --fix-file $C/MacOS/pod-gui \
    --dest-dir $C/libs \
    --search-path $ASANPATH

LOADERS=$(find $C/Resources/lib -type f -name '*.so')
for i in $LOADERS;
do
	echo "Processing $i ..."
	dylibbundler --create-dir --bundle-deps --overwrite-files \
	    --fix-file $i \
	    --dest-dir $C/libs \
	    --search-path $ASANPATH
done

echo "Creating a DMG file..."
hdiutil create -fs HFS+ -volname "Pod-UI $V" -srcfolder $DIR "target/pod-ui-$V-osx-unsigned.dmg"

[ "$SIGN" != "1" ] && { exit; }

source .codesign


$TOOLS_DIR/osx-sign-app.sh $DIR

echo "Creating a signed DMG file..."
hdiutil create -fs HFS+ -volname "Pod-UI $V" -srcfolder $DIR "target/pod-ui-$V-osx.dmg"

$TOOLS_DIR/osx-sign-dmg.sh "target/pod-ui-$V-osx.dmg"

xcnotary notarize "target/pod-ui-$V-osx.dmg" \
	--developer-account "$DEVELOPER" \
	--developer-password-keychain-item "$DEVELOPER_KEY"
stapler staple -v "target/pod-ui-$V-osx.dmg"

echo "!!! $DIR !!!"
