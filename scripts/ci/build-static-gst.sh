#!/usr/bin/env bash
set -euo pipefail
prefix=/opt/apexshot-static/gst
work="$RUNNER_TEMP/gst-static-source"
mkdir -p "$work" "$prefix/share/static-sources"
cd "$work"
git clone --depth 1 --branch 1.28.2 https://github.com/GStreamer/gstreamer.git source
git -C source rev-parse HEAD > "$prefix/share/static-sources/gstreamer-commit.txt"
apt-get source orc
orc_source=$(find "$work" -maxdepth 1 -type d -name 'orc-*' | head -n 1)
python3 "$GITHUB_WORKSPACE/scripts/ci/meson-static.py" "$orc_source" "$work/build-orc" "$prefix"
export PKG_CONFIG_PATH="$prefix/lib/pkgconfig"
args=(
  --prefix="$prefix" --libdir=lib --buildtype=release --wrap-mode=nofallback
  -Ddefault_library=static -Dprefer_static=true -Dauto_features=disabled
  -Dgst-full=enabled -Dgst-full-target-type=static_library '-Dgst-full-libraries=*'
  -Dbase=enabled -Dgood=enabled -Dbad=enabled -Dugly=enabled -Dlibav=enabled
  -Dgpl=enabled -Dtools=enabled -Dorc=enabled -Dorc-source=system
  -Dbuild-tools-source=system
)
for plugin in app audioconvert audiomixer audiorate audioresample audiotestsrc \
  compositor encoding gio playback rawparse subparse typefind videoconvertscale \
  videorate videotestsrc volume ogg opus vorbis theora x11 xshm xvideo gl drm; do
  args+=("-Dgst-plugins-base:$plugin=enabled")
done
for plugin in pulseaudio vpx isomp4 matroska avi wavparse wavenc png jpeg \
  videobox videocrop videofilter videomixer ximagesrc; do
  args+=("-Dgst-plugins-good:$plugin=enabled")
done
for plugin in audioparsers videoparsers webrtcdsp voaacenc openh264 autoconvert; do
  args+=("-Dgst-plugins-bad:$plugin=enabled")
done
args+=(-Dgst-plugins-ugly:x264=enabled)
meson setup "$work/build" source "${args[@]}"
meson compile -C "$work/build" -j 4
meson install -C "$work/build"
test -f "$prefix/lib/libgstreamer-full-1.0.a"
export GST_PLUGIN_SYSTEM_PATH_1_0=''
export GST_PLUGIN_PATH_1_0=''
export GST_REGISTRY="$work/static-registry.bin"
"$prefix/bin/gst-inspect-1.0" | tee "$GITHUB_WORKSPACE/verification/gst-static-plugins.txt"
for element in appsink appsrc ximagesrc videoconvert videoscale x264enc vp9enc \
  theoraenc mp4mux webmmux pulsesrc audiomixer audioconvert audioresample \
  voaacenc aacparse opusenc oggmux webrtcdsp; do
  "$prefix/bin/gst-inspect-1.0" "$element" > /dev/null
done
ldd "$prefix/bin/gst-inspect-1.0" | tee "$GITHUB_WORKSPACE/verification/gst-static-ldd.txt"
! grep -E 'libgst[^ /]*\.so' "$GITHUB_WORKSPACE/verification/gst-static-ldd.txt"
