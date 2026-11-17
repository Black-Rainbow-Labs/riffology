#!/usr/bin/env sh
# Downloads the guitar model library pinned in tools/guitar-abi.json, checks both SHA-256 hashes, and unpacks it into
# vendor/guitar-abi/<platform>/ (docs/guitar-model.md). The shell version of fetch-guitar-abi.ps1, for macOS and Linux.
# Needs curl, unzip, node and shasum or sha256sum.
set -eu
root="$(cd "$(dirname "$0")/.." && pwd)"
pin="$root/tools/guitar-abi.json"

case "$(uname -s)-$(uname -m)" in
  Darwin-arm64) platform=macos-arm64 ;;
  Darwin-*) platform=macos-x64 ;;
  *) platform=linux-x64 ;;
esac

field() { node -e "const p=require(process.argv[1]);const e=p.platforms[process.argv[2]];if(!e){process.exit(3)};process.stdout.write(String(process.argv[3]in e?e[process.argv[3]]:p[process.argv[3]]))" "$pin" "$platform" "$1"; }
zipname="$(field zip)" || { echo "tools/guitar-abi.json has no library for $platform yet. See docs/guitar-model.md, Platforms." >&2; exit 1; }
lib="$(field library)"; libsha="$(field sha256)"; zipsha="$(field zipSha256)"; tag="$(field tag)"; repo="$(field repo)"
hash() { if command -v sha256sum >/dev/null; then sha256sum "$1" | cut -d' ' -f1; else shasum -a 256 "$1" | cut -d' ' -f1; fi; }

dest="$root/vendor/guitar-abi/$platform"
if [ -f "$dest/$lib" ] && [ "$(hash "$dest/$lib")" = "$libsha" ]; then echo "The pinned library is already in $dest"; exit 0; fi

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
curl -L --fail --retry 4 -o "$tmp/$zipname" "https://github.com/$repo/releases/download/$tag/$zipname"
[ "$(hash "$tmp/$zipname")" = "$zipsha" ] || { echo "$zipname does not match the pinned SHA-256" >&2; exit 1; }
unzip -q "$tmp/$zipname" -d "$tmp/unpacked"
[ "$(hash "$tmp/unpacked/$lib")" = "$libsha" ] || { echo "$lib does not match the pinned SHA-256" >&2; exit 1; }
mkdir -p "$dest"
cp "$tmp/unpacked/"* "$dest/"
echo "Fetched $lib into $dest"
