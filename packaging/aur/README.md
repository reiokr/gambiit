# AUR release

This directory contains the AUR-facing `PKGBUILD`. Unlike
`packaging/PKGBUILD`, which builds from a locally generated source archive, this
recipe downloads the source archive attached to the GitHub release:

```text
https://github.com/reiokr/gambiit/releases/download/v0.1.0/gambiit-0.1.0.tar.gz
```

The checksum is the SHA-256 of that release asset.

## Publish to AUR

1. Add your SSH public key to your AUR account:
   <https://aur.archlinux.org/account/>

2. Clone the AUR repository:

   ```bash
   git clone ssh://aur@aur.archlinux.org/gambiit.git
   cd gambiit
   ```

3. Copy `PKGBUILD` from this directory and regenerate `.SRCINFO`:

   ```bash
   cp /path/to/gambiit/packaging/aur/PKGBUILD .
   makepkg --printsrcinfo > .SRCINFO
   ```

4. Commit and push:

   ```bash
   git add PKGBUILD .SRCINFO
   git commit -m "gambiit 0.1.0-1"
   git push
   ```

## Version bumps

After a new GitHub release:

1. Update `pkgver` and `pkgrel` in `PKGBUILD`.
2. Point `source` at the new release archive.
3. Run `updpkgsums` to refresh the checksum.
4. Regenerate `.SRCINFO` with `makepkg --printsrcinfo > .SRCINFO`.
5. Verify with `makepkg --verifysource` and, ideally, a clean-chroot build.
