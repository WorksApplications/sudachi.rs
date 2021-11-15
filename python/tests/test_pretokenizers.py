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


if __name__ == '__main__':
    unittest.main()
