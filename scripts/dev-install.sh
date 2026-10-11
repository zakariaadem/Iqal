#!/bin/sh
# Installs the locally built Al-Dir as a system service (development use;
# real packages come later).
#
#   cargo build --release
#   sudo scripts/dev-install.sh             # install or update
#   sudo scripts/dev-install.sh --uninstall
set -eu

PREFIX=/usr/local
LIBDIR=$PREFIX/lib/al-dir
BINDIR=$PREFIX/bin
UNIT=/etc/systemd/system/al-dir.service
AUTOSTART=/etc/xdg/autostart/al-dir-agent.desktop
APPS=$PREFIX/share/applications/al-dir.desktop
ICON=$PREFIX/share/icons/hicolor/scalable/apps/al-dir.svg
SRC=$(cd "$(dirname "$0")/.." && pwd)

if [ "$(id -u)" -ne 0 ]; then
    echo "Run this with sudo." >&2
    exit 1
fi

if [ "${1:-}" = "--uninstall" ]; then
    # Refuses (and stops this script) while a block is locked.
    if [ -x "$LIBDIR/al-dir-daemon" ]; then "$LIBDIR/al-dir-daemon" --cleanup; fi
    systemctl disable --now al-dir.service 2>/dev/null || true
    pkill -x al-dir-agent 2>/dev/null || true
    rm -f "$UNIT" "$AUTOSTART" "$APPS" "$ICON" "$BINDIR/al-dir" "$BINDIR/al-dir-cli" "$BINDIR/al-dir-agent" "$LIBDIR/al-dir-daemon" "$LIBDIR/al-dir-nm-host"
    rm -rf "$LIBDIR/extension"
    rmdir "$LIBDIR" 2>/dev/null || true
    systemctl daemon-reload
    echo "Al-Dir removed. Settings are kept in /var/lib/al-dir."
    exit 0
fi

for f in al-dir-daemon al-dir-cli al-dir-agent al-dir-nm-host al-dir; do
    if [ ! -x "$SRC/target/release/$f" ]; then
        echo "Missing target/release/$f. Run 'cargo build --release' first (without sudo)." >&2
        exit 1
    fi
done

install -d -m 0755 "$LIBDIR"
install -m 0755 "$SRC/target/release/al-dir-daemon" "$LIBDIR/al-dir-daemon"
install -m 0755 "$SRC/target/release/al-dir-nm-host" "$LIBDIR/al-dir-nm-host"
if [ -f "$SRC/dist/extension/al-dir.crx" ]; then
    install -d -m 0755 "$LIBDIR/extension"
    for f in al-dir.xpi al-dir.crx chromium-id version; do
        install -m 0644 "$SRC/dist/extension/$f" "$LIBDIR/extension/$f"
    done
    # A Mozilla-signed build replaces the unsigned one when present.
    if [ -f "$SRC/dist/extension/al-dir-signed.xpi" ]; then
        install -m 0644 "$SRC/dist/extension/al-dir-signed.xpi" "$LIBDIR/extension/al-dir.xpi"
    fi
fi
install -m 0755 "$SRC/target/release/al-dir-cli" "$BINDIR/al-dir-cli"
install -m 0755 "$SRC/target/release/al-dir-agent" "$BINDIR/al-dir-agent"
install -m 0755 "$SRC/target/release/al-dir" "$BINDIR/al-dir"
install -d -m 0755 "$(dirname "$APPS")" "$(dirname "$ICON")"
sed "s|@BINDIR@|$BINDIR|" "$SRC/packaging/linux/al-dir.desktop" > "$APPS"
chmod 0644 "$APPS"
install -m 0644 "$SRC/packaging/linux/al-dir.svg" "$ICON"
gtk-update-icon-cache -q "$PREFIX/share/icons/hicolor" 2>/dev/null || true
sed "s|@BINDIR@|$BINDIR|" "$SRC/packaging/linux/al-dir-agent.desktop" > "$AUTOSTART"
chmod 0644 "$AUTOSTART"
sed "s|@LIBDIR@|$LIBDIR|" "$SRC/packaging/linux/al-dir.service" > "$UNIT"
chmod 0644 "$UNIT"

systemctl daemon-reload
systemctl enable al-dir.service >/dev/null
systemctl restart al-dir.service
systemctl --no-pager --lines=3 status al-dir.service
