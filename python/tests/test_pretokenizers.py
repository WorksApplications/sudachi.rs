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

import tokenizers
from tokenizers.models import WordLevel

import sudachipy
from sudachipy import MorphemeList
from sudachipy.config import Config


class PretokenizerTestCase(unittest.TestCase):
    def setUp(self) -> None:
        self.dict = sudachipy.Dictionary()

    def test_instantiates_default(self):
        pretok = self.dict.pre_tokenizer()
        self.assertIsNotNone(pretok)

    def test_works_in_simple_pipeline(self):
        pretok = self.dict.pre_tokenizer()
        vocab = {
            "[UNK]": 0,
            "京都": 1,
            "に": 2,
            "行く": 3
        }
        tok = tokenizers.Tokenizer(WordLevel(vocab, unk_token="[UNK]"))
        tok.pre_tokenizer = pretok
        res = tok.encode("京都へ行く")
        self.assertEqual(res.ids, [1, 0, 3])

    def test_works_with_different_split_mode(self):
        pretok = self.dict.pre_tokenizer(sudachipy.SplitMode.A)
        vocab = {
            "[UNK]": 0,
            "外国": 1,
            "参政": 2,
            "権": 3,
            "人": 5,
            "外国人参政権": 4
        }
        tok = tokenizers.Tokenizer(WordLevel(vocab, unk_token="[UNK]"))
        tok.pre_tokenizer = pretok
        res = tok.encode("外国人参政権")
        self.assertEqual(res.ids, [1, 5, 2, 3])

    def test_works_with_different_split_mode_str(self):
        pretok = self.dict.pre_tokenizer(mode='A')
        vocab = {
            "[UNK]": 0,
            "外国": 1,
            "参政": 2,
            "権": 3,
            "人": 5,
            "外国人参政権": 4
        }
        tok = tokenizers.Tokenizer(WordLevel(vocab, unk_token="[UNK]"))
        tok.pre_tokenizer = pretok
        res = tok.encode("外国人参政権")
        self.assertEqual(res.ids, [1, 5, 2, 3])

    def test_with_handler(self):
        def _handler(index, sentence: tokenizers.NormalizedString, ml: MorphemeList):
            return [tokenizers.NormalizedString(ml[0].part_of_speech()[0]), tokenizers.NormalizedString(str(len(ml)))]
        pretok = self.dict.pre_tokenizer(sudachipy.SplitMode.A, handler=_handler)
        vocab = {
            "[UNK]": 0,
            "名詞": 6,
            "4": 7,
        }
        tok = tokenizers.Tokenizer(WordLevel(vocab, unk_token="[UNK]"))
        tok.pre_tokenizer = pretok
        res = tok.encode("外国人参政権")
        self.assertEqual(res.ids, [6, 7])


    def test_with_projection(self):
        pretok = self.dict.pre_tokenizer(sudachipy.SplitMode.A, projection="reading")
        vocab = {
            "[UNK]": 0,
            "ノム": 1,
            "サケ": 2,
            "ヒト": 3,
            "ヲ": 5,
        }
        tok = tokenizers.Tokenizer(WordLevel(vocab, unk_token="[UNK]"))
        tok.pre_tokenizer = pretok
        res = tok.encode("酒を飲む人")
        self.assertEqual([2, 5, 1, 3], res.ids)

    def test_projection_surface_override(self):
        dictobj = sudachipy.Dictionary(config=sudachipy.config.Config(projection="reading"))
        pretok = dictobj.pre_tokenizer(sudachipy.SplitMode.A, projection="surface")
        vocab = {
            "[UNK]": 0,
            "サケ": 1,
            "ヒト": 2,
            "ノム": 3,
            "ヲ": 5,
            "外国人参政権": 4
        }
        tok = tokenizers.Tokenizer(WordLevel(vocab, unk_token="[UNK]"))
        tok.pre_tokenizer = pretok
        res = tok.encode("酒を飲む人")
        self.assertEqual(res.ids, [1, 5, 3, 2])


if __name__ == '__main__':
    unittest.main()
