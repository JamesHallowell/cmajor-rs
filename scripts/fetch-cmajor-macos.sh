VERSION=${1}

if [ -z "$VERSION" ]; then
  echo "Usage: $0 <version>"
  exit 1
fi

curl -L "https://github.com/SoundStacks/cmajor/releases/download/${VERSION}/cmajor.dmg" -o cmajor.dmg
tempdir=$(mktemp -d)
hdiutil attach cmajor.dmg -mountpoint "$tempdir"
cp "$tempdir"/libCmajPerformer.dylib ./libCmajPerformer.dylib
hdiutil detach "$tempdir"
rm -rf "$tempdir"