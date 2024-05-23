#!/bin/sh

NAME=`basename $(pwd)`
PACKAGE="$NAME-${GITHUB_REF_NAME:-latest}"

# build binaries
cargo build --all --release
strip target/release/wclipd

# build project structure
mkdir -p "$PACKAGE"
cp README.md "$PACKAGE/."
cp LICENSE "$PACKAGE/."
cp default-config.yaml "$PACKAGE/config.yaml"
cp -r bin "$PACKAGE/."
mv target/release/wclipd "$PACKAGE/bin/."
cp .github/install-linux.sh "$PACKAGE/install.sh"

# tar items together
tar czf "linux-amd64.tar.gz" "$PACKAGE"

