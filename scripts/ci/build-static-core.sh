#!/usr/bin/env bash
set -euo pipefail
prefix=/opt/apexshot-static/core
builder="$GITHUB_WORKSPACE/scripts/ci/meson-static.py"
work="$RUNNER_TEMP/core-static-source"
mkdir -p "$work" "$prefix/share/static-sources"
export PKG_CONFIG_PATH="$prefix/lib/pkgconfig"
cd "$work"
apt-get source pixman cairo harfbuzz pango1.0 gdk-pixbuf graphene gtk4 gtk4-layer-shell graphite2
cp ./*.dsc "$prefix/share/static-sources/"
source_dir() { find "$work" -maxdepth 1 -type d -name "$1-*" | sort | head -n 1; }
build() {
  local name="$1"
  shift
  python3 "$builder" "$(source_dir "$name")" "$work/build-$name" "$prefix" "$@"
}
# Graphite is a dependency of HarfBuzz for Graphite fonts.
cmake -S "$(source_dir graphite2)" -B "$work/build-graphite2" -G Ninja \
  -DCMAKE_BUILD_TYPE=Release -DCMAKE_INSTALL_PREFIX="$prefix" \
  -DCMAKE_INSTALL_LIBDIR=lib -DBUILD_SHARED_LIBS=OFF -DGRAPHITE2_COMPARE_RENDERER=OFF
cmake --build "$work/build-graphite2" --parallel 4
cmake --install "$work/build-graphite2"
build pixman
build cairo
build harfbuzz
build pango1.0
# Built-in raster loaders avoid separate plugin files; SVG is checked separately.
build gdk-pixbuf -Dglycin=disabled -Dbuiltin_loaders=all -Dpng=enabled -Djpeg=enabled -Dgif=enabled
build graphene

# GTK builds an internal static archive but hard-codes its public target as shared.
# Honor default_library for that target and retain its link_whole dependencies.
python3 - "$(source_dir gtk4)/gtk/meson.build" <<'PY'
import pathlib, sys
p = pathlib.Path(sys.argv[1])
s = p.read_text()
old = "libgtk = shared_library('gtk-4',"
assert s.count(old) == 1, 'GTK target changed; review the static patch'
p.write_text(s.replace(old, "libgtk = library('gtk-4',"))
PY
build gtk4
build gtk4-layer-shell
test -f "$prefix/lib/libgtk-4.a"
test -f "$prefix/lib/libgtk4-layer-shell.a"
pkg-config --static --libs gtk4 gtk4-layer-shell-0
