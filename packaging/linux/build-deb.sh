#!/bin/sh
# Builds the Iqal .deb for Linux Mint 22 / Ubuntu 24.04 (amd64).
#
#   cargo build --release
#   cargo run -p iqal-pack            # browser extension
#   packaging/linux/build-deb.sh        # -> dist/iqal_<version>_amd64.deb
#
# A Mozilla-signed extension at dist/extension/iqal-signed.xpi is used
# for Firefox when present (Firefox refuses unsigned extensions).
set -eu

SRC=$(cd "$(dirname "$0")/../.." && pwd)
VERSION=$(sed -n 's/^version = "\(.*\)"/\1/p' "$SRC/Cargo.toml" | head -1)
ARCH=amd64
OUT="$SRC/dist/iqal_${VERSION}_${ARCH}.deb"
ROOT=$(mktemp -d)
trap 'rm -rf "$ROOT"' EXIT

for f in iqal iqal-cli iqal-agent iqal-daemon iqal-nm-host; do
    [ -x "$SRC/target/release/$f" ] || { echo "missing target/release/$f (cargo build --release)" >&2; exit 1; }
done
[ -f "$SRC/dist/extension/iqal.crx" ] || { echo "missing dist/extension (cargo run -p iqal-pack)" >&2; exit 1; }

d() { install -d -m 0755 "$ROOT$1"; }
d /usr/bin
d /usr/lib/iqal/extension
d /usr/lib/systemd/system
d /usr/share/iqal/lists
d /usr/share/icons/hicolor/scalable/actions
d /usr/share/applications
d /usr/share/icons/hicolor/scalable/apps
d /usr/share/doc/iqal
d /etc/xdg/autostart
d /DEBIAN

install -m 0755 "$SRC/target/release/iqal" "$SRC/target/release/iqal-cli" "$SRC/target/release/iqal-agent" "$ROOT/usr/bin/"
install -m 0755 "$SRC/target/release/iqal-daemon" "$SRC/target/release/iqal-nm-host" "$ROOT/usr/lib/iqal/"
for f in iqal.crx chromium-id version; do
    install -m 0644 "$SRC/dist/extension/$f" "$ROOT/usr/lib/iqal/extension/"
done
if [ -f "$SRC/dist/extension/iqal-signed.xpi" ]; then
    install -m 0644 "$SRC/dist/extension/iqal-signed.xpi" "$ROOT/usr/lib/iqal/extension/iqal.xpi"
else
    echo "warning: no Mozilla-signed extension; Firefox will not accept it" >&2
    install -m 0644 "$SRC/dist/extension/iqal.xpi" "$ROOT/usr/lib/iqal/extension/iqal.xpi"
fi
install -m 0644 "$SRC"/lists/*.txt "$ROOT/usr/share/iqal/lists/"
# Icons the desktop's icon set may not have, in the standard folder (so
# they are coloured like the others).
install -m 0644 "$SRC"/packaging/linux/icons/*.svg "$ROOT/usr/share/icons/hicolor/scalable/actions/"

sed "s|@LIBDIR@|/usr/lib/iqal|" "$SRC/packaging/linux/iqal.service" > "$ROOT/usr/lib/systemd/system/iqal.service"
sed "s|@BINDIR@|/usr/bin|" "$SRC/packaging/linux/iqal-agent.desktop" > "$ROOT/etc/xdg/autostart/iqal-agent.desktop"
sed "s|@BINDIR@|/usr/bin|" "$SRC/packaging/linux/iqal.desktop" > "$ROOT/usr/share/applications/iqal.desktop"
install -m 0644 "$SRC/packaging/linux/iqal.svg" "$ROOT/usr/share/icons/hicolor/scalable/apps/iqal.svg"
install -m 0644 "$SRC/packaging/linux/debian/copyright" "$ROOT/usr/share/doc/iqal/copyright"
chmod 0644 "$ROOT/usr/lib/systemd/system/iqal.service" "$ROOT/etc/xdg/autostart/iqal-agent.desktop" "$ROOT/usr/share/applications/iqal.desktop"

for s in preinst postinst prerm postrm; do
    install -m 0755 "$SRC/packaging/linux/debian/$s" "$ROOT/DEBIAN/$s"
done
echo "/etc/xdg/autostart/iqal-agent.desktop" > "$ROOT/DEBIAN/conffiles"

SIZE=$(du -sk --exclude=DEBIAN "$ROOT" | cut -f1)
cat > "$ROOT/DEBIAN/control" <<EOF
Package: iqal
Version: $VERSION
Architecture: $ARCH
Maintainer: Iqal contributors <iqal@localhost>
Installed-Size: $SIZE
Depends: libc6 (>= 2.39), libgtk-4-1 (>= 4.14), libglib2.0-0t64 | libglib2.0-0, nftables, curl, polkitd, systemd
Recommends: geoclue-2.0, network-manager
Conflicts: al-dir
Replaces: al-dir
Section: admin
Priority: optional
Description: website and app blocker (Iqal / عِقال)
 Blocks websites and apps for chosen people and times, with locks that
 cannot be weakened until they end: time, schedule, password, random
 text, restart and location locks, plus an optional hardened mode.
 .
 Websites are blocked for every program by a DNS filter and, in Firefox
 and Chromium-based browsers, by a force-installed extension (block
 pages, URL and search-term rules). Apps are stopped before they start.
EOF

mkdir -p "$SRC/dist"
dpkg-deb --root-owner-group --build "$ROOT" "$OUT" >/dev/null
echo "$OUT"
