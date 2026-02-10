#!/usr/bin/env bash

set -euo pipefail

usage() {
  cat <<'EOF'
Generate a Homebrew formula for prebuilt vartui binaries.

Usage:
  scripts/homebrew/generate-formula.sh \
    --owner <github-owner> \
    --repo <github-repo> \
    --tag <release-tag> \
    --sha-arm64 <sha256> \
    --sha-x86_64 <sha256> \
    [--output <path>]

Example:
  scripts/homebrew/generate-formula.sh \
    --owner dsanchezp \
    --repo vartui \
    --tag v0.1.0 \
    --sha-arm64 abc123... \
    --sha-x86_64 def456... \
    --output Formula/vartui.rb
EOF
}

owner=""
repo=""
tag=""
sha_arm64=""
sha_x86_64=""
output=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --owner)
      owner="${2:-}"
      shift 2
      ;;
    --repo)
      repo="${2:-}"
      shift 2
      ;;
    --tag)
      tag="${2:-}"
      shift 2
      ;;
    --sha-arm64)
      sha_arm64="${2:-}"
      shift 2
      ;;
    --sha-x86_64)
      sha_x86_64="${2:-}"
      shift 2
      ;;
    --output)
      output="${2:-}"
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "Unknown argument: $1" >&2
      usage >&2
      exit 1
      ;;
  esac
done

if [[ -z "$owner" || -z "$repo" || -z "$tag" || -z "$sha_arm64" || -z "$sha_x86_64" ]]; then
  echo "Missing required arguments." >&2
  usage >&2
  exit 1
fi

version="${tag#v}"
url_base="https://github.com/${owner}/${repo}/releases/download/${tag}"

formula_content="class Vartui < Formula
  desc \"Terminal timesheet TUI and JSON CLI for VAR\"
  homepage \"https://github.com/${owner}/${repo}\"
  version \"${version}\"

  on_macos do
    on_arm do
      url \"${url_base}/vartui-${tag}-darwin-arm64.tar.gz\"
      sha256 \"${sha_arm64}\"
    end

    on_intel do
      url \"${url_base}/vartui-${tag}-darwin-x86_64.tar.gz\"
      sha256 \"${sha_x86_64}\"
    end
  end

  def install
    bin.install \"vartui\"
  end

  test do
    assert_match \"api <subcomando>\", shell_output(\"#{bin}/vartui --help\")
  end
end"

if [[ -n "$output" ]]; then
  mkdir -p "$(dirname "$output")"
  printf '%s\n' "$formula_content" > "$output"
  echo "Formula written to $output"
else
  printf '%s\n' "$formula_content"
fi
