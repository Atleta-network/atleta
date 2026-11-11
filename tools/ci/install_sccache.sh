#!/bin/sh
set -eu
destination="${1:-/usr/local/bin}"
case "$(uname -s):$(uname -m)" in
  Linux:x86_64)
    architecture=x86_64
    checksum=aa73c7270c997894038c0d8f0d297865f57569e1687883322bc431713d4bb9dd ;;
  Linux:aarch64)
    architecture=aarch64
    checksum=3e47fc4a71b1ddd1b4294a0de9d34498922fdf41e3cd0bda5df3954e3d6deb9d ;;
  *) echo 'Unsupported sccache builder architecture' >&2; exit 1 ;;
esac
temporary="$(mktemp -d)"
trap 'rm -rf "$temporary"' EXIT
archive="sccache-v0.19.0-${architecture}-unknown-linux-musl"
curl --fail --location --silent --show-error \
  "https://github.com/mozilla/sccache/releases/download/v0.19.0/$archive.tar.gz" \
  --output "$temporary/archive.tgz"
printf '%s  %s\n' "$checksum" "$temporary/archive.tgz" | sha256sum -c - >/dev/null
tar -xzf "$temporary/archive.tgz" -C "$temporary" "$archive/sccache"
mkdir -p "$destination"
cp "$temporary/$archive/sccache" "$destination/sccache"
chmod 0755 "$destination/sccache"
