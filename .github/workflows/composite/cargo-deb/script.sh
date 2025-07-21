#!/bin/bash
set -e

echo "📦 deb: $INPUT_DEB"
echo "📦 before hook: $INPUT_BEFORE"
echo "📦 OS: $INPUT_OS"

if [[ -n "$INPUT_BEFORE" ]]; then
  eval "$INPUT_BEFORE"
fi

if [[ "$INPUT_OS" == "ubuntu-latest" || "$INPUT_OS" == "ubuntu-24.04" || "$INPUT_OS" == "ubuntu-22.04" || "$INPUT_OS" == "ubuntu-24.04-arm" || "$INPUT_OS" == "ubuntu-22.04-arm" ]]; then
  if [[ -n "$INPUT_DEB" ]]; then
    echo "🛠️ Building DEB package..."
    cargo deb --no-build --output $INPUT_DEB.deb

    if [[ -n "${GITHUB_OUTPUT:-}" ]]; then
      printf 'deb=%s\n' "${INPUT_DEB}" >>"${GITHUB_OUTPUT}"
    else
      echo "GITHUB_OUTPUT is not set; skip setting the 'archive' output"
      echo "📦 DEB archive created: $INPUT_DEB.deb"
    fi
  fi
else 
  echo "🛠️ Not running on Linux, skipping .deb archiving."
fi

echo "✅ Done."
