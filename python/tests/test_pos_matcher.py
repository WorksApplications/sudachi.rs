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
import os
import unittest

import sudachipy
from sudachipy import Dictionary


class PosMatcherTestCase(unittest.TestCase):

    def setUp(self):
        resource_dir = os.path.join(os.path.dirname(
            os.path.abspath(__file__)), 'resources')
        self.dict = Dictionary(os.path.join(
            resource_dir, 'sudachi.json'), resource_dir)
        self.tokenizer_obj = self.dict.create()

    def test_create_fn(self):
        m = self.dict.pos_matcher(lambda p: p[0] == "名詞")
        self.assertIsNotNone(m)

    def test_create_declarative(self):
        m = self.dict.pos_matcher([("名詞",)])
        self.assertEqual(list(m), [('名詞', '固有名詞', '地名', '一般', '*', '*'),
                                   ('名詞', '普通名詞', '一般', '*', '*', '*'),
                                   ('名詞', '数詞', '*', '*', '*', '*')])

    def test_create_declarative_empty(self):
        m = self.dict.pos_matcher([()])
        self.assertEqual(len(m), 9)

    def test_iter(self):
        m = self.dict.pos_matcher(lambda p: p[0] != "名詞")
        self.assertEqual(list(m), [('助動詞', '*', '*', '*', '助動詞-タ', '終止形-一般'),
                                   ('助詞', '接続助詞', '*', '*', '*', '*'),
                                   ('助詞', '格助詞', '*', '*', '*', '*'),
                                   ('動詞', '非自立可能', '*', '*', '五段-カ行', '終止形-一般'),
                                   ('動詞', '非自立可能', '*', '*', '五段-カ行', '連用形-促音便'),
                                   ('被子植物門', '双子葉植物綱', 'ムクロジ目', 'ミカン科', 'ミカン属', 'スダチ')])

    def test_skips(self):
        m = self.dict.pos_matcher([(None, None, None, None, None, "終止形-一般")])
        self.assertEqual(list(m), [('助動詞', '*', '*', '*', '助動詞-タ', '終止形-一般'),
                                   ('動詞', '非自立可能', '*', '*', '五段-カ行', '終止形-一般')])

    def test_match_nouns(self):
        tok = self.dict.create(mode=sudachipy.SplitMode.A)
        m = self.dict.pos_matcher([("名詞",)])
        data = [x.surface() for x in tok.tokenize("東京に行く") if m(x)]
        self.assertEqual(data, ["東京"])

    def test_union(self):
        m1 = self.dict.pos_matcher(lambda p: p[0] == "名詞")
        m2 = self.dict.pos_matcher(lambda p: p[0] == "動詞")
        m3 = m1 | m2
        self.assertEqual(len(m1) + len(m2), len(m3))

    def test_intersection(self):
        m1 = self.dict.pos_matcher(lambda p: p[5] == "終止形-一般")
        m2 = self.dict.pos_matcher(lambda p: p[0] == "動詞")
        m3 = m1 & m2
        self.assertEqual(1, len(m3))
        self.assertEqual(list(m3), [('動詞', '非自立可能', '*', '*', '五段-カ行', '終止形-一般')])

    def test_difference(self):
        m1 = self.dict.pos_matcher(lambda p: p[5] == "終止形-一般")
        m2 = self.dict.pos_matcher(lambda p: p[0] == "動詞")
        m3 = m1 - m2
        self.assertEqual(1, len(m3))
        self.assertEqual(list(m3), [('助動詞', '*', '*', '*', '助動詞-タ', '終止形-一般')])

    def test_invert(self):
        m1 = self.dict.pos_matcher(lambda p: p[5] == "終止形-一般")
        self.assertEqual(len(m1), 2)
        self.assertEqual(len(~m1), 7)


if __name__ == '__main__':
    unittest.main()
