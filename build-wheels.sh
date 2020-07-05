#!/usr/bin/env bash

# Expected to be run in a manylinux container

set -ex

cd /io

curl https://sh.rustup.rs -sSf | \
    sh -s -- --default-toolchain stable -y

export PATH=/root/.cargo/bin:$PATH

mkdir -p build && rm -rf build/*

for PYBIN in /opt/python/{cp36-cp36m,cp37-cp37m,cp38-cp38}/bin; do
    export PYTHON_SYS_EXECUTABLE="$PYBIN/python"

    "${PYBIN}/python" -m ensurepip
    "${PYBIN}/python" -m pip install -U setuptools wheel setuptools-rust
    "${PYBIN}/python" setup.py bdist_wheel
done

for whl in dist/*.whl; do
    auditwheel repair "$whl" -w dist/
done
