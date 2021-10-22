#   Copyright (c) 2021 Works Applications Co., Ltd.
#
#   Licensed under the Apache License, Version 2.0 (the "License");
#   you may not use this file except in compliance with the License.
#   You may obtain a copy of the License at
#
#       http://www.apache.org/licenses/LICENSE-2.0
#
#    Unless required by applicable law or agreed to in writing, software
#   distributed under the License is distributed on an "AS IS" BASIS,
#   WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
#   See the License for the specific language governing permissions and
#   limitations under the License.

import unittest


class TestImport(unittest.TestCase):
    def test_import_dictionary(self):
        from sudachipy.dictionary import Dictionary
        self.assertIsNotNone(Dictionary)

    def test_import_morpheme(self):
        from sudachipy.morpheme import Morpheme
        self.assertIsNotNone(Morpheme)

    def test_import_morphemelist(self):
        from sudachipy.morphemelist import MorphemeList
        self.assertIsNotNone(MorphemeList)
