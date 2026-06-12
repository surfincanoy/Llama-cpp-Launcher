#!/usr/bin/env bash
set -euo pipefail

TARGET="${1:-x86_64-unknown-linux-gnu}"

cargo build --target "$TARGET" --release

ARCH=$(echo "$TARGET" | awk -F- '{print $1}')
OS=""
case "$TARGET" in
    *-windows-*) OS="win" ;;
    *-linux-*)   OS="linux" ;;
esac
SUFFIX=""
case "$TARGET" in
    *-windows-*) SUFFIX=".exe" ;;
esac

# host target uses default dir, cross-compiled uses subdir
if [ -f "target/$TARGET/release/llamacpp-launcher$SUFFIX" ]; then
    SRC="target/$TARGET/release/llamacpp-launcher$SUFFIX"
else
    SRC="target/release/llamacpp-launcher$SUFFIX"
fi

DST="$(dirname "$SRC")/llamacpp-launcher-$OS-$ARCH$SUFFIX"
cp "$SRC" "$DST"
echo "Created: $DST"
