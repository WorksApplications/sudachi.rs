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

from setuptools import setup
from setuptools_rust import Binding, RustExtension

setup(
    name="sudachi",
    version="0.1",
    rust_extensions=[RustExtension("sudachi.sudachi", binding=Binding.PyO3)],
    packages=["sudachi", "sudachi.dictionary", "sudachi.tokenizer",
              "sudachi.morpheme", "sudachi.morphemelist"],
    package_dir={"": "py_src"},
    # rust extensions are not zip safe, just like C-extensions.
    zip_safe=False,
)
