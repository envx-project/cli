#!/bin/sh
set -eu

usage() {
  cat <<'HELP'
Install a published envx release.

  -f, -y, --force, --yes   Skip confirmation
  -b, --bin-dir DIR       Installation directory (default /usr/local/bin)
  -a, --arch ARCH         Override architecture
  -p, --platform TARGET   Override platform suffix
  -B, --base-url URL      HTTPS release mirror
  -V, --verbose           Show download details
  -r, --remove            Remove the envx executable found on PATH
  -h, --help              Show this help without network access

Environment: ENVX_VERSION, ENVX_BIN_DIR, ENVX_ARCH, ENVX_PLATFORM, ENVX_BASE_URL.
ENVX_VERSION selects an existing release; the default is the latest published release.
HELP
}

fail() { printf 'envx: %s\n' "$*" >&2; exit 1; }
info() { printf '%s\n' "$*"; }
has() { command -v "$1" >/dev/null 2>&1; }

BIN_DIR=${ENVX_BIN_DIR:-/usr/local/bin}
BASE_URL=${ENVX_BASE_URL:-https://github.com/envx-project/cli/releases}
VERSION=${ENVX_VERSION:-}
ARCH=${ENVX_ARCH:-}
PLATFORM=${ENVX_PLATFORM:-}
FORCE=
VERBOSE=
REMOVE=
while [ "$#" -gt 0 ]; do
  case "$1" in
    -h|--help) usage; exit 0 ;;
    -f|-y|--force|--yes) FORCE=1; shift ;;
    -f=*|-y=*|--force=*|--yes=*) FORCE=${1#*=}; shift ;;
    -V|--verbose) VERBOSE=1; shift ;;
    -V=*|--verbose=*) VERBOSE=${1#*=}; shift ;;
    -r|--remove|--uninstall) REMOVE=1; shift ;;
    -b|--bin-dir|-B|--base-url|-a|--arch|-p|--platform)
      [ "$#" -ge 2 ] && [ -n "$2" ] || fail "Missing value for $1"
      case "$1" in
        -b|--bin-dir) BIN_DIR=$2 ;;
        -B|--base-url) BASE_URL=$2 ;;
        -a|--arch) ARCH=$2 ;;
        -p|--platform) PLATFORM=$2 ;;
      esac
      shift 2 ;;
    -b=*|--bin-dir=*) BIN_DIR=${1#*=}; shift ;;
    -B=*|--base-url=*) BASE_URL=${1#*=}; shift ;;
    -a=*|--arch=*) ARCH=${1#*=}; shift ;;
    -p=*|--platform=*) PLATFORM=${1#*=}; shift ;;
    *) fail "Unknown option: $1" ;;
  esac
done

confirm() {
  if [ -z "$FORCE" ] && [ -t 0 ]; then
    printf '%s [y/N] ' "$1"
    read -r answer || fail 'Could not read confirmation; use --yes'
    case "$answer" in y|yes) ;; *) fail 'Cancelled' ;; esac
  fi
}

# Never probe with a predictable filename: it may already belong to the user.
writeable() {
  probe=$(mktemp "$1/.envx-write.XXXXXX" 2>/dev/null) || return 1
  rm -f "$probe"
}

privileged() {
  if writeable "$BIN_DIR"; then
    "$@"
  else
    has sudo || fail "Cannot write $BIN_DIR; choose a writable --bin-dir or install sudo"
    sudo "$@"
  fi
}

if [ -n "$REMOVE" ]; then
  executable=$(command -v envx) || fail 'envx is not on PATH'
  [ -f "$executable" ] || fail 'envx on PATH is not an executable file'
  BIN_DIR=$(dirname "$executable")
  confirm "Remove $executable?"
  privileged rm -f "$executable"
  info 'Removed envx'
  exit 0
fi

[ -d "$BIN_DIR" ] || fail "Installation directory does not exist: $BIN_DIR"
case "$BASE_URL" in https://*) ;; *) fail 'Release downloads require an HTTPS base URL' ;; esac
BASE_URL=${BASE_URL%/}
has curl || fail 'curl is required to download releases'

if [ -z "$ARCH" ]; then
  ARCH=$(uname -m | tr '[:upper:]' '[:lower:]')
  case "$ARCH" in amd64) ARCH=x86_64 ;; arm64) ARCH=aarch64 ;; esac
  if [ "$ARCH" = x86_64 ] && [ "$(getconf LONG_BIT)" = 32 ]; then ARCH=i686; fi
