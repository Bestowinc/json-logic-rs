from pathlib import Path
from setuptools import setup
from setuptools_rust import Binding, RustExtension
from subprocess import Popen, PIPE

PKG_ROOT = Path(__file__).parent
SETUP_REQUIRES = ["setuptools-rust", "wheel", "setuptools"]
SHORT_DESCRIPTION = "JsonLogic implemented with a Rust backend"
URL = "https://www.github.com/bestowinc/json-logic-rs"
AUTHOR = "Matthew Planchard"
EMAIL = "msplanchard@gmail.com"


def generate_lockfile():
    if (PKG_ROOT / "Cargo.lock").exists():
        return
    print("Generating Cargo lockfile")
    proc = Popen(("cargo", "generate-lockfile"), stdout=PIPE, stderr=PIPE)
    _out, err = tuple(map(bytes.decode, proc.communicate()))
    if proc.returncode != 0:
        raise RuntimeError(f"Could not generate Cargo lockfile: {err}")
    return

def get_version():
    generate_lockfile()
    proc = Popen(("cargo", "pkgid"), stdout=PIPE, stderr=PIPE)
    out, err = tuple(map(bytes.decode, proc.communicate()))
    if proc.returncode != 0:
        raise RuntimeError(f"Could not get Cargo package info: {err}")
    version = out.split(":")[-1]
    return version.strip()


with open(PKG_ROOT / "README.md") as readme_f:
    LONG_DESCRIPTION = readme_f.read()

VERSION = get_version()


setup(
    name="jsonlogic-rs",
    author=AUTHOR,
    version=VERSION,
    author_email=EMAIL,
    maintainer_email=EMAIL,
    url=URL,
    description=SHORT_DESCRIPTION,
    long_description=LONG_DESCRIPTION,
    long_description_content_type="text/markdown",
    keywords=["json", "jsonlogic", "s-expressions", "rust"],
    classifiers=[
        # See https://pypi.python.org/pypi?%3Aaction=list_classifiers for all
        # available setup classifiers
        # "Development Status :: 1 - Planning",
        # 'Development Status :: 2 - Pre-Alpha',
        # "Development Status :: 3 - Alpha",
        # "Development Status :: 4 - Beta",
        'Development Status :: 5 - Production/Stable',
        # 'Development Status :: 6 - Mature',
        # 'Framework :: AsyncIO',
        # 'Framework :: Flask',
        # 'Framework :: Sphinx',
        # 'Environment :: Web Environment',
        "Intended Audience :: Developers",
        # 'Intended Audience :: End Users/Desktop',
        # 'Intended Audience :: Science/Research',
        # 'Intended Audience :: System Administrators',
        # 'License :: Other/Proprietary License',
        # 'License :: OSI Approved :: GNU General Public License v3 (GPLv3)',
        "License :: OSI Approved :: MIT License",
        # "License :: OSI Approved :: Apache Software License",
        "Natural Language :: English",
        "Operating System :: POSIX :: Linux",
        "Operating System :: MacOS :: MacOS X",
        "Operating System :: Microsoft :: Windows",
        "Programming Language :: Python",
        "Programming Language :: Python :: 3 :: Only",
        "Programming Language :: Python :: 3.6",
        "Programming Language :: Python :: 3.7",
        "Programming Language :: Python :: 3.8",
        "Programming Language :: Python :: 3.9",
        "Programming Language :: Rust",
        # 'Programming Language :: Python :: Implementation :: PyPy',
    ],
    rust_extensions=[
        RustExtension(
            # Python package name before the dot, name of C extension to
            # stick inside of it after the dot.
            "jsonlogic_rs.jsonlogic",
            "Cargo.toml",
            features=["python"],
            binding=Binding.RustCPython,
        )
    ],
    packages=["jsonlogic_rs"],
    package_dir={"": "py"},
    include_package_data=True,
    setup_requires=SETUP_REQUIRES,
    # rust extensions are not zip safe, just like C-extensions.
    zip_safe=False,
)
