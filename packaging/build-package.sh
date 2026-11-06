#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
version="$(sed -n 's/^version = "\(.*\)"/\1/p' "$root/src-tauri/Cargo.toml" | head -n 1)"
source_archive="$root/packaging/gambiit-$version.tar.gz"
builddir="$root/packaging/build"

if ! command -v makepkg >/dev/null 2>&1; then
  echo "error: makepkg was not found. Install base-devel first." >&2
  exit 1
fi

"$root/packaging/make-source.sh"

rm -rf "$builddir"
mkdir -p "$builddir"
cp "$root/packaging/PKGBUILD" "$builddir/"
cp "$source_archive" "$builddir/"

sum="$(sha256sum "$source_archive" | awk '{print $1}')"
sed -i "s/sha256sums=('SKIP')/sha256sums=('$sum')/" "$builddir/PKGBUILD"

(
  cd "$builddir"
  makepkg -f "$@"
)

package="$(find "$builddir" -maxdepth 1 -name '*.pkg.tar.*' -print -quit)"
if [[ -z "$package" ]]; then
  echo "error: makepkg finished without producing a package" >&2
  exit 1
fi
cp -v "$package" "$root/packaging/"
echo "Package written to $root/packaging/"
