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
SRC=$(cd "$(dirname "$0")/.." && pwd)

if [ "$(id -u)" -ne 0 ]; then
    echo "Run this with sudo." >&2
    exit 1
fi

if [ "${1:-}" = "--uninstall" ]; then
    systemctl disable --now al-dir.service 2>/dev/null || true
    rm -f "$UNIT" "$BINDIR/al-dir-cli" "$LIBDIR/al-dir-daemon"
    rmdir "$LIBDIR" 2>/dev/null || true
    systemctl daemon-reload
    echo "Al-Dir removed. Settings are kept in /var/lib/al-dir."
    exit 0
fi

for f in al-dir-daemon al-dir-cli; do
    if [ ! -x "$SRC/target/release/$f" ]; then
        echo "Missing target/release/$f. Run 'cargo build --release' first (without sudo)." >&2
        exit 1
    fi
done

install -d -m 0755 "$LIBDIR"
install -m 0755 "$SRC/target/release/al-dir-daemon" "$LIBDIR/al-dir-daemon"
install -m 0755 "$SRC/target/release/al-dir-cli" "$BINDIR/al-dir-cli"
sed "s|@LIBDIR@|$LIBDIR|" "$SRC/packaging/linux/al-dir.service" > "$UNIT"
chmod 0644 "$UNIT"

systemctl daemon-reload
systemctl enable al-dir.service >/dev/null
systemctl restart al-dir.service
systemctl --no-pager --lines=3 status al-dir.service
