#!/bin/sh
# gutcheck installer: curl -fsSL https://raw.githubusercontent.com/sfmqrb/gutcheck/main/install.sh | sh
# Env: GUTCHECK_VERSION (e.g. v0.4.0, default latest), GUTCHECK_BIN (default $HOME/.local/bin)
set -eu

REPO=sfmqrb/gutcheck

if [ -t 1 ] && [ -z "${NO_COLOR:-}" ]; then
  B=$(printf '\033[1m'); G=$(printf '\033[32m'); Y=$(printf '\033[33m'); R=$(printf '\033[31m'); D=$(printf '\033[2m'); N=$(printf '\033[0m')
else
  B=; G=; Y=; R=; D=; N=
fi
say()  { printf '%s\n' "$*"; }
warn() { printf '%s!%s %s\n' "$Y" "$N" "$*" >&2; }
die()  { printf '%serror:%s %s\n' "$R" "$N" "$*" >&2; exit 1; }
need() { command -v "$1" >/dev/null 2>&1 || die "$1 is required but not found"; }

need curl; need tar; need uname; need mktemp

case "$(uname -s)/$(uname -m)" in
  Linux/x86_64)                target=x86_64-unknown-linux-gnu ;;
  Darwin/arm64|Darwin/aarch64) target=aarch64-apple-darwin ;;
  *) die "unsupported platform $(uname -s)/$(uname -m). Prebuilt binaries: linux x86_64, macOS arm64. Build from source: cargo install --git https://github.com/$REPO" ;;
esac

version=${GUTCHECK_VERSION:-}
if [ -z "$version" ]; then
  version=$(curl -fsSL "https://api.github.com/repos/$REPO/releases/latest" \
    | sed -n 's/.*"tag_name"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' | head -n 1) \
    || die "could not query the latest release (GitHub API rate limit? set GUTCHECK_VERSION=v0.4.0)"
  [ -n "$version" ] || die "could not determine the latest release (set GUTCHECK_VERSION=v0.4.0)"
fi
case "$version" in v*) ;; *) version=v$version ;; esac

bindir=${GUTCHECK_BIN:-$HOME/.local/bin}
name=gutcheck-$version-$target.tar.gz
base=https://github.com/$REPO/releases/download/$version

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT INT TERM

say "${B}gutcheck${N} $version for $target"
curl -fSL --progress-bar -o "$tmp/$name" "$base/$name" || die "download failed: $base/$name"

# Verify checksum when the release publishes SHA256SUMS (releases before v0.4.1 do not).
if curl -fsSL -o "$tmp/SHA256SUMS" "$base/SHA256SUMS" 2>/dev/null; then
  want=$(awk -v f="$name" '$2 == f || $2 == "*" f { print $1; exit }' "$tmp/SHA256SUMS")
  [ -n "$want" ] || die "SHA256SUMS has no entry for $name"
  if command -v sha256sum >/dev/null 2>&1; then got=$(sha256sum "$tmp/$name" | awk '{print $1}')
  elif command -v shasum >/dev/null 2>&1; then got=$(shasum -a 256 "$tmp/$name" | awk '{print $1}')
  else die "no sha256sum or shasum available to verify the download"; fi
  [ "$got" = "$want" ] || die "checksum mismatch for $name (expected $want, got $got)"
  say "${G}ok${N} sha256 verified"
else
  warn "no SHA256SUMS published for $version; skipping checksum verification"
fi

tar xzf "$tmp/$name" -C "$tmp" gutcheck || die "archive does not contain a gutcheck binary"
mkdir -p "$bindir" || die "cannot create $bindir (set GUTCHECK_BIN to a writable directory)"
[ -w "$bindir" ] || die "$bindir is not writable (set GUTCHECK_BIN to a writable directory)"
# Copy then rename so a running gutcheck is replaced atomically.
cp "$tmp/gutcheck" "$bindir/.gutcheck.new" && chmod 755 "$bindir/.gutcheck.new" && mv -f "$bindir/.gutcheck.new" "$bindir/gutcheck"

say "${G}installed${N} $bindir/gutcheck ${D}($("$bindir/gutcheck" --version 2>/dev/null || echo "$version"))${N}"

case ":$PATH:" in
  *":$bindir:"*) ;;
  *) warn "$bindir is not on your PATH. Add this to your shell profile:"
     say "    export PATH=\"$bindir:\$PATH\"" ;;
esac

say ""
say "${B}Try it:${N}  echo 'my card was charged twice' | gutcheck 'is this a billing complaint?'"
say "${D}The first run downloads a ~1.3 GB model (one time, then it is cached).${N}"
