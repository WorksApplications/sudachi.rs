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
import dataclasses
from dataclasses import dataclass, asdict
from json import dumps

@dataclass
class Config:
    """
    SudachiPy rich configuration object.

    Fields passed here will override the fields in the default configuration.
    """
    system: str = None
    """
    Path to dictionary or one of three strings: 'small', 'core', 'notcore'.    
    If the file with the specified path does not exist and is not one of three special values, raise an error.
    If you want to use dictionary with one of predefined names, use relative paths e.g. './core' instead of 'core'.
    
    If the value is one of three special values and there does not exist a file with the same name,
    we try to load the dictionary from the SudachiDict_{system} installed package.
    For example, for "core" we will try to load the dictionary from the installed SudachiDict_core package.
    """
    user: list[str] = None
    """
    Paths to user dictionaries, maximum number of user dictionaries is 14
    """
    projection: str = "surface"
    """
    Output the following field as the result of [Morpheme.surface()] instead of its value.
    This option works for pre-tokenizers created for a given dictionary as well.
    The original value is available as [Morpheme.raw_surface()].
    
    This option is created for chiTra integration.
    
    Available options:
    - surface
    - normalized
    - reading
    - dictionary 
    - dictionary_and_surface 
    - normalized_and_surface
    - normalized_nouns
    """

    connectionCostPlugin: list = None
    oovProviderPlugin: list = None
    pathRewritePlugin: list = None
    inputTextPlugin: list = None
    characterDefinitionFile: str = None

    def as_jsons(self):
        """
        Convert this Config object to the json string
        """
        return dumps(_filter_nulls(asdict(self)))

    def update(self, **changes):
        return dataclasses.replace(self, **changes)


def _filter_nulls(data: dict) -> dict:
    keys = list(data.keys())
    for key in keys:
        v = data[key]
        if v is None:
            del data[key]
    return data
