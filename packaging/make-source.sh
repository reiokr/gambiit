#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
pkgbuild="$root/packaging/PKGBUILD"
version="$(sed -n 's/^version = "\(.*\)"/\1/p' "$root/src-tauri/Cargo.toml" | head -n 1)"
pkgver="$(sed -n 's/^pkgver=//p' "$pkgbuild" | head -n 1)"

if [[ -z "$version" || "$version" != "$pkgver" ]]; then
  echo "error: src-tauri/Cargo.toml version '$version' does not match PKGBUILD pkgver '$pkgver'" >&2
  exit 1
fi

out="$root/packaging/gambiit-$version.tar.gz"
rm -f "$out"

tar \
  --exclude-vcs \
  --exclude='./target' \
  --exclude='./node_modules' \
  --exclude='./dist' \
  --exclude='./src-tauri/gen' \
  --exclude='./packaging/build' \
  --exclude='./packaging/gambiit-*.tar.gz' \
  --exclude='./packaging/*.pkg.tar.*' \
  --transform "s,^\.,gambiit-$version," \
  -czf "$out" -C "$root" .

echo "$out"
