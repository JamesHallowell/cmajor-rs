VERSION=${1}

if [ -z "$VERSION" ]; then
  echo "Usage: $0 <version>"
  exit 1
fi

curl -L "https://github.com/SoundStacks/cmajor/releases/download/${VERSION}/cmajor.linux.x64.zip" -o cmajor.zip
tempdir=$(mktemp -d)
unzip -q cmajor.zip -d "$tempdir"
mv "$tempdir"/linux/x64/libCmajPerformer.so ./libCmajPerformer.so
