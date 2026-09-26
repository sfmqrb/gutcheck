#!/bin/sh
# Rewrite Formula/gutcheck.rb in the homebrew-tap checkout for a release.
# Usage: packaging/update-tap.sh v0.4.1 [path/to/homebrew-tap]   (tap defaults to ../homebrew-tap)
set -eu
ver=${1:?usage: update-tap.sh vX.Y.Z [tap-dir]}
case "$ver" in v*) ;; *) ver=v$ver ;; esac
tap=${2:-$(dirname "$0")/../../homebrew-tap}
[ -d "$tap/Formula" ] || { echo "no Formula/ dir in $tap" >&2; exit 1; }
tmp=$(mktemp -d); trap 'rm -rf "$tmp"' EXIT
base=https://github.com/sfmqrb/gutcheck/releases/download/$ver
sha() {
  curl -fsSL -o "$tmp/$1" "$base/gutcheck-$ver-$1.tar.gz"
  if command -v sha256sum >/dev/null; then sha256sum "$tmp/$1"; else shasum -a 256 "$tmp/$1"; fi | cut -d' ' -f1
}
mac=$(sha aarch64-apple-darwin); lin=$(sha x86_64-unknown-linux-gnu)
cat > "$tap/Formula/gutcheck.rb" <<RB
class Gutcheck < Formula
  desc "System-1 grep: filter, score and classify text streams with a natural-language question, locally"
  homepage "https://github.com/sfmqrb/gutcheck"
  version "${ver#v}"
  license "MIT"

  on_macos do
    on_arm do
      url "$base/gutcheck-$ver-aarch64-apple-darwin.tar.gz"
      sha256 "$mac"
    end
  end

  on_linux do
    on_intel do
      url "$base/gutcheck-$ver-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "$lin"
    end
  end

  def install
    bin.install "gutcheck"
  end

  def caveats
    <<~EOS
      The first run downloads a ~1.3 GB model to \$XDG_CACHE_HOME/gutcheck.
    EOS
  end

  test do
    assert_match "gutcheck", shell_output("#{bin}/gutcheck --version")
  end
end
RB
echo "updated $tap/Formula/gutcheck.rb to $ver; now: cd tap && git commit -am 'gutcheck $ver' && git push"
