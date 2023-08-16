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

import tempfile
import unittest
from pathlib import Path

import sudachipy
from sudachipy.config import Config
from dataclasses import replace

FILE_PATH = Path(__file__)
RESOURCES_PATH = FILE_PATH.parent / "resources"
CFG_TEMPLATE = Config(
    oovProviderPlugin=[
        { "class" : "com.worksap.nlp.sudachi.SimpleOovPlugin",
          "oovPOS" : [ "名詞", "普通名詞", "一般", "*", "*", "*" ],
          "leftId" : 8,
          "rightId" : 8,
          "cost" : 6000 }
    ]
)


class MyTestCase(unittest.TestCase):
    def setUp(self) -> None:
        self.tempfiles = []
        self.tmpdir = tempfile.mkdtemp("sudachi", "test")
        super().setUp()

    def tearDown(self) -> None:
        for f in self.tempfiles:
            p = Path(f)
            if p.exists():
                p.unlink()
        Path(self.tmpdir).rmdir()
        super().tearDown()

    def test_build_system(self):
        out_tmp = tempfile.mktemp(prefix="sudachi_sy", suffix=".dic", dir=self.tmpdir)
        self.tempfiles.append(out_tmp)
        stats = sudachipy.sudachipy.build_system_dic(
            matrix=RESOURCES_PATH / "matrix.def",
            lex=[RESOURCES_PATH / "lex.csv"],
            output=out_tmp
        )
        self.assertIsNotNone(stats)
        cfg = replace(CFG_TEMPLATE, system=out_tmp)
        dict = sudachipy.Dictionary(config_path=cfg)
        tok = dict.create()
        result = tok.tokenize("東京にいく")
        self.assertEqual(result.size(), 3)

    def test_build_user1(self):
        sys_dic = tempfile.mktemp(prefix="sudachi_sy", suffix=".dic", dir=self.tmpdir)
        self.tempfiles.append(sys_dic)
        sudachipy.sudachipy.build_system_dic(
            matrix=RESOURCES_PATH / "matrix.def",
            lex=[RESOURCES_PATH / "lex.csv"],
            output=sys_dic
        )
        u1_dic = tempfile.mktemp(prefix="sudachi_u1", suffix=".dic", dir=self.tmpdir)
        self.tempfiles.append(u1_dic)
        sudachipy.sudachipy.build_user_dic(
            system=sys_dic,
            lex=[RESOURCES_PATH / "user1.csv"],
            output=u1_dic
        )

        cfg = replace(CFG_TEMPLATE, system=sys_dic, user=[u1_dic])
        dict = sudachipy.Dictionary(config=cfg)
        tok = dict.create()
        result = tok.tokenize("すだちにいく")
        self.assertEqual(result.size(), 3)
        self.assertEqual(result[0].dictionary_id(), 1)

    def test_build_user2(self):
        sys_dic = tempfile.mktemp(prefix="sudachi_sy", suffix=".dic", dir=self.tmpdir)
        self.tempfiles.append(sys_dic)
        sudachipy.sudachipy.build_system_dic(
            matrix=RESOURCES_PATH / "matrix.def",
            lex=[RESOURCES_PATH / "lex.csv"],
            output=sys_dic
        )
        u1_dic = tempfile.mktemp(prefix="sudachi_u1", suffix=".dic", dir=self.tmpdir)
        self.tempfiles.append(u1_dic)
        sudachipy.sudachipy.build_user_dic(
            system=sys_dic,
            lex=[RESOURCES_PATH / "user1.csv"],
            output=u1_dic
        )

        u2_dic = tempfile.mktemp(prefix="sudachi_u2", suffix=".dic", dir=self.tmpdir)
        self.tempfiles.append(u2_dic)
        sudachipy.sudachipy.build_user_dic(
            system=sys_dic,
            lex=[RESOURCES_PATH / "user2.csv"],
            output=u2_dic
        )

        cfg = replace(CFG_TEMPLATE, system=sys_dic, user=[u1_dic, u2_dic])
        dict = sudachipy.Dictionary(config_path=cfg)
        tok = dict.create()
        result = tok.tokenize("かぼすにいく")
        self.assertEqual(result.size(), 3)
        self.assertEqual(result[0].dictionary_id(), 2)
        self.assertEqual(result[0].part_of_speech()[0], "被子植物門")


if __name__ == '__main__':
    unittest.main()
