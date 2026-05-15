#   Copyright (c) 2021-2026 Works Applications Co., Ltd.
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
        self.tempdirs = []
        self.tempfiles = []
        self.tmpdir = tempfile.mkdtemp("sudachi", "test")
        super().setUp()

    def tearDown(self) -> None:
        for f in self.tempfiles:
            p = Path(f)
            if p.exists():
                p.unlink()
        for d in self.tempdirs:
            p = Path(d)
            if p.exists():
                p.rmdir()
        Path(self.tmpdir).rmdir()
        super().tearDown()

    def make_tempfile(self, prefix, suffix):
        tmpdir = tempfile.mkdtemp(prefix=prefix, dir=self.tmpdir)
        self.tempdirs.append(tmpdir)
        path = Path(tmpdir) / f"tmp{suffix}"
        self.tempfiles.append(str(path))
        return str(path)

    def test_build_system(self):
        out_tmp = self.make_tempfile("sudachi_sy", ".dic")
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
        sys_dic = self.make_tempfile("sudachi_sy", ".dic")
        sudachipy.sudachipy.build_system_dic(
            matrix=RESOURCES_PATH / "matrix.def",
            lex=[RESOURCES_PATH / "lex.csv"],
            output=sys_dic
        )
        u1_dic = self.make_tempfile("sudachi_u1", ".dic")
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
        sys_dic = self.make_tempfile("sudachi_sy", ".dic")
        sudachipy.sudachipy.build_system_dic(
            matrix=RESOURCES_PATH / "matrix.def",
            lex=[RESOURCES_PATH / "lex.csv"],
            output=sys_dic
        )
        u1_dic = self.make_tempfile("sudachi_u1", ".dic")
        sudachipy.sudachipy.build_user_dic(
            system=sys_dic,
            lex=[RESOURCES_PATH / "user1.csv"],
            output=u1_dic
        )

        u2_dic = self.make_tempfile("sudachi_u2", ".dic")
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

    def test_reject_incompatible_user_dictionary(self):
        sys_dic = self.make_tempfile("sudachi_sy", ".dic")
        sudachipy.sudachipy.build_system_dic(
            matrix=RESOURCES_PATH / "matrix.def",
            lex=[RESOURCES_PATH / "lex.csv"],
            output=sys_dic,
            description="system"
        )

        another_sys_dic = self.make_tempfile("sudachi_sy2", ".dic")
        sudachipy.sudachipy.build_system_dic(
            matrix=RESOURCES_PATH / "matrix.def",
            lex=[RESOURCES_PATH / "lex.csv"],
            output=another_sys_dic,
            description="another"
        )

        incompatible_user_dic = self.make_tempfile("sudachi_u1", ".dic")
        sudachipy.sudachipy.build_user_dic(
            system=another_sys_dic,
            lex=[RESOURCES_PATH / "user1.csv"],
            output=incompatible_user_dic
        )

        cfg = replace(CFG_TEMPLATE, system=sys_dic, user=[incompatible_user_dic])
        with self.assertRaises(sudachipy.errors.SudachiError) as err:
            sudachipy.Dictionary(config=cfg)

        self.assertIn("Error while constructing dictionary", str(err.exception))
        self.assertIn("user dictionary is not compatible with the system dictionary", str(err.exception))


if __name__ == '__main__':
    unittest.main()
