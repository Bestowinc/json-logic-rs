#!/usr/bin/env bash
set -euo pipefail

# Build wheels for each of the manylinux specifications.

TARGETS="quay.io/pypa/manylinux1_i686:2020-07-04-283458f "
TARGETS+="quay.io/pypa/manylinux1_x86_64:2020-07-04-283458f "
TARGETS+="quay.io/pypa/manylinux2010_i686:2020-07-04-10a3c30 "
TARGETS+="quay.io/pypa/manylinux2010_x86_64:2020-07-04-10a3c30 "
TARGETS+="quay.io/pypa/manylinux2014_i686:2020-07-04-bb5f087 "
TARGETS+="quay.io/pypa/manylinux2014_x86_64:2020-07-04-bb5f087 "

for TARGET in ${TARGETS}; do
    MANYLINUX_IMG="${TARGET}" make build-py-wheel-manylinux-no-clean;
    sleep 5;
done
