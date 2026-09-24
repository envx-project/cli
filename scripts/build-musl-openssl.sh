#!/bin/sh
set -eu

# OpenSSL 3.5 LTS is supported through April 2030. Verify the upstream release
# archive before building; use musl's own headers instead of mixing in glibc.
version=3.5.8
sha256=a8f84a39918ec6415ce765d9b429d313ba97b8143169c172e734b9514464f5b2
prefix=${1:?Usage: build-musl-openssl.sh ABSOLUTE_INSTALL_PREFIX}
case "$prefix" in /*) ;; *) echo 'Install prefix must be absolute' >&2; exit 2 ;; esac
build_dir=$(mktemp -d)
trap 'rm -rf "$build_dir"' EXIT HUP INT TERM
cd "$build_dir"
archive="openssl-$version.tar.gz"
curl --fail --silent --show-error --location --proto '=https' --proto-redir '=https' \
  "https://github.com/openssl/openssl/releases/download/openssl-$version/$archive" -o "$archive"
printf '%s  %s\n' "$sha256" "$archive" | sha256sum -c -
tar xzf "$archive"
cd "openssl-$version"
# envx does not initialize OpenSSL's optional secure heap; disabling it avoids
# Linux-only mman headers absent from musl-tools without adding glibc headers.
CC=musl-gcc ./Configure linux-x86_64 no-shared no-module no-async no-afalgeng no-secure-memory no-tests \
  --prefix="$prefix" --libdir=lib --openssldir=/etc/ssl
make -j"${BUILD_JOBS:-$(getconf _NPROCESSORS_ONLN)}"
make install_sw
