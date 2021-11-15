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
import tokenizers.pre_tokenizers
from tokenizers.tokenizers import PreTokenizedString


class SudachiPreTokenizer(tokenizers.pre_tokenizers.PreTokenizer):
    def __init__(self, internal):
        """Do not create this object manually, use Dictionary.create_pretokenizer method instead"""
        self.internal = internal

    def pre_tokenize(self, data: PreTokenizedString):
        data.split(self.internal)

    def __repr__(self):
        return repr(self.internal)