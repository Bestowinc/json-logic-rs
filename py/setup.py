from setuptools import setup
from setuptools_rust import Binding, RustExtension

setup(
    name="jsonlogic",
    version="1.0",
    rust_extensions=[
        RustExtension(
            # Python package name before the dot, name of C extension to
            # stick inside of it after the dot.
            "jsonlogic.jsonlogic",
            "../Cargo.toml",
            features=["python"],
            binding=Binding.RustCPython,
        )
    ],
    packages=["jsonlogic"],
    # rust extensions are not zip safe, just like C-extensions.
    zip_safe=False,
)
