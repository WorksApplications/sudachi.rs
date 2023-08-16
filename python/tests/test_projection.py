#   Copyright (c) 2023 Works Applications Co., Ltd.
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

from sudachipy import Dictionary
from sudachipy.config import Config

class MyTestCase(unittest.TestCase):
    def setUp(self) -> None:
        self.dict = Dictionary(config=Config(projection="reading"))

    def test_projection_surface_override(self):
        tok = self.dict.create(projection="surface")
        morphs = tok.tokenize("酒を飲む人")
        self.assertEqual(4, morphs.size())
        self.assertEqual("酒", morphs[0].surface())
        self.assertEqual("を", morphs[1].surface())
        self.assertEqual("飲む", morphs[2].surface())
        self.assertEqual("人", morphs[3].surface())

    def test_projection_reading(self):
        tok = self.dict.create()
        morphs = tok.tokenize("酒を飲む人")
        self.assertEqual(4, morphs.size())
        self.assertEqual("サケ", morphs[0].surface())
        self.assertEqual("ヲ", morphs[1].surface())
        self.assertEqual("ノム", morphs[2].surface())
        self.assertEqual("ヒト", morphs[3].surface())

    def test_projection_dictionary(self):
        tok = self.dict.create(projection="dictionary")
        morphs = tok.tokenize("酒を飲まなかった人")
        self.assertEqual(6, morphs.size())
        self.assertEqual("酒", morphs[0].surface())
        self.assertEqual("を", morphs[1].surface())
        self.assertEqual("飲む", morphs[2].surface())
        self.assertEqual("ない", morphs[3].surface())
        self.assertEqual("た", morphs[4].surface())
        self.assertEqual("人", morphs[5].surface())

    def test_projection_normalized(self):
        tok = self.dict.create(projection="normalized")
        morphs = tok.tokenize("MEGAへ行く")
        self.assertEqual(3, morphs.size())
        self.assertEqual("メガ", morphs[0].surface())
        self.assertEqual("MEGA", morphs[0].raw_surface())
        self.assertEqual("へ", morphs[1].surface())
        self.assertEqual("行く", morphs[2].surface())

    def test_projection_dictionary_and_surface(self):
        tok = self.dict.create(projection="dictionary_and_surface")
        morphs = tok.tokenize("酒を飲まなかった人")
        self.assertEqual(6, morphs.size())
        self.assertEqual("酒", morphs[0].surface())
        self.assertEqual("を", morphs[1].surface())
        self.assertEqual("飲ま", morphs[2].surface())
        self.assertEqual("なかっ", morphs[3].surface())
        self.assertEqual("た", morphs[4].surface())
        self.assertEqual("人", morphs[5].surface())

    def test_projection_normalized_and_surface(self):
        tok = self.dict.create(projection="normalized_and_surface")
        morphs = tok.tokenize("MEGAへ行こう")
        self.assertEqual(3, morphs.size())
        self.assertEqual("メガ", morphs[0].surface())
        self.assertEqual("MEGA", morphs[0].raw_surface())
        self.assertEqual("へ", morphs[1].surface())
        self.assertEqual("行こう", morphs[2].surface())

    def test_projection_normalized_nouns(self):
        tok = self.dict.create(projection="normalized_nouns")
        morphs = tok.tokenize("MEGAへ行こう")
        self.assertEqual(3, morphs.size())
        self.assertEqual("メガ", morphs[0].surface())
        self.assertEqual("MEGA", morphs[0].raw_surface())
        self.assertEqual("へ", morphs[1].surface())
        self.assertEqual("行こう", morphs[2].surface())


if __name__ == '__main__':
    unittest.main()
