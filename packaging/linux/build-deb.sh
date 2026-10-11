#!/bin/sh
# Builds the Al-Dir .deb for Linux Mint 22 / Ubuntu 24.04 (amd64).
#
#   cargo build --release
#   cargo run -p al-dir-pack            # browser extension
#   packaging/linux/build-deb.sh        # -> dist/al-dir_<version>_amd64.deb
#
# A Mozilla-signed extension at dist/extension/al-dir-signed.xpi is used
# for Firefox when present (Firefox refuses unsigned extensions).
set -eu

SRC=$(cd "$(dirname "$0")/../.." && pwd)
VERSION=$(sed -n 's/^version = "\(.*\)"/\1/p' "$SRC/Cargo.toml" | head -1)
ARCH=amd64
OUT="$SRC/dist/al-dir_${VERSION}_${ARCH}.deb"
ROOT=$(mktemp -d)
trap 'rm -rf "$ROOT"' EXIT

for f in al-dir al-dir-cli al-dir-agent al-dir-daemon al-dir-nm-host; do
    [ -x "$SRC/target/release/$f" ] || { echo "missing target/release/$f (cargo build --release)" >&2; exit 1; }
done
[ -f "$SRC/dist/extension/al-dir.crx" ] || { echo "missing dist/extension (cargo run -p al-dir-pack)" >&2; exit 1; }

d() { install -d -m 0755 "$ROOT$1"; }
d /usr/bin
d /usr/lib/al-dir/extension
d /usr/lib/systemd/system
d /usr/share/al-dir/lists
d /usr/share/applications
d /usr/share/icons/hicolor/scalable/apps
d /usr/share/doc/al-dir
d /etc/xdg/autostart
d /DEBIAN

install -m 0755 "$SRC/target/release/al-dir" "$SRC/target/release/al-dir-cli" "$SRC/target/release/al-dir-agent" "$ROOT/usr/bin/"
install -m 0755 "$SRC/target/release/al-dir-daemon" "$SRC/target/release/al-dir-nm-host" "$ROOT/usr/lib/al-dir/"
for f in al-dir.crx chromium-id version; do
    install -m 0644 "$SRC/dist/extension/$f" "$ROOT/usr/lib/al-dir/extension/"
done
if [ -f "$SRC/dist/extension/al-dir-signed.xpi" ]; then
    install -m 0644 "$SRC/dist/extension/al-dir-signed.xpi" "$ROOT/usr/lib/al-dir/extension/al-dir.xpi"
else
    echo "warning: no Mozilla-signed extension; Firefox will not accept it" >&2
    install -m 0644 "$SRC/dist/extension/al-dir.xpi" "$ROOT/usr/lib/al-dir/extension/al-dir.xpi"
fi
install -m 0644 "$SRC"/lists/*.txt "$ROOT/usr/share/al-dir/lists/"

sed "s|@LIBDIR@|/usr/lib/al-dir|" "$SRC/packaging/linux/al-dir.service" > "$ROOT/usr/lib/systemd/system/al-dir.service"
sed "s|@BINDIR@|/usr/bin|" "$SRC/packaging/linux/al-dir-agent.desktop" > "$ROOT/etc/xdg/autostart/al-dir-agent.desktop"
sed "s|@BINDIR@|/usr/bin|" "$SRC/packaging/linux/al-dir.desktop" > "$ROOT/usr/share/applications/al-dir.desktop"
install -m 0644 "$SRC/packaging/linux/al-dir.svg" "$ROOT/usr/share/icons/hicolor/scalable/apps/al-dir.svg"
install -m 0644 "$SRC/packaging/linux/debian/copyright" "$ROOT/usr/share/doc/al-dir/copyright"
chmod 0644 "$ROOT/usr/lib/systemd/system/al-dir.service" "$ROOT/etc/xdg/autostart/al-dir-agent.desktop" "$ROOT/usr/share/applications/al-dir.desktop"

for s in postinst prerm postrm; do
    install -m 0755 "$SRC/packaging/linux/debian/$s" "$ROOT/DEBIAN/$s"
done
echo "/etc/xdg/autostart/al-dir-agent.desktop" > "$ROOT/DEBIAN/conffiles"

SIZE=$(du -sk --exclude=DEBIAN "$ROOT" | cut -f1)
cat > "$ROOT/DEBIAN/control" <<EOF
Package: al-dir
Version: $VERSION
Architecture: $ARCH
Maintainer: Al-Dir contributors <al-dir@localhost>
Installed-Size: $SIZE
Depends: libc6 (>= 2.39), libgtk-4-1 (>= 4.14), libshumate-1.0-1, libglib2.0-0t64 | libglib2.0-0, nftables, curl, polkitd, systemd
Recommends: geoclue-2.0, network-manager
Section: admin
Priority: optional
Description: website and app blocker (Al-Dir / الدرع)
 Blocks websites and apps for chosen people and times, with locks that
 cannot be weakened until they end: time, schedule, password, random
 text, restart and location locks, plus an optional hardened mode.
 .
 Websites are blocked for every program by a DNS filter and, in Firefox
 and Chromium-based browsers, by a force-installed extension (block
 pages, URL and search-term rules, partial blocks such as YouTube
 Shorts). Apps are stopped before they start.
EOF

mkdir -p "$SRC/dist"
dpkg-deb --root-owner-group --build "$ROOT" "$OUT" >/dev/null
echo "$OUT"
