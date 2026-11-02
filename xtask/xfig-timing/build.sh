#!/bin/sh
# Build xfig 3.2.9a with timing hooks (XFIG_TIMING=1 prints spans to stderr), for
# `cargo xtask speed-compare`. Same configure flags and dependencies as Homebrew's formula
# (Xaw3d, Xft, libpng, jpeg-turbo, libtiff, libxpm, ghostscript; Homebrew's app-defaults dir),
# CFLAGS -O2 as Homebrew's default (+ -std=gnu17: xfig is C89/gnu17 code, newer compilers default to C23). xfig's own sources are only read (copied), never changed.
# Usage: xtask/xfig-timing/build.sh [WORKDIR]   (default target/xfig-timing); result: WORKDIR/src/src/xfig
set -e
here=$(cd "$(dirname "$0")" && pwd)
root=$(cd "$here/../.." && pwd)
work=${1:-$root/target/xfig-timing}
orig=${XFIG_SRC:-$root/old/xfig-3.2.9a}
export PATH=/opt/homebrew/opt/gnu-sed/libexec/gnubin:/opt/homebrew/bin:$PATH
export PKG_CONFIG_PATH=/opt/homebrew/lib/pkgconfig:/opt/homebrew/share/pkgconfig
rm -rf "$work"; mkdir -p "$work"
cp -R "$orig" "$work/src"
cd "$work/src"
patch -p1 < "$here/xfig-timing.patch"
CFLAGS="-O2 -std=gnu17 -I/opt/homebrew/opt/freetype/include/freetype2 -I/opt/homebrew/include" \
LDFLAGS="-L/opt/homebrew/lib" \
./configure --with-appdefaultdir=/opt/homebrew/etc/X11/app-defaults --prefix="$work/inst" --disable-silent-rules > ../configure.log 2>&1
make -j"$(sysctl -n hw.ncpu)" > ../make.log 2>&1
echo "built $work/src/src/xfig"
