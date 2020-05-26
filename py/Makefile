SUPPORTED_PYTHONS := 3.6 3.7 3.8
BUILD_REQUIRES := wheel setuptools setuptools-rust

PYVER := $(if $(PYVER),$(PYVER),3)
VENV = venv$(PYVER)
VENV_TARGET = $(VENV)/bin/activate
SRC_VENV = source $(VENV_TARGET);

INSTALL_REQS := pip install -e .


.PHONY: build build-all $(SUPPORTED_PYTHONS) distribute distribute-clean


venv: $(VENV_TARGET)
$(VENV_TARGET): setup.py
	python$(PYVER) -m venv $(VENV)
	$(SRC_VENV) $(INSTALL_REQS)
	touch $(VENV_TARGET)

venv-clean:
	rm -rf $(VENV)

venv-refresh: venv-clean venv


develop: clean venv
	$(SRC_VENV) pip install -e .


build: clean venv-clean venv build-clean
	$(SRC_VENV) pip install -U setuptools setuptools-rust wheel
	$(SRC_VENV) python setup.py bdist_wheel

build-all: $(SUPPORTED_PYTHONS)

$(SUPPORTED_PYTHONS): export PYVER = $@
$(SUPPORTED_PYTHONS):
	$(MAKE) -C . build


build-linux: clean venv-clean build-clean
	docker run \
		--rm \
		-it \
		--mount type=bind,source="$(PWD)",target="/src" \
		-w "/src" \
		python:$(PYVER) \
		bash -c "make build-all"

build-linux-all: linux-$(SUPPORTED_PYTHONS)
linux-$(SUPPORTED_PYTHONS): export PYVER = $(subst linux-,,$@)
linux-$(SUPPORTED_PYTHONS):
	$(MAKE) -C . build-linux



build-clean:
	find . -type f -name "*.so" -delete
	rm -rf build build
	rm -rf pip-wheel-metadata
	rm -rf src/*.egg-info

clean:
	find . -type f -name "*.py[co]" -delete
	find . -type d -name "__pycache__" -delete

# Requires VERSION to be set on the CLI or in an environment variable,
# e.g. make VERSION=1.0.0 distribute
distribute: build
	$(SRC_VENV) pip install twine
	$(VENV) twine upload -s dist/*


distribute-clean:
	rm -rf dist/


test-3.8:
	docker run --rm -it --mount type=bind,source="$(PWD)",target="/src" -w "/src" \
		python:3.8 bash -c "make clean && pip install -e .[dev] && $(TEST); make clean"


test-all-versions: test-3.6 test-3.7 test-3.8

