#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
WORKFLOW_FILE="$ROOT_DIR/.github/workflows/release.yml"

if [ ! -f "$WORKFLOW_FILE" ]; then
  echo "FAIL: Missing release workflow at $WORKFLOW_FILE" >&2
  exit 1
fi

if ! grep -q 'build-linux-cli:' "$WORKFLOW_FILE"; then
  echo "FAIL: release workflow must define a build-linux-cli job" >&2
  exit 1
fi

if ! grep -q 'image: swift:6.0-jammy' "$WORKFLOW_FILE"; then
  echo "FAIL: Linux release job must build inside a pinned Swift container" >&2
  exit 1
fi

if ! grep -q 'swiftc -parse-as-library -O CLI/cmux.swift -o dist/cmux' "$WORKFLOW_FILE"; then
  echo "FAIL: Linux release job must compile the CLI from CLI/cmux.swift" >&2
  exit 1
fi

if ! grep -q 'cmux-linux-x86_64.tar.gz' "$WORKFLOW_FILE"; then
  echo "FAIL: release workflow must publish a Linux tar.gz asset" >&2
  exit 1
fi

if ! grep -q 'cmux-linux-x86_64.AppImage' "$WORKFLOW_FILE"; then
  echo "FAIL: release workflow must publish a Linux AppImage asset" >&2
  exit 1
fi

echo "PASS: Linux release assets workflow is configured"
