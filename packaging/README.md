# Packaging

Gambiit ships as a native Tauri binary. The Arch/CachyOS package installs:

- `/usr/bin/gambiit`
- `/usr/share/applications/gambiit.desktop`
- hicolor icons in 32, 128, 256, 512 and scalable sizes
- the MIT license, README and `docs/` guides

Stockfish is an optional runtime dependency, not bundled: it is GPL software and
the app discovers the system binary. OpenCode is the AI opponent; either the
`opencode` CLI or `opencode-desktop` package works.

## Build the package

From the repository root:

```bash
./packaging/build-package.sh
```

This creates a source tarball, pins its checksum in a temporary PKGBUILD and
runs `makepkg`. The resulting `gambiit-*.pkg.tar.zst` is copied to `packaging/`.
To build and install in one step:

```bash
./packaging/build-package.sh -i
```

Build requirements: `base-devel`, `rust` (includes cargo), `nodejs`, `pnpm`,
`pkgconf`, `webkit2gtk-4.1`, `gtk3` and `libsoup3`.

The package itself depends only on the runtime libraries. `opencode`,
`opencode-desktop` and `stockfish` are optional because the app can start and
show a clear prompt when the engine or the agent is unavailable.

## Manual makepkg

```bash
./packaging/make-source.sh
cd packaging
makepkg -si
```

The committed PKGBUILD uses `sha256sums=('SKIP')` for local builds.
`build-package.sh` pins the real checksum automatically. For an AUR release,
upload the tarball, point `source=` at the release URL and run `updpkgsums`.
The `url=` field points to the public repository.

## AUR

`packaging/aur/` contains a release-based `PKGBUILD` for AUR. It downloads the
source archive attached to the GitHub release and verifies its SHA-256 checksum.
See [`aur/README.md`](aur/README.md) for publishing and version-bump steps.

## Release binary

Tauri's regular bundle target is a Debian package:

```bash
pnpm tauri build
```

For a standalone binary without a bundle:

```bash
pnpm tauri build --no-bundle
# target/release/gambiit-app
```
