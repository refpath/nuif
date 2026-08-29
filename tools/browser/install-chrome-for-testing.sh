#!/bin/sh
set -eu

destination=${1:-target/chrome-for-testing}
script_directory=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
lock_file="$script_directory/../../conformance/browser-oracle.lock.json"
version=$(sed -n 's/^[[:space:]]*"version": "\([^"]*\)",$/\1/p' "$lock_file")
if [ -z "$version" ]; then
  echo "could not read browser version from $lock_file" >&2
  exit 1
fi

case "$(uname -s):$(uname -m)" in
  Darwin:arm64)
    platform=mac-arm64
    executable="chrome-mac-arm64/Google Chrome for Testing.app/Contents/MacOS/Google Chrome for Testing"
    ;;
  Darwin:x86_64)
    platform=mac-x64
    executable="chrome-mac-x64/Google Chrome for Testing.app/Contents/MacOS/Google Chrome for Testing"
    ;;
  Linux:x86_64)
    platform=linux64
    executable=chrome-linux64/chrome
    ;;
  *)
    echo "unsupported Chrome for Testing platform: $(uname -s) $(uname -m)" >&2
    exit 1
    ;;
esac

if [ -x "$destination/$executable" ]; then
  observed=$("$destination/$executable" --version | awk '{print $NF}')
  if [ "$observed" = "$version" ]; then
    printf '%s\n' "$destination/$executable"
    exit 0
  fi
  echo "existing browser has version $observed, expected $version" >&2
  exit 1
fi

archive=$(mktemp "${TMPDIR:-/tmp}/nuif-chrome.XXXXXX.zip")
staging=$(mktemp -d "${TMPDIR:-/tmp}/nuif-chrome.XXXXXX")
cleanup() {
  rm -f "$archive"
  rm -rf "$staging"
}
trap cleanup EXIT HUP INT TERM

url=$(sed -n "s|^[[:space:]]*\"$platform\": \"\([^\"]*\)\"[,]*$|\1|p" "$lock_file")
if [ -z "$url" ]; then
  echo "could not read $platform download from $lock_file" >&2
  exit 1
fi
curl --fail --location --retry 3 --silent --show-error --output "$archive" "$url"
unzip -q "$archive" -d "$staging"
mkdir -p "$destination"
mv "$staging/chrome-$platform" "$destination/chrome-$platform"

observed=$("$destination/$executable" --version | awk '{print $NF}')
if [ "$observed" != "$version" ]; then
  echo "downloaded browser has version $observed, expected $version" >&2
  exit 1
fi
printf '%s\n' "$destination/$executable"
