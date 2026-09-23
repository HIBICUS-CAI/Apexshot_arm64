#!/usr/bin/env bash
set -euo pipefail
source /etc/os-release
test "$VERSION_ID" = 26.04
test "$(uname -m)" = aarch64
printf 'System: %s %s\n' "$PRETTY_NAME" "$(uname -m)"
packages=(gtk4 gtk4-layer-shell-0 gstreamer-1.0 gstreamer-app-1.0 gstreamer-video-1.0 libpipewire-0.3 tesseract Qt5Widgets Qt5DBus Qt5X11Extras Qt5Network)
pkg-config --modversion "${packages[@]}"
python3 - "${packages[@]}" <<'PY'
import pathlib, shlex, subprocess, sys
for package in sys.argv[1:]:
    result = subprocess.run(['pkg-config', '--static', '--libs', package], text=True, capture_output=True)
    print(f'\n[{package}] {result.stdout.strip()}')
    if result.returncode:
        print(result.stderr)
        continue
    flags = shlex.split(result.stdout)
    dirs = [x[2:] for x in flags if x.startswith('-L')]
    dirs += ['/usr/lib/aarch64-linux-gnu', '/usr/lib', '/lib/aarch64-linux-gnu']
    for name in dict.fromkeys(x[2:] for x in flags if x.startswith('-l')):
        archives = [str(pathlib.Path(d) / f'lib{name}.a') for d in dirs if (pathlib.Path(d) / f'lib{name}.a').exists()]
        print(f'  {name}: {archives[0] if archives else "NO STATIC ARCHIVE"}')
PY
