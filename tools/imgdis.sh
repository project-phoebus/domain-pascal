#!/bin/sh
# tools/imgdis.sh - disassemble a byte range of the SR10.2 SAU2 kernel image
#
# usage: tools/imgdis.sh <VA-hex> <LEN-hex>      e.g.  tools/imgdis.sh E00EEC 2A
#
# The image is an Apollo RFC file loaded at 0x101400; the kernel proper
# (OS_PROC, VA 0xE00800) starts at file offset 0xC00, and the whole loaded
# part satisfies  file offset = VA - 0xDFFC00  (see ../source/docs/rfc-cold-start.md).
# Valid for VAs 0xE00800 .. 0xE88834 (the file ends at RELOC); bss/NOBITS
# regions beyond that are not in the file.
#
# Reading only; writes a temporary file under tmp/ (git-ignored).
set -e
IMG=${DOMAIN_OS_IMAGE:-$HOME/src/domainos-archeology/sr10.2-install/install/ri.apollo.os.v.10.2/sau2/domain_os}
[ $# -eq 2 ] || { echo "usage: $0 <VA-hex> <LEN-hex>" >&2; exit 2; }
VA=$((0x$1)); LEN=$((0x$2)); OFF=$((VA - 0xDFFC00))
HERE=$(cd "$(dirname "$0")/.." && pwd)
mkdir -p "$HERE/tmp"
BIN="$HERE/tmp/imgdis.$$.bin"
dd if="$IMG" of="$BIN" bs=1 skip=$OFF count=$LEN 2>/dev/null
m68k-elf-objdump -D -b binary -m m68k:68020 --adjust-vma=$VA "$BIN" | sed -n '8,$p'
rm -f "$BIN"
