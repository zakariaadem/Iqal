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
SRC=$(cd "$(dirname "$0")/.." && pwd)

if [ "$(id -u)" -ne 0 ]; then
    echo "Run this with sudo." >&2
    exit 1
fi

if [ "${1:-}" = "--uninstall" ]; then
    systemctl disable --now al-dir.service 2>/dev/null || true
    [ -x "$LIBDIR/al-dir-daemon" ] && "$LIBDIR/al-dir-daemon" --cleanup
    pkill -x al-dir-agent 2>/dev/null || true
    rm -f "$UNIT" "$AUTOSTART" "$BINDIR/al-dir-cli" "$BINDIR/al-dir-agent" "$LIBDIR/al-dir-daemon"
    rmdir "$LIBDIR" 2>/dev/null || true
    systemctl daemon-reload
    echo "Al-Dir removed. Settings are kept in /var/lib/al-dir."
    exit 0
fi

for f in al-dir-daemon al-dir-cli al-dir-agent; do
    if [ ! -x "$SRC/target/release/$f" ]; then
        echo "Missing target/release/$f. Run 'cargo build --release' first (without sudo)." >&2
        exit 1
    fi
done

install -d -m 0755 "$LIBDIR"
install -m 0755 "$SRC/target/release/al-dir-daemon" "$LIBDIR/al-dir-daemon"
install -m 0755 "$SRC/target/release/al-dir-cli" "$BINDIR/al-dir-cli"
install -m 0755 "$SRC/target/release/al-dir-agent" "$BINDIR/al-dir-agent"
sed "s|@BINDIR@|$BINDIR|" "$SRC/packaging/linux/al-dir-agent.desktop" > "$AUTOSTART"
chmod 0644 "$AUTOSTART"
sed "s|@LIBDIR@|$LIBDIR|" "$SRC/packaging/linux/al-dir.service" > "$UNIT"
chmod 0644 "$UNIT"

systemctl daemon-reload
systemctl enable al-dir.service >/dev/null
systemctl restart al-dir.service
systemctl --no-pager --lines=3 status al-dir.service
