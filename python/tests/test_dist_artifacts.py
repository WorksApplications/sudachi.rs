# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
#     http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.

import contextlib
import importlib.util
import io
from pathlib import Path
import tarfile
import tempfile
import unittest
import zipfile

SCRIPT = Path(__file__).resolve().parents[1] / 'verify-dist-artifacts.py'
spec = importlib.util.spec_from_file_location('verify_dist_artifacts', SCRIPT)
artifacts = importlib.util.module_from_spec(spec)
spec.loader.exec_module(artifacts)

WHEEL_FILES = {
    'sudachipy/__init__.py',
    'sudachipy/sudachipy.pyi',
    'sudachipy/resources/sudachi.json',
    'sudachipy/sudachipy.pyd',
}
EMBEDDED_FILES = {
    'sudachipy/resources/char.def',
    'sudachipy/resources/rewrite.def',
    'sudachipy/resources/unk.def',
}


class TestDistributionArtifacts(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.directory = Path(self.temp.name)

    def wheel(self, files):
        path = self.directory / 'sudachipy-0.7.1a1-cp310-abi3-win_amd64.whl'
        with zipfile.ZipFile(path, 'w') as archive:
            for name in files:
                archive.writestr(name, '')
        return path

    def sdist(self, files):
        path = self.directory / 'sudachipy-0.7.1a1.tar.gz'
        with tarfile.open(path, 'w:gz') as archive:
            for name in files:
                member = tarfile.TarInfo('sudachipy-0.7.1a1/' + name)
                archive.addfile(member, io.BytesIO())
        return path

    def assert_invalid(self, check, path, message):
        stderr = io.StringIO()
        with contextlib.redirect_stderr(stderr), self.assertRaises(SystemExit):
            check(path)
        self.assertIn(message, stderr.getvalue())

    def test_wheel_with_configuration_and_embedded_native_defaults(self):
        artifacts.check_wheel(self.wheel(WHEEL_FILES))

    def test_wheel_rejects_redundant_embedded_resources(self):
        for name in sorted(EMBEDDED_FILES):
            with self.subTest(resource=name):
                self.assert_invalid(artifacts.check_wheel, self.wheel(WHEEL_FILES | {name}),
                                    'contains redundant embedded resource ' + name)

    def test_wheel_requires_python_configuration(self):
        name = 'sudachipy/resources/sudachi.json'
        self.assert_invalid(artifacts.check_wheel, self.wheel(WHEEL_FILES - {name}),
                            'is missing ' + name)

    def test_wheel_requires_typing_stub(self):
        name = 'sudachipy/sudachipy.pyi'
        self.assert_invalid(artifacts.check_wheel, self.wheel(WHEEL_FILES - {name}),
                            'is missing ' + name)

    def test_wheel_requires_native_extension(self):
        self.assert_invalid(artifacts.check_wheel,
                            self.wheel(WHEEL_FILES - {'sudachipy/sudachipy.pyd'}),
                            'is missing the compiled sudachipy extension')

    def test_sdist_retains_build_resources(self):
        artifacts.check_sdist(self.sdist(artifacts.SDIST_REQUIRED))

    def test_sdist_requires_each_build_resource_and_python_configuration(self):
        for name in ('resources/sudachi.json', 'resources/char.def',
                     'resources/rewrite.def', 'resources/unk.def',
                     'python/py_src/sudachipy/resources/sudachi.json'):
            with self.subTest(resource=name):
                self.assert_invalid(artifacts.check_sdist,
                                    self.sdist(set(artifacts.SDIST_REQUIRED) - {name}),
                                    'is missing ' + name)

    def test_sdist_archive_paths_are_posix_on_all_platforms(self):
        self.assertEqual(artifacts.sdist_names(self.sdist({'resources/char.def'})),
                         {'resources/char.def'})


if __name__ == '__main__':
    unittest.main()
