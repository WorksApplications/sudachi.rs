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

from sudachi.sudachi import Dictionary, SplitMode


class TestTokenizer(unittest.TestCase):

    def setUp(self):
        resource_dir = os.path.join(os.path.dirname(
            os.path.abspath(__file__)), 'resources')
        self.dict_ = Dictionary(os.path.join(
            resource_dir, 'sudachi.json'), resource_dir)
        self.tokenizer_obj = self.dict_.create()

    def test_wordinfo_surface(self):
        wi = self.tokenizer_obj.tokenize('京都', SplitMode.C)[0].get_word_info()
        self.assertEqual(wi.surface, "京都")

        wi = self.tokenizer_obj.tokenize('東京府', SplitMode.C)[0].get_word_info()
        self.assertEqual(wi.surface, "東京府")

    def test_wordinfo_length(self):
        wi = self.tokenizer_obj.tokenize('京都', SplitMode.C)[0].get_word_info()
        self.assertEqual(wi.head_word_length, 6)
        self.assertEqual(wi.length(), 6)

        wi = self.tokenizer_obj.tokenize('東京府', SplitMode.C)[0].get_word_info()
        self.assertEqual(wi.head_word_length, 9)
        self.assertEqual(wi.length(), 9)

    def test_wordinfo_pos(self):
        wi = self.tokenizer_obj.tokenize('東', SplitMode.C)[0].get_word_info()
        self.assertEqual(wi.pos_id,     4)

        wi = self.tokenizer_obj.tokenize('東京府', SplitMode.C)[0].get_word_info()
        self.assertEqual(wi.pos_id, 3)

    def test_wordinfo_forms(self):
        wi = self.tokenizer_obj.tokenize('東京', SplitMode.C)[0].get_word_info()
        self.assertEqual(wi.dictionary_form_word_id, -1)
        self.assertEqual(wi.dictionary_form, '東京')
        self.assertEqual(wi.normalized_form, '東京')
        self.assertEqual(wi.reading_form, 'トウキョウ')

        wi = self.tokenizer_obj.tokenize('東京府', SplitMode.C)[0].get_word_info()
        self.assertEqual(wi.dictionary_form_word_id, -1)
        self.assertEqual(wi.dictionary_form, "東京府")
        self.assertEqual(wi.normalized_form, "東京府")
        self.assertEqual(wi.reading_form, "トウキョウフ")

    def test_wordinfo_unit_split(self):
        wi = self.tokenizer_obj.tokenize('東京', SplitMode.C)[0].get_word_info()
        self.assertEqual(wi.a_unit_split, [])
        self.assertEqual(wi.b_unit_split, [])

        wi = self.tokenizer_obj.tokenize('東京府', SplitMode.C)[0].get_word_info()
        self.assertEqual(wi.a_unit_split, [5, 2**28 + 1])
        self.assertEqual(wi.b_unit_split, [])

    def test_wordinfo_word_structure(self):
        wi = self.tokenizer_obj.tokenize('東京', SplitMode.C)[0].get_word_info()
        self.assertEqual(wi.word_structure, [])

        wi = self.tokenizer_obj.tokenize('東京府', SplitMode.C)[0].get_word_info()
        self.assertEqual(wi.word_structure, [5, 2**28 + 1])

    def test_wordinfo_synonym_group_ids(self):
        wi = self.tokenizer_obj.tokenize('東京', SplitMode.C)[0].get_word_info()
        self.assertEqual(wi.synonym_group_ids, [])

        wi = self.tokenizer_obj.tokenize('東京府', SplitMode.C)[0].get_word_info()
        self.assertEqual(wi.synonym_group_ids, [1, 3])


if __name__ == '__main__':
    unittest.main()
