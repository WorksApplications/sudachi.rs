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

from sudachipy import Dictionary


class TestTokenizer(unittest.TestCase):

    def setUp(self):
        resource_dir = os.path.join(os.path.dirname(
            os.path.abspath(__file__)), 'resources')
        self.dict_ = Dictionary(os.path.join(
            resource_dir, 'sudachi.json'), resource_dir)
        self.tokenizer_obj = self.dict_.create()

    def test_wordinfo(self):
        # た
        wi = self.tokenizer_obj.tokenize('た')[0].get_word_info()
        self.assertEqual('た', wi.surface)
        self.assertEqual(3, wi.head_word_length)
        self.assertEqual(0, wi.pos_id)
        self.assertEqual('た', wi.normalized_form)
        self.assertEqual(-1, wi.dictionary_form_word_id)
        self.assertEqual('た', wi.dictionary_form)
        self.assertEqual('タ', wi.reading_form)
        self.assertEqual([], wi.a_unit_split)
        self.assertEqual([], wi.b_unit_split)
        self.assertEqual([], wi.word_structure)

        # 行っ
        wi = self.tokenizer_obj.tokenize('行っ')[0].get_word_info()
        self.assertEqual('行っ', wi.surface)
        self.assertEqual('行く', wi.normalized_form)
        self.assertEqual(7, wi.dictionary_form_word_id)
        self.assertEqual('行く', wi.dictionary_form)

        # 東京都
        wi = self.tokenizer_obj.tokenize('東京都')[0].get_word_info()
        self.assertEqual('東京都', wi.surface)
        self.assertEqual([5, 9], wi.a_unit_split)
        self.assertEqual([], wi.b_unit_split)
        self.assertEqual([5, 9], wi.word_structure)
        self.assertEqual([], wi.synonym_group_ids)

    def test_wordinfo_with_longword(self):
        s = "0123456789" * 30
        wi = self.tokenizer_obj.tokenize(s)[0].get_word_info()
        self.assertEqual(300, len(wi.surface))
        self.assertEqual(300, wi.head_word_length)
        self.assertEqual(300, len(wi.normalized_form))
        self.assertEqual(-1, wi.dictionary_form_word_id)
        self.assertEqual(300, len(wi.dictionary_form))
        self.assertEqual(570, len(wi.reading_form))

    def test_wordinfo_surface(self):
        wi = self.tokenizer_obj.tokenize('京都')[0].get_word_info()
        self.assertEqual(wi.surface, "京都")

        wi = self.tokenizer_obj.tokenize('東京府')[0].get_word_info()
        self.assertEqual(wi.surface, "東京府")

    def test_wordinfo_length(self):
        wi = self.tokenizer_obj.tokenize('京都')[0].get_word_info()
        self.assertEqual(wi.head_word_length, 6)
        self.assertEqual(wi.length(), 6)

        wi = self.tokenizer_obj.tokenize('東京府')[0].get_word_info()
        self.assertEqual(wi.head_word_length, 9)
        self.assertEqual(wi.length(), 9)

    def test_wordinfo_pos(self):
        wi = self.tokenizer_obj.tokenize('東')[0].get_word_info()
        self.assertEqual(wi.pos_id, 4)

        wi = self.tokenizer_obj.tokenize('東京府')[0].get_word_info()
        self.assertEqual(wi.pos_id, 3)

    def test_wordinfo_forms(self):
        wi = self.tokenizer_obj.tokenize('東京')[0].get_word_info()
        self.assertEqual(wi.dictionary_form_word_id, -1)
        self.assertEqual(wi.dictionary_form, '東京')
        self.assertEqual(wi.normalized_form, '東京')
        self.assertEqual(wi.reading_form, 'トウキョウ')

        wi = self.tokenizer_obj.tokenize('東京府')[0].get_word_info()
        self.assertEqual(wi.dictionary_form_word_id, -1)
        self.assertEqual(wi.dictionary_form, "東京府")
        self.assertEqual(wi.normalized_form, "東京府")
        self.assertEqual(wi.reading_form, "トウキョウフ")

    def test_wordinfo_unit_split(self):
        wi = self.tokenizer_obj.tokenize('東京')[0].get_word_info()
        self.assertEqual(wi.a_unit_split, [])
        self.assertEqual(wi.b_unit_split, [])

        wi = self.tokenizer_obj.tokenize('東京府')[0].get_word_info()
        self.assertEqual(wi.a_unit_split, [5, 2**28 + 1])
        self.assertEqual(wi.b_unit_split, [])

    def test_wordinfo_word_structure(self):
        wi = self.tokenizer_obj.tokenize('東京')[0].get_word_info()
        self.assertEqual(wi.word_structure, [])

        wi = self.tokenizer_obj.tokenize('東京府')[0].get_word_info()
        self.assertEqual(wi.word_structure, [5, 2**28 + 1])

    def test_wordinfo_synonym_group_ids(self):
        wi = self.tokenizer_obj.tokenize('東京')[0].get_word_info()
        self.assertEqual(wi.synonym_group_ids, [])

        wi = self.tokenizer_obj.tokenize('東京府')[0].get_word_info()
        self.assertEqual(wi.synonym_group_ids, [1, 3])


if __name__ == '__main__':
    unittest.main()
