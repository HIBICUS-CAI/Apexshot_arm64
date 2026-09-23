#!/usr/bin/env python3
"""Build a distro source package as static libraries, without its test tools."""
import pathlib
import re
import subprocess
import sys

source, build, prefix, *extra = sys.argv[1:]
source = pathlib.Path(source)
option_file = source / 'meson.options'
if not option_file.exists():
    option_file = source / 'meson_options.txt'
option_text = option_file.read_text() if option_file.exists() else ''
options = {}
for match in re.finditer(r"option\(\s*['\"]([^'\"]+)['\"]\s*,(.*?)\)" , option_text, re.S):
    kind = re.search(r"type\s*:\s*['\"]([^'\"]+)['\"]", match[2])
    if kind:
        options[match[1]] = kind[1]
args = ['meson', 'setup', build, str(source), f'--prefix={prefix}', '--libdir=lib',
        '--buildtype=release', '-Ddefault_library=static', '-Dprefer_static=true',
        '--wrap-mode=nofallback']
for name in ['tests', 'test', 'installed_tests', 'examples', 'docs', 'doc', 'documentation',
             'gtk_doc', 'man', 'man-pages', 'manpages', 'introspection', 'vapi',
             'build-demos', 'build-tests', 'build-testsuite', 'build-examples']:
    if options.get(name) in ('boolean', 'feature'):
        args.append(f'-D{name}=' + ('false' if options[name] == 'boolean' else 'disabled'))
subprocess.run(args + extra, check=True)
subprocess.run(['meson', 'compile', '-C', build, '-j', '4'], check=True)
subprocess.run(['meson', 'install', '-C', build], check=True)
