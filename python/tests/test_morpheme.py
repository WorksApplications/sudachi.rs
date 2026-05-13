# Copyright (c) 2019-2026 Works Applications Co., Ltd.
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
import csv
import json
import tempfile
import unittest

from sudachipy import Dictionary, SplitMode
from sudachipy.sudachipy import build_system_dic


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

    def test_dictionary_form_morpheme(self):
        m = self.tokenizer_obj.tokenize('行っ')[0]
        df = m.dictionary_form_morpheme()

        self.assertEqual(df.surface(), '行く')
        self.assertEqual(df.raw_surface(), '行く')
        self.assertEqual(df.dictionary_form(), '行く')
        self.assertEqual(df.reading_form(), 'イク')
        self.assertEqual(df.begin(), 0)
        self.assertEqual(df.end(), len(df.surface()))

    def test_normalized_form_morpheme(self):
        m = self.tokenizer_obj.tokenize('いっ')[0]
        nf = m.normalized_form_morpheme()

        self.assertEqual(nf.surface(), '行く')
        self.assertEqual(nf.raw_surface(), '行く')
        self.assertEqual(nf.normalized_form(), '行く')
        self.assertEqual(nf.reading_form(), 'イク')
        self.assertEqual(nf.begin(), 0)
        self.assertEqual(nf.end(), len(nf.surface()))

    def test_form_morpheme_split_uses_standalone_offsets(self):
        resource_dir = os.path.join(os.path.dirname(
            os.path.abspath(__file__)), 'resources')
        split_ref = '東京,3,トウキョウ/都,4,ト'

        with tempfile.TemporaryDirectory() as temp_dir:
            extra_lex = os.path.join(temp_dir, 'extra.csv')
            system_dic = os.path.join(temp_dir, 'system.dic')
            config_path = os.path.join(temp_dir, 'sudachi.json')

            with open(extra_lex, 'w', encoding='utf-8', newline='') as out:
                writer = csv.writer(out)
                writer.writerow([
                    'index_form', 'left_id', 'right_id', 'cost', 'headword',
                    'pos1', 'pos2', 'pos3', 'pos4', 'pos5', 'pos6',
                    'reading_form', 'normalized_form', 'dictionary_form',
                    'split_a', 'split_b', 'split_c', 'word_structure',
                    'synonym_groups',
                ])
                writer.writerow([
                    '首都', 6, 6, 1000, '',
                    '名詞', '固有名詞', '地名', '一般', '*', '*',
                    'シュト', '', '', split_ref, '', '', split_ref, '',
                ])
                writer.writerow([
                    '首都旧', 6, 6, 1000, '',
                    '名詞', '固有名詞', '地名', '一般', '*', '*',
                    'シュトキュウ', '首都',
                    '首都,3,シュト',
                    '', '', '', '', '',
                ])

            build_system_dic(
                os.path.join(resource_dir, 'matrix.def'),
                [os.path.join(resource_dir, 'lex.csv'), extra_lex],
                system_dic,
            )

            with open(os.path.join(resource_dir, 'sudachi.json'), encoding='utf-8') as inp:
                config = json.load(inp)
            config['systemDict'] = system_dic
            config.pop('userDict', None)
            config.pop('path', None)
            with open(config_path, 'w', encoding='utf-8') as out:
                json.dump(config, out, ensure_ascii=False)

            tokenizer = Dictionary(config_path, resource_dir).create()
            m = tokenizer.tokenize('首都旧')[0]
            nf = m.normalized_form_morpheme()
            splits = nf.split(SplitMode.A, add_single=True)
            out = self.tokenizer_obj.tokenize('東京都', SplitMode.C)[0].split(
                SplitMode.A)
            out_result = nf.split(SplitMode.A, out=out, add_single=True)
            surface_tokenizer = Dictionary(config_path, resource_dir).create(
                fields={'surface'})
            surface_nf = surface_tokenizer.tokenize(
                '首都旧')[0].normalized_form_morpheme()
            surface_splits = surface_nf.split(SplitMode.A, add_single=True)

            self.assertEqual(nf.surface(), '首都')
            self.assertEqual(['東京', '都'], [s.surface() for s in splits])
            self.assertEqual([(0, 2), (2, 3)], [(s.begin(), s.end()) for s in splits])
            self.assertIs(out_result, out)
            self.assertEqual(['東京', '都'], [s.surface() for s in out_result])
            self.assertEqual(['東京', '都'], [
                s.surface() for s in surface_splits])

    def test_form_morpheme_for_same_entry(self):
        m = self.tokenizer_obj.tokenize('東京')[0]
        df = m.dictionary_form_morpheme()
        nf = m.normalized_form_morpheme()

        self.assertEqual(df.word_id(), m.word_id())
        self.assertEqual(nf.word_id(), m.word_id())
        self.assertEqual(df.surface(), '東京')
        self.assertEqual(nf.surface(), '東京')
        self.assertEqual(df.begin(), m.begin())
        self.assertEqual(df.end(), m.end())
        self.assertEqual(nf.begin(), m.begin())
        self.assertEqual(nf.end(), m.end())

    def test_form_morpheme_for_normalized_input_same_entry(self):
        m = self.tokenizer_obj.tokenize('特Ａ東京')[0]
        nf = m.normalized_form_morpheme()

        self.assertEqual(m.surface(), '特Ａ')
        self.assertEqual(m.normalized_form(), '特A')
        self.assertEqual(nf.word_id(), m.word_id())
        self.assertEqual(nf.raw_surface(), m.raw_surface())
        self.assertEqual(nf.surface(), m.surface())

    def test_form_morpheme_with_surface_subset(self):
        tokenizer = self.dict_.create(fields={'surface'})
        m = tokenizer.tokenize('行っ')[0]
        df = m.dictionary_form_morpheme()
        nf = m.normalized_form_morpheme()

        self.assertEqual(df.surface(), '行く')
        self.assertEqual(nf.surface(), '行く')
        self.assertEqual(df.raw_surface(), '行く')
        self.assertEqual(nf.raw_surface(), '行く')

    def test_form_morpheme_with_reading_subset(self):
        tokenizer = self.dict_.create(fields={'surface', 'reading_form'})
        m = tokenizer.tokenize('行っ')[0]
        df = m.dictionary_form_morpheme()
        nf = m.normalized_form_morpheme()

        self.assertEqual(df.surface(), '行く')
        self.assertEqual(nf.surface(), '行く')
        self.assertEqual(df.reading_form(), 'イク')
        self.assertEqual(nf.reading_form(), 'イク')

    def test_single_backed_form_morpheme_uses_projection(self):
        tokenizer = self.dict_.create(projection='reading')
        m = tokenizer.tokenize('行っ')[0]
        df = m.dictionary_form_morpheme()

        self.assertEqual(df.raw_surface(), '行く')
        self.assertEqual(df.surface(), 'イク')

    def test_single_backed_form_morpheme_can_chain_form_accessors(self):
        m = self.tokenizer_obj.tokenize('いっ')[0]
        nf = m.normalized_form_morpheme()
        df = nf.dictionary_form_morpheme()

        self.assertEqual(nf.word_id(), df.word_id())
        self.assertEqual(df.raw_surface(), '行く')
        self.assertEqual(df.surface(), '行く')
        self.assertEqual(df.dictionary_form(), '行く')

    def test_single_backed_morpheme_list_protocol(self):
        m = self.tokenizer_obj.tokenize('いっ')[0]
        nf = m.normalized_form_morpheme()
        result = nf.split(SplitMode.A, add_single=True)

        self.assertEqual(1, len(result))
        self.assertEqual('行く', result[0].surface())
        self.assertEqual('行く', result[-1].surface())
        self.assertEqual('行く', str(result))
        self.assertIn('<Morpheme(行く, 0:2, ', repr(result))

        with self.assertRaisesRegex(
                Exception,
                'standalone morpheme lists do not have a lattice path cost'):
            result.get_internal_cost()

    def test_single_backed_morpheme_list_can_be_reused_as_tokenize_out(self):
        m = self.tokenizer_obj.tokenize('いっ')[0]
        nf = m.normalized_form_morpheme()
        out = nf.split(SplitMode.A, add_single=True)

        result = self.tokenizer_obj.tokenize('東京', out=out)

        self.assertIs(result, out)
        self.assertEqual(['東京'], [m.surface() for m in result])

    def test_form_morpheme_oov_returns_self_equivalent(self):
        for m in self.tokenizer_obj.tokenize('xyzzy123不在語'):
            if not m.is_oov():
                continue

            df = m.dictionary_form_morpheme()
            nf = m.normalized_form_morpheme()

            self.assertTrue(df.is_oov())
            self.assertTrue(nf.is_oov())
            self.assertEqual(df.surface(), m.surface())
            self.assertEqual(nf.surface(), m.surface())
            self.assertEqual(df.begin(), m.begin())
            self.assertEqual(df.end(), m.end())
            self.assertEqual(nf.begin(), m.begin())
            self.assertEqual(nf.end(), m.end())
            return

        self.skipTest('test dictionary contains all words; cannot verify OOV branch')

    def test_morpheme_dictionary_id(self):
        m = self.tokenizer_obj.tokenize('京都')[0]
        self.assertEqual(m.dictionary_id(), 0)

        m = self.tokenizer_obj.tokenize('ぴらる')[0]
        self.assertEqual(m.dictionary_id(), 1)

        m = self.tokenizer_obj.tokenize('京')[0]
        self.assertTrue(m.dictionary_id() < 0)

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
        self.assertEqual(repr(
            ms), '<MorphemeList[\n  <Morpheme(東京, 0:2, (0, 25))>,\n  <Morpheme(都, 2:3, (0, 42))>,\n]>')
        self.assertEqual(str(ms[0]), '東京')
        self.assertEqual(str(ms[1]), '都')
        self.assertEqual(repr(ms[0]), '<Morpheme(東京, 0:2, (0, 25))>')
        self.assertEqual(repr(ms[1]), '<Morpheme(都, 2:3, (0, 42))>')


if __name__ == '__main__':
    unittest.main()
