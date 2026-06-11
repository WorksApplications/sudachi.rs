# Copyright (c) 2026 Works Applications Co., Ltd.
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
from pathlib import Path

from sudachipy import Config, Dictionary, TextNormalizer


ORIGINAL_TEXT = "ÂＢΓД㈱ｶﾞウ゛⼼Ⅲ"
NORMALIZED_TEXT = "âbγд(株)ガヴ⼼ⅲ"


class TestTextNormalizer(unittest.TestCase):
    def setUp(self):
        self.resource_dir = Path(__file__).parent / "resources"
        self.dict_ = Dictionary(
            os.path.join(self.resource_dir, "sudachi.json"),
            resource_dir=self.resource_dir,
        )

    def tearDown(self):
        self.dict_.close()

    def test_dictionary_text_normalizer(self):
        normalizer = self.dict_.text_normalizer()
        self.assertIsInstance(normalizer, TextNormalizer)
        self.assertEqual(NORMALIZED_TEXT, normalizer.normalize(ORIGINAL_TEXT))

    def test_default_text_normalizer(self):
        normalizer = TextNormalizer()
        self.assertEqual(NORMALIZED_TEXT, normalizer.normalize(ORIGINAL_TEXT))

    def test_text_normalizer_constructor(self):
        normalizer = TextNormalizer(self.dict_)
        self.assertEqual("abc", normalizer.normalize("ＡＢＣ"))

    def test_repeated_calls_and_empty_text(self):
        normalizer = self.dict_.text_normalizer()
        self.assertEqual("", normalizer.normalize(""))
        self.assertEqual("abc", normalizer.normalize("ＡＢＣ"))
        self.assertEqual(NORMALIZED_TEXT, normalizer.normalize(ORIGINAL_TEXT))

    def test_normalize_rejects_non_string_input(self):
        normalizer = self.dict_.text_normalizer()
        with self.assertRaises(TypeError):
            normalizer.normalize(1)

    def test_normalize_is_not_morpheme_normalized_form(self):
        normalizer = self.dict_.text_normalizer()
        self.assertEqual("附属", normalizer.normalize("附属"))

    def test_uses_dictionary_input_text_plugins(self):
        dictionary = Dictionary(config=Config(
            system=str(self.resource_dir / "system.dic.test"),
            characterDefinitionFile=str(self.resource_dir / "char.def"),
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
            normalizer = dictionary.text_normalizer()
            self.assertEqual("京都", normalizer.normalize("京都（キョウト）"))
        finally:
            dictionary.close()


if __name__ == "__main__":
    unittest.main()