fi
if [ -z "$PLATFORM" ]; then
  PLATFORM=$(uname -s | tr '[:upper:]' '[:lower:]')
  case "$PLATFORM" in
    linux) PLATFORM=unknown-linux-musl ;;
    darwin) PLATFORM=apple-darwin ;;
    msys_nt*|cygwin_nt*|mingw*) PLATFORM=pc-windows-msvc ;;
  esac
fi
TARGET=$ARCH-$PLATFORM
case "$TARGET" in
  x86_64-unknown-linux-gnu|x86_64-unknown-linux-musl|x86_64-apple-darwin|aarch64-apple-darwin|x86_64-pc-windows-msvc|x86_64-pc-windows-gnu) ;;
  *) fail "No published build for $TARGET; see https://github.com/envx-project/cli/releases" ;;
esac

# Restrict both the initial request and redirects to HTTPS.
fetch() {
  [ -z "$VERBOSE" ] || info "Downloading $2"
  curl --fail --silent --show-error --location --proto '=https' --proto-redir '=https' --tlsv1.2 --output "$1" "$2"
}
if [ -z "$VERSION" ]; then
  latest=$(curl --fail --silent --show-error --location --proto '=https' --proto-redir '=https' --tlsv1.2 --output /dev/null --write-out '%{url_effective}' "$BASE_URL/latest")
  VERSION=${latest##*/}
fi
VERSION=${VERSION#v}
printf '%s\n' "$VERSION" | grep -Eq '^[0-9]+\.[0-9]+\.[0-9]+([+-][0-9A-Za-z.-]+)?$' || fail "Invalid release version: $VERSION"

EXT=tar.gz
BINARY=envx
case "$PLATFORM" in pc-windows-*) EXT=zip; BINARY=envx.exe ;; esac
ASSET=envx-$VERSION-$TARGET.$EXT
URL=$BASE_URL/download/v$VERSION/$ASSET
confirm "Install envx $VERSION to $BIN_DIR?"

work_dir=$(mktemp -d "${TMPDIR:-/tmp}/envx-install.XXXXXX")
trap 'rm -rf "$work_dir"' 0
trap 'exit 1' HUP INT TERM
fetch "$work_dir/$ASSET" "$URL" || fail 'Release download failed'

# Older published releases predate checksum sidecars. Never silently downgrade
# verification for a new release or when a checksum exists but does not match.
if fetch "$work_dir/checksum" "$URL.sha256"; then
  expected=$(awk 'NR == 1 { print $1 }' "$work_dir/checksum")
  printf '%s\n' "$expected" | grep -Eq '^[0-9a-fA-F]{64}$' || fail 'Invalid SHA256 checksum'
  if has sha256sum; then
    actual=$(sha256sum "$work_dir/$ASSET" | awk '{print $1}')
  elif has shasum; then
    actual=$(shasum -a 256 "$work_dir/$ASSET" | awk '{print $1}')
  else
    fail 'sha256sum or shasum is required to verify this release'
  fi
  [ "$(printf '%s' "$expected" | tr 'A-F' 'a-f')" = "$actual" ] || fail 'Release checksum mismatch; installation aborted'
else
  major=${VERSION%%.*}
  rest=${VERSION#*.}
  minor=${rest%%.*}
  if [ "$major" -lt 2 ] || { [ "$major" -eq 2 ] && [ "$minor" -le 13 ]; }; then
    printf 'Warning: legacy release %s has no SHA256 sidecar; relying on HTTPS transport.\n' "$VERSION" >&2
  else
    fail 'Release checksum unavailable; installation aborted'
  fi
fi

# Extract only the named executable to stdout. Archive paths, permissions, and
# symlinks must never be unpacked directly into a privileged installation folder.
case "$EXT" in
  tar.gz) tar -xzOf "$work_dir/$ASSET" "$BINARY" > "$work_dir/$BINARY" ;;
  zip) unzip -p "$work_dir/$ASSET" "$BINARY" > "$work_dir/$BINARY" ;;
esac
[ -s "$work_dir/$BINARY" ] || fail 'Release archive contains no executable'
privileged install -m 755 "$work_dir/$BINARY" "$BIN_DIR/$BINARY"
info "Installed envx $VERSION to $BIN_DIR/$BINARY"
case ":$PATH:" in *":$BIN_DIR:"*) ;; *) info "Add $BIN_DIR to PATH to run envx." ;; esac
