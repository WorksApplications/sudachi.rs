from setuptools import setup
from setuptools_rust import Binding, RustExtension

setup(
    name="sudachi",
    version="0.1",
    rust_extensions=[RustExtension("sudachi.sudachi", binding=Binding.PyO3)],
    packages=["sudachi"],
    package_dir={"": "py_src"},
    # rust extensions are not zip safe, just like C-extensions.
    zip_safe=False,
)
