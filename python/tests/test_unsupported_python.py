import importlib
import sys
import sysconfig
import unittest
from pathlib import Path


@unittest.skipUnless(
    sys.version_info[:2] == (3, 13)
    and sysconfig.get_config_var("Py_GIL_DISABLED"),
    "requires a free-threaded Python 3.13 build",
)
class TestUnsupportedPython(unittest.TestCase):
    def test_import_raises_explicit_error(self):
        package_source = Path(__file__).resolve().parents[1] / "py_src"
        sys.path.insert(0, str(package_source))
        self.addCleanup(sys.path.remove, str(package_source))

        with self.assertRaisesRegex(
            ImportError,
            "SudachiPy does not support Python 3.13 free-threaded",
        ):
            importlib.import_module("sudachipy")
