#!/usr/bin/env bash
set -euo pipefail

CURRENT_VERSION=$(cargo pkgid | tr '#' '\n' | tail -n 1 | tr ':' ' ' | awk '{print $2}')

RESP=$(curl 'https://crates.io/api/v1/crates/jsonlogic-rs' -s \
    -H 'User-Agent: mplanchard_verison_check (msplanchard@gmail.com)' \
    -H 'Accept: application/json' \
    -H 'Cache-Control: max-age=0')

PREV_VERSION=$(echo "${RESP}" \
    | tr ',' '\n' \
    | grep newest_version \
    | tr ':' ' ' \
    | awk '{print $2}' \
    | sed 's/"//g')

if [[ "${CURRENT_VERSION}" == "${PREV_VERSION}" ]]; then
    echo false
    exit 0
else
    echo true
    exit 0
fi
