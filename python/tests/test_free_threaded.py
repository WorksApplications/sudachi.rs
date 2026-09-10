import os
import subprocess
import sys
import sysconfig
import textwrap
import unittest


@unittest.skipUnless(
    sysconfig.get_config_var("Py_GIL_DISABLED"),
    "requires a free-threaded Python build",
)
class TestFreeThreadedImport(unittest.TestCase):
    def test_import_does_not_enable_gil(self):
        script = textwrap.dedent(
            """
            import sys

            import sudachipy

            if sys._is_gil_enabled():
                raise AssertionError("sudachipy import enabled the GIL")
            """
        )
        env = os.environ.copy()
        env["PYTHON_GIL"] = "0"

        subprocess.run(
            [sys.executable, "-Xgil=0", "-c", script],
            check=True,
            env=env,
            text=True,
        )
