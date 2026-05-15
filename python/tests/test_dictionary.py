# Copyright (c) 2019 Works Applications Co., Ltd.
#
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

import os
import unittest

import sudachipy
from sudachipy import Dictionary, Tokenizer


class TestDictionary(unittest.TestCase):

    def setUp(self):
        resource_dir = os.path.join(os.path.dirname(
            os.path.abspath(__file__)), 'resources')
        self.dict_ = Dictionary(os.path.join(
            resource_dir, 'sudachi.json'), resource_dir=resource_dir)

    def tearDown(self) -> None:
        self.dict_.close()

    def test_create(self):
        self.assertEqual(Tokenizer, type(self.dict_.create()))

    def test_pos_of(self):
        self.assertIsNotNone(self.dict_.pos_of(0))

    def test_repr(self):
        repr_str = repr(self.dict_)
        self.assertTrue(repr_str.startswith("<SudachiDictionary(system="))
        self.assertTrue(repr_str.endswith("user.dic.test])>"))

    def test_lookup(self):
        ms = self.dict_.lookup("東京都")
        self.assertEqual(1, len(ms))
        self.assertEqual("トウキョウト", ms[0].reading_form())
        self.assertEqual(0, ms[0].begin())
        self.assertEqual(3, ms[0].end())
        splits = ms[0].split(sudachipy.SplitMode.A)
        self.assertEqual(2, len(splits))
        ms = self.dict_.lookup("京都", out=ms)
        self.assertEqual(1, len(ms))
        self.assertEqual("キョウト", ms[0].reading_form())
        self.assertEqual(0, ms[0].begin())
        self.assertEqual(2, ms[0].end())

    def test_entries(self):
        entries = list(self.dict_.entries())
        surfaces = [m.raw_surface() for m in entries]

        self.assertIn("東京都", surfaces)
        self.assertIn("すだち", surfaces)
        self.assertTrue(any(m.dictionary_id() == 0 for m in entries))
        self.assertEqual(1, next(m for m in entries if m.raw_surface() == "すだち").dictionary_id())

    def test_lookup_all_entries(self):
        self.assertFalse(self.dict_.lookup_all_entries("存在しない語"))

        tokyo = self.dict_.lookup_all_entries("東京都")
        self.assertEqual(1, len(tokyo))
        self.assertEqual("トウキョウト", tokyo[0].reading_form())
        self.assertEqual("東京都", tokyo[0].raw_surface())
        self.assertEqual(0, tokyo[0].begin())
        self.assertEqual(3, tokyo[0].end())

        normalized = self.dict_.lookup_all_entries("特A")
        self.assertEqual(1, len(normalized))
        self.assertEqual("特A", normalized[0].raw_surface())

        user_entry = self.dict_.lookup_all_entries("すだち")
        self.assertEqual(1, len(user_entry))
        self.assertEqual(1, user_entry[0].dictionary_id())
        self.assertEqual("スダチ", user_entry[0].reading_form())

        out = self.dict_.lookup("京都")
        reused = self.dict_.lookup_all_entries("東京都", out=out)
        self.assertIs(reused, out)
        self.assertEqual("東京都", reused[0].raw_surface())


if __name__ == '__main__':
    unittest.main()
