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

import json
import os
import tempfile
import unittest
from pathlib import Path

import sudachipy
from sudachipy import Dictionary, Tokenizer
from sudachipy.config import Config
from sudachipy.sudachipy import build_system_dic


NON_INDEXED_ENTRY_LEXICON = (
    "index_form,left_id,right_id,cost,pos1,pos2,pos3,pos4,pos5,pos6,reading_form,"
    "normalized_form,dictionary_form,mode,split_a,split_b,word_structure\n"
    "京都,6,6,5293,名詞,固有名詞,地名,一般,*,*,キョウト,,,A,,,\n"
    "隠し,-1,-1,5293,名詞,普通名詞,一般,*,*,*,カクシ,,,A,,,\n"
    "舞台藝術,1,1,2816,名詞,普通名詞,一般,*,*,*,ブタイゲイジュツ,舞台芸術,,A,,,\n"
)


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
        normalized = self.dict_.lookup("特A")
        self.assertEqual(1, len(normalized))
        self.assertEqual("トクエー", normalized[0].reading_form())

    def test_entries(self):
        entries = list(self.dict_.entries())
        surfaces = [m.raw_surface() for m in entries]

        self.assertIn("東京都", surfaces)
        self.assertIn("すだち", surfaces)
        self.assertTrue(any(m.dictionary_id() == 0 for m in entries))
        tokyo = next(m for m in entries if m.raw_surface() == "東京都")
        self.assertEqual(("名詞", "固有名詞", "地名", "一般", "*", "*"), tokyo.part_of_speech())
        self.assertEqual("東京都", tokyo.normalized_form())
        self.assertEqual("東京都", tokyo.dictionary_form())
        self.assertEqual("東京都", str(tokyo))
        self.assertGreater(tokyo.word_id(), 0)

        user_entry = next(m for m in entries if m.raw_surface() == "すだち")
        self.assertEqual(1, user_entry.dictionary_id())
        self.assertEqual("すだち", user_entry.normalized_form())

    def test_lookup_all_entries(self):
        self.assertFalse(self.dict_.lookup_all_entries("存在しない語"))

        tokyo = self.dict_.lookup_all_entries("東京都")
        self.assertEqual(1, len(tokyo))
        self.assertEqual("トウキョウト", tokyo[0].reading_form())
        self.assertEqual("東京都", tokyo[0].raw_surface())
        self.assertEqual(0, tokyo[0].begin())
        self.assertEqual(3, tokyo[0].end())
        self.assertEqual(2, len(tokyo[0].split(sudachipy.SplitMode.A)))
        with self.assertRaises(sudachipy.errors.SudachiError):
            tokyo.get_internal_cost()

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

    def test_entries_include_non_indexed_entries_and_exclude_phantoms(self):
        resource_dir = Path(__file__).parent / "resources"
        with tempfile.TemporaryDirectory() as temp_dir:
            lexicon = Path(temp_dir) / "non_indexed.csv"
            system_dic = Path(temp_dir) / "system.dic"
            lexicon.write_text(NON_INDEXED_ENTRY_LEXICON, encoding="utf-8")
            build_system_dic(
                matrix=resource_dir / "matrix.def",
                lex=[lexicon],
                output=system_dic,
            )

            dictionary = Dictionary(config=Config(
                system=str(system_dic),
                characterDefinitionFile=str(resource_dir / "char.def"),
                inputTextPlugin=[
                    {"class": "com.worksap.nlp.sudachi.DefaultInputTextPlugin"},
                    {"class": "com.worksap.nlp.sudachi.IgnoreYomiganaPlugin",
                     "leftBrackets": ["(", "（"],
                     "rightBrackets": [")", "）"],
                     "maxYomiganaLength": 4},
                ],
                oovProviderPlugin=[
                    {"class": "com.worksap.nlp.sudachi.SimpleOovPlugin",
                     "oovPOS": ["名詞", "普通名詞", "一般", "*", "*", "*"],
                     "leftId": 8,
                     "rightId": 8,
                     "cost": 6000},
                ],
                pathRewritePlugin=[],
            ))
            try:
                surfaces = [m.raw_surface() for m in dictionary.entries()]
                self.assertIn("京都", surfaces)
                self.assertIn("隠し", surfaces)
                self.assertIn("舞台藝術", surfaces)
                self.assertNotIn("舞台芸術", surfaces)

                non_indexed = dictionary.lookup_all_entries("隠し")
                self.assertEqual(1, len(non_indexed))
                self.assertEqual("隠し", non_indexed[0].raw_surface())
                self.assertFalse(dictionary.lookup("隠し"))

                yomigana = dictionary.lookup("京都（キョウト）")
                self.assertEqual(1, len(yomigana))
                self.assertEqual("京都", yomigana[0].surface())
                self.assertEqual("京都", yomigana[0].raw_surface())
                self.assertEqual(0, yomigana[0].begin())
                self.assertEqual(2, yomigana[0].end())

                phantom = dictionary.lookup_all_entries("舞台芸術")
                self.assertFalse(phantom)
            finally:
                dictionary.close()

    def test_resource_dir_precedes_config_parent(self):
        resource_dir = Path(__file__).parent / "resources"
        with tempfile.TemporaryDirectory() as temp_dir:
            temp_path = Path(temp_dir)
            config_path = temp_path / "sudachi.json"
            # If the config directory is searched first, dictionary creation will try to
            # open this directory as `char.def` and fail before reaching `resource_dir`.
            (temp_path / "char.def").mkdir()
            config_path.write_text(json.dumps({
                "systemDict": str(resource_dir / "system.dic.test"),
                "userDict": [str(resource_dir / "user.dic.test")],
                "characterDefinitionFile": "char.def",
                "inputTextPlugin": [
                    {"class": "com.worksap.nlp.sudachi.DefaultInputTextPlugin"}
                ],
                "oovProviderPlugin": [
                    {"class": "com.worksap.nlp.sudachi.SimpleOovPlugin",
                     "oovPOS": ["名詞", "普通名詞", "一般", "*", "*", "*"],
                     "leftId": 8,
                     "rightId": 8,
                     "cost": 6000}
                ],
                "pathRewritePlugin": [
                    {"class": "com.worksap.nlp.sudachi.JoinNumericPlugin",
                     "enableNormalize": True},
                    {"class": "com.worksap.nlp.sudachi.JoinKatakanaOovPlugin",
                     "oovPOS": ["名詞", "普通名詞", "一般", "*", "*", "*"],
                     "minLength": 3}
                ]
            }), encoding="utf-8")

            dictionary = Dictionary(str(config_path), resource_dir=str(resource_dir))
            try:
                morphemes = dictionary.lookup("東京都")
                self.assertEqual(1, len(morphemes))
                self.assertEqual("トウキョウト", morphemes[0].reading_form())
            finally:
                dictionary.close()

    def test_oov_morpheme(self):
        pos_id1 = 1
        m1 = self.dict_.oov_morpheme(pos_id1, "OOV")
        self.assertEqual(0, m1.begin())
        self.assertEqual(3, m1.end())
        self.assertEqual(pos_id1, m1.part_of_speech_id())
        self.assertEqual("OOV", m1.surface())
        self.assertEqual("OOV", m1.reading_form())
        self.assertEqual("OOV", m1.normalized_form())
        self.assertEqual("OOV", m1.dictionary_form())
        self.assertTrue(m1.is_oov())
        self.assertEqual(-1, m1.dictionary_id())
        self.assertEqual([], m1.synonym_group_ids())

        pos_id2 = 2
        m2 = self.dict_.oov_morpheme(pos_id2, "OOVs", "OOVr", "OOVn", "OOVd")
        self.assertEqual(0, m2.begin())
        self.assertEqual(4, m2.end())
        self.assertEqual(pos_id2, m2.part_of_speech_id())
        self.assertEqual("OOVs", m2.surface())
        self.assertEqual("OOVr", m2.reading_form())
        self.assertEqual("OOVn", m2.normalized_form())
        self.assertEqual("OOVd", m2.dictionary_form())

        # form_morpheme return self for OOV morphemes
        self.assertEqual("OOV", m1.normalized_form_morpheme().surface())
        self.assertEqual("OOV", m1.dictionary_form_morpheme().surface())
        self.assertTrue(m1.normalized_form_morpheme().is_oov())
        self.assertTrue(m1.dictionary_form_morpheme().is_oov())
        self.assertEqual("OOVs", m2.normalized_form_morpheme().surface())
        self.assertEqual("OOVs", m2.dictionary_form_morpheme().surface())
        self.assertTrue(m2.normalized_form_morpheme().is_oov())
        self.assertTrue(m2.dictionary_form_morpheme().is_oov())


if __name__ == '__main__':
    unittest.main()
