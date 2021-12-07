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

from sudachipy import Dictionary, SplitMode


class TestTokenizer(unittest.TestCase):

    def setUp(self):
        resource_dir = os.path.join(os.path.dirname(
            os.path.abspath(__file__)), 'resources')
        self.dict_ = Dictionary(os.path.join(
            resource_dir, 'sudachi.json'), resource_dir)
        self.tokenizer_obj = self.dict_.create()

    def test_empty_list(self):
        ms = self.tokenizer_obj.tokenize('')
        self.assertEqual(0, ms.size())

    def test_morpheme_split(self):
        ms = self.tokenizer_obj.tokenize('東京都', SplitMode.C)
        self.assertEqual(1, ms.size())
        self.assertEqual(ms[0].surface(), '東京都')

        ms_a = ms[0].split(SplitMode.A)
        self.assertEqual(2, ms_a.size())
        self.assertEqual(ms_a[0].surface(), '東京')
        self.assertEqual(ms_a[1].surface(), '都')

    def test_morpheme_split_middle(self):
        ms_c = self.tokenizer_obj.tokenize('京都東京都京都', SplitMode.C)
        self.assertEqual(3, ms_c.size())
        self.assertEqual(ms_c[1].surface(), '東京都')
        self.assertEqual(ms_c[1].begin(), 2)
        self.assertEqual(ms_c[1].end(), 5)

        ms_a = ms_c[1].split(SplitMode.A)
        self.assertEqual(2, ms_a.size())
        self.assertEqual(ms_a[0].surface(), '東京')
        self.assertEqual(ms_a[0].begin(), 2)
        self.assertEqual(ms_a[0].end(), 4)
        self.assertEqual(ms_a[1].surface(), '都')
        self.assertEqual(ms_a[1].begin(), 4)
        self.assertEqual(ms_a[1].end(), 5)

    def test_morpheme_index(self):
        m = self.tokenizer_obj.tokenize('東京都')[0]
        self.assertEqual(m.begin(), 0)
        self.assertEqual(m.end(), 3)

    def test_morpheme_pos(self):
        m = self.tokenizer_obj.tokenize('京都')[0]
        self.assertEqual(m.part_of_speech_id(), 3)
        self.assertEqual(m.part_of_speech(), (
                         '名詞', '固有名詞', '地名', '一般', '*', '*'))

    def test_morpheme_forms(self):
        m = self.tokenizer_obj.tokenize('東京')[0]
        self.assertEqual(m.surface(), '東京')
        self.assertEqual(m.dictionary_form(), '東京')
        self.assertEqual(m.normalized_form(), '東京')
        self.assertEqual(m.reading_form(), 'トウキョウ')

        m = self.tokenizer_obj.tokenize('ぴらる')[0]
        self.assertEqual(m.surface(), 'ぴらる')
        self.assertEqual(m.dictionary_form(), 'ぴらる')
        self.assertEqual(m.normalized_form(), 'ぴらる')
        self.assertEqual(m.reading_form(), 'ピラル')

    def test_morpheme_dictionary_id(self):
        m = self.tokenizer_obj.tokenize('京都')[0]
        self.assertEqual(m.dictionary_id(), 0)

        m = self.tokenizer_obj.tokenize('ぴらる')[0]
        self.assertEqual(m.dictionary_id(), 1)

        m = self.tokenizer_obj.tokenize('京')[0]
        self.assertTrue(m.dictionary_id() < 0)

    def test_morpheme_word_id(self):
        m = self.tokenizer_obj.tokenize('京都')[0]
        self.assertEqual(m.word_id(), 3)

        m = self.tokenizer_obj.tokenize('ぴらる')[0]
        self.assertEqual(m.word_id(), 2**28 + 0)

    def test_morpheme_oov(self):
        m = self.tokenizer_obj.tokenize('京都')[0]
        self.assertEqual(m.is_oov(), False)

        m = self.tokenizer_obj.tokenize('京')[0]
        self.assertEqual(m.is_oov(), True)

    def test_morpheme_synonym_group_ids(self):
        m = self.tokenizer_obj.tokenize('京都')[0]
        self.assertEqual(m.synonym_group_ids(), [1, 5])

        m = self.tokenizer_obj.tokenize('ぴらる')[0]
        self.assertEqual(m.synonym_group_ids(), [])

        m = self.tokenizer_obj.tokenize('東京府')[0]
        self.assertEqual(m.synonym_group_ids(), [1, 3])

    def test_normalize_half_full(self):
        m = self.tokenizer_obj.tokenize('特Ａ東京')
        self.assertEqual(len(m), 2)

        self.assertEqual(m[0].surface(), "特Ａ")
        self.assertEqual(m[0].begin(), 0)
        self.assertEqual(m[0].end(), 2)

    def test_normalize_katakana_half_full(self):
        m = self.tokenizer_obj.tokenize('ｶﾞ5')
        self.assertEqual(len(m), 2)

        self.assertEqual(m[0].surface(), "ｶﾞ")
        self.assertEqual(m[0].end(), 2)
        self.assertEqual(m[1].begin(), 2)
        self.assertEqual(len(m[0]), 2)

    def test_morpheme_split_out(self):
        ms = self.tokenizer_obj.tokenize('東京都', SplitMode.C)
        self.assertEqual(1, ms.size())
        self.assertEqual(ms[0].surface(), '東京都')

        ms_a = ms[0].split(SplitMode.A, out=None)
        self.assertEqual(2, ms_a.size())
        self.assertEqual(ms_a[0].surface(), '東京')
        self.assertEqual(ms_a[1].surface(), '都')

        ms = self.tokenizer_obj.tokenize("京都東京都京都", SplitMode.C)
        ms_b = ms[1].split(SplitMode.A, out=ms_a)
        self.assertEqual(id(ms_a), id(ms_b))
        self.assertEqual(ms_a[0].surface(), '東京')
        self.assertEqual(ms_a[1].surface(), '都')

    def test_morpheme_str_repr(self):
        ms = self.tokenizer_obj.tokenize('東京都', SplitMode.A)
        self.assertEqual(2, ms.size())
        self.assertEqual(str(ms), '東京 都')
        self.assertEqual(repr(ms), '<MorphemeList[\n  <Morpheme(東京, 0:2, (0, 5))>,\n  <Morpheme(都, 2:3, (0, 9))>,\n]>')
        self.assertEqual(str(ms[0]), '東京')
        self.assertEqual(str(ms[1]), '都')
        self.assertEqual(repr(ms[0]), '<Morpheme(東京, 0:2, (0, 5))>')
        self.assertEqual(repr(ms[1]), '<Morpheme(都, 2:3, (0, 9))>')


if __name__ == '__main__':
    unittest.main()
