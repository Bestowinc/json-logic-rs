#!/usr/bin/env bash
set -euo pipefail

DIST_VERSION=$(pip search jsonlogic-rs | grep -e '^jsonlogic-rs (' | awk '{print $2}' | sed 's/[\(\)]//g')

CURRENT_VERSION=$(cargo pkgid | tr ':' ' ' | awk '{print $3}')

if [[ "${CURRENT_VERSION}" == "${DIST_VERSION}" ]]; then
    echo false
    exit 0
else
    echo true
    exit 0
fi
