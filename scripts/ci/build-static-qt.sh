#!/usr/bin/env bash
set -euo pipefail
prefix=/opt/apexshot-static/qt
work="$RUNNER_TEMP/qt-static-source"
mkdir -p "$work"
cd "$work"
apt-get source qtbase-opensource-src qtx11extras-opensource-src
base=$(find "$work" -maxdepth 1 -type d -name 'qtbase-opensource-src-*' | head -n 1)
extras=$(find "$work" -maxdepth 1 -type d -name 'qtx11extras-opensource-src-*' | head -n 1)
mkdir -p "$work/build-base"
cd "$work/build-base"
"$base/configure" -prefix "$prefix" -opensource -confirm-license -release -static \
  -nomake examples -nomake tests -qt-zlib -qt-libpng -qt-libjpeg -qt-freetype \
  -qt-harfbuzz -no-icu -dbus-linked
make -j4
make install
mkdir -p "$work/build-extras"
cd "$work/build-extras"
"$prefix/bin/qmake" "$extras"
make -j4
make install
