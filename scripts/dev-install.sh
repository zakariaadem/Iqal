#!/bin/sh
# Installs the locally built Iqal as a system service (development use;
# real packages come later).
#
#   cargo build --release
#   sudo scripts/dev-install.sh             # install or update
#   sudo scripts/dev-install.sh --uninstall
set -eu

PREFIX=/usr/local
LIBDIR=$PREFIX/lib/iqal
BINDIR=$PREFIX/bin
UNIT=/etc/systemd/system/iqal.service
AUTOSTART=/etc/xdg/autostart/iqal-agent.desktop
APPS=$PREFIX/share/applications/iqal.desktop
ICON=$PREFIX/share/icons/hicolor/scalable/apps/iqal.svg
LISTS=$PREFIX/share/iqal/lists
SRC=$(cd "$(dirname "$0")/.." && pwd)

if [ "$(id -u)" -ne 0 ]; then
    echo "Run this with sudo." >&2
    exit 1
fi

if [ "${1:-}" = "--uninstall" ]; then
    # Refuses (and stops this script) while a block is locked.
    if [ -x "$LIBDIR/iqal-daemon" ]; then "$LIBDIR/iqal-daemon" --cleanup; fi
    systemctl disable --now iqal.service 2>/dev/null || true
    pkill -x iqal-agent 2>/dev/null || true
    rm -f "$UNIT" "$AUTOSTART" "$APPS" "$ICON" "$BINDIR/iqal" "$BINDIR/iqal-cli" "$BINDIR/iqal-agent" "$LIBDIR/iqal-daemon" "$LIBDIR/iqal-nm-host"
    rm -rf "$LIBDIR/extension"
    rm -rf "$PREFIX/share/iqal"
    rmdir "$LIBDIR" 2>/dev/null || true
    systemctl daemon-reload
    echo "Iqal removed. Settings are kept in /var/lib/iqal."
    exit 0
fi

for f in iqal-daemon iqal-cli iqal-agent iqal-nm-host iqal; do
    if [ ! -x "$SRC/target/release/$f" ]; then
        echo "Missing target/release/$f. Run 'cargo build --release' first (without sudo)." >&2
        exit 1
    fi
done

install -d -m 0755 "$LIBDIR"
install -m 0755 "$SRC/target/release/iqal-daemon" "$LIBDIR/iqal-daemon"
install -m 0755 "$SRC/target/release/iqal-nm-host" "$LIBDIR/iqal-nm-host"
if [ -f "$SRC/dist/extension/iqal.crx" ]; then
    install -d -m 0755 "$LIBDIR/extension"
    for f in iqal.xpi iqal.crx chromium-id version; do
        install -m 0644 "$SRC/dist/extension/$f" "$LIBDIR/extension/$f"
    done
    # A Mozilla-signed build replaces the unsigned one when present.
    if [ -f "$SRC/dist/extension/iqal-signed.xpi" ]; then
        install -m 0644 "$SRC/dist/extension/iqal-signed.xpi" "$LIBDIR/extension/iqal.xpi"
    fi
fi
install -m 0755 "$SRC/target/release/iqal-cli" "$BINDIR/iqal-cli"
install -d -m 0755 "$LISTS"
install -m 0644 "$SRC"/lists/*.txt "$LISTS"/
install -m 0755 "$SRC/target/release/iqal-agent" "$BINDIR/iqal-agent"
install -m 0755 "$SRC/target/release/iqal" "$BINDIR/iqal"
install -d -m 0755 "$(dirname "$APPS")" "$(dirname "$ICON")"
sed "s|@BINDIR@|$BINDIR|" "$SRC/packaging/linux/iqal.desktop" > "$APPS"
chmod 0644 "$APPS"
install -m 0644 "$SRC/packaging/linux/iqal.svg" "$ICON"
gtk-update-icon-cache -q "$PREFIX/share/icons/hicolor" 2>/dev/null || true
sed "s|@BINDIR@|$BINDIR|" "$SRC/packaging/linux/iqal-agent.desktop" > "$AUTOSTART"
chmod 0644 "$AUTOSTART"
sed "s|@LIBDIR@|$LIBDIR|" "$SRC/packaging/linux/iqal.service" > "$UNIT"
chmod 0644 "$UNIT"

systemctl daemon-reload
systemctl enable iqal.service >/dev/null
systemctl restart iqal.service
systemctl --no-pager --lines=3 status iqal.service
