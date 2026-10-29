#!/bin/sh
# Упаковывает бинарник в самораспаковывающийся .sh для Linux.
#   scripts/make-sh.sh <бинарник> <выход.sh>
set -eu
BIN=$1
OUT=$2
HASH=$(sha256sum "$BIN" | cut -c1-12)
cat > "$OUT" <<HEADER
#!/bin/sh
# Escape From Podval 2 для Linux. Игра лежит внутри этого скрипта:
# при первом запуске она распаковывается в кэш и дальше запускается оттуда.
set -e
DIR="\${XDG_CACHE_HOME:-\$HOME/.cache}/escape-from-podval-2/$HASH"
GAME="\$DIR/escape-from-podval-2"
if [ ! -x "\$GAME" ]; then
    mkdir -p "\$DIR"
    tail -n +__SKIP__ "\$0" > "\$GAME.tmp"
    chmod +x "\$GAME.tmp"
    mv "\$GAME.tmp" "\$GAME"
fi
exec "\$GAME" "\$@"
HEADER
SKIP=$(($(wc -l < "$OUT") + 1))
sed -i "s/__SKIP__/$SKIP/" "$OUT"
cat "$BIN" >> "$OUT"
chmod +x "$OUT"
