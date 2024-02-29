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

    def test_split_mode_default(self):
        mode_c = SplitMode()
        self.assertEqual(mode_c, SplitMode.C)

    def test_split_mode_from_string_a(self):
        mode = SplitMode("A")
        self.assertEqual(mode, SplitMode.A)

    def test_split_mode_from_string_b(self):
        mode = SplitMode("B")
        self.assertEqual(mode, SplitMode.B)

    def test_split_mode_from_string_c(self):
        mode = SplitMode("C")
        self.assertEqual(mode, SplitMode.C)

    def test_tokenizer_with_split_mode_str(self):
        tok_a = self.dict_.create("A")
        self.assertEqual(tok_a.mode, SplitMode.A)

    def test_tokenize_small_katanana_only(self):
        ms = self.tokenizer_obj.tokenize('ァ')
        self.assertEqual(1, len(ms))

    def test_part_of_speech(self):
        ms = self.tokenizer_obj.tokenize('京都')
        self.assertEqual(1, len(ms))
        m = ms[0]
        pid = m.part_of_speech_id()
        self.assertEqual(3, pid)
        pos = m.part_of_speech()
        self.assertEqual(('名詞', '固有名詞', '地名', '一般', '*', '*'), pos)

    def test_get_word_id(self):
        ms = self.tokenizer_obj.tokenize('京都')
        self.assertEqual(1, len(ms))
        self.assertEqual(('名詞', '固有名詞', '地名', '一般', '*', '*'),
                         ms[0].part_of_speech())

        wid = ms[0].word_id()
        ms = self.tokenizer_obj.tokenize('ぴらる')
        self.assertEqual(1, len(ms))
        self.assertNotEqual(wid, ms[0].word_id())
        self.assertEqual(('名詞', '普通名詞', '一般', '*', '*', '*'),
                         ms[0].part_of_speech())

        ms = self.tokenizer_obj.tokenize('京')
        self.assertEqual(1, len(ms))

    def test_get_dictionary_id(self):
        ms = self.tokenizer_obj.tokenize('京都')
        self.assertEqual(1, ms.size())
        self.assertEqual(0, ms[0].dictionary_id())

        ms = self.tokenizer_obj.tokenize('ぴらる')
        self.assertEqual(1, ms.size())
        self.assertEqual(1, ms[0].dictionary_id())

        ms = self.tokenizer_obj.tokenize('京')
        self.assertEqual(1, ms.size())
        self.assertTrue(ms[0].dictionary_id() < 0)

    def test_get_synonym_group_ids(self):
        ms = self.tokenizer_obj.tokenize('京都')
        self.assertEqual(1, ms.size())
        self.assertEqual([1, 5], ms[0].synonym_group_ids())

        ms = self.tokenizer_obj.tokenize('ぴらる')
        self.assertEqual(1, ms.size())
        self.assertEqual([], ms[0].synonym_group_ids())

        ms = self.tokenizer_obj.tokenize('東京府')
        self.assertEqual(1, ms.size())
        self.assertEqual([1, 3], ms[0].synonym_group_ids())

    def test_tokenize_kanji_alphabet_word(self):
        self.assertEqual(len(self.tokenizer_obj.tokenize('特a')), 1)
        self.assertEqual(len(self.tokenizer_obj.tokenize('ab')), 1)
        self.assertEqual(len(self.tokenizer_obj.tokenize('特ab')), 2)

    def test_tokenizer_with_dots(self):
        ms = self.tokenizer_obj.tokenize('京都…')
        self.assertEqual(4, ms.size())
        self.assertEqual(ms[1].surface(), '…')
        self.assertEqual(ms[1].normalized_form(), '.')
        self.assertEqual(ms[2].surface(), '')
        self.assertEqual(ms[2].normalized_form(), '.')
        self.assertEqual(ms[3].surface(), '')
        self.assertEqual(ms[3].normalized_form(), '.')

    def test_tokenizer_morpheme_split(self):
        ms = self.tokenizer_obj.tokenize('東京都', SplitMode.C)
        self.assertEqual(1, ms.size())
        self.assertEqual(ms[0].surface(), '東京都')

        ms_a = ms[0].split(SplitMode.A)
        self.assertEqual(2, ms_a.size())
        self.assertEqual(ms_a[0].surface(), '東京')
        self.assertEqual(ms_a[1].surface(), '都')

    def test_tokenizer_morpheme_split_strings(self):
        ms = self.tokenizer_obj.tokenize('東京都', 'C')
        self.assertEqual(1, ms.size())
        self.assertEqual(ms[0].surface(), '東京都')

        ms_a = ms[0].split('A')
        self.assertEqual(2, ms_a.size())
        self.assertEqual(ms_a[0].surface(), '東京')
        self.assertEqual(ms_a[1].surface(), '都')

    def test_tokenizer_morpheme_list_range(self):
        ms = self.tokenizer_obj.tokenize('東京都', SplitMode.A)
        self.assertEqual(2, ms.size())
        self.assertEqual(ms[0].surface(), '東京')
        self.assertEqual(ms[1].surface(), '都')

        self.assertEqual(ms[-1].surface(), ms[1].surface())
        self.assertEqual(ms[-2].surface(), ms[0].surface())
        with self.assertRaises(IndexError):
            ms[2]
        with self.assertRaises(IndexError):
            ms[-3]

    def test_tokenizer_subset(self):
        ms1 = self.tokenizer_obj.tokenize('東京都')

        tok = self.dict_.create(fields={"pos"})
        ms2 = tok.tokenize('東京都')
        self.assertEqual(ms1[0].part_of_speech_id(), ms2[0].part_of_speech_id())

    def test_tokenizer_out_param(self):
        ms1 = self.tokenizer_obj.tokenize('東京都東京府')
        m = ms1[0]
        self.assertEqual(m.surface(), '東京都')

        ms2 = self.tokenizer_obj.tokenize('すだち', out=ms1)
        self.assertEqual(id(ms1), id(ms2))
        self.assertEqual(m.surface(), 'すだち')


if __name__ == '__main__':
    unittest.main()
