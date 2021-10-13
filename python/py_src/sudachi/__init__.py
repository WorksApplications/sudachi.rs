from .sudachi import dictionary
from .sudachi import tokenizer
from .sudachi import morpheme
from .sudachi import morphemelist

from .sudachi import (
    Dictionary,
    Tokenizer,
    SplitMode,
    MorphemeList,
    Morpheme,
    WordInfo
)

from importlib import import_module
from importlib.util import find_spec
from pathlib import Path


def _get_absolute_dict_path(dict_type: str) -> str:
    pkg_path = Path(import_module(f'sudachidict_{dict_type}').__file__).parent
    dic_path = pkg_path / 'resources' / 'system.dic'
    return str(dic_path.absolute())


def _find_dict_path(dict_type='core'):
    if dict_type not in ['small', 'core', 'full']:
        raise ValueError('"dict_type" must be "small", "core", or "full".')

    is_installed = find_spec(f'sudachidict_{dict_type}')
    if is_installed:
        return _get_absolute_dict_path(dict_type)
    else:
        raise ModuleNotFoundError(
            f'Package `sudachidict_{dict_type}` does not exist. '
            f'You may install it with a command `$ pip install sudachidict_{dict_type}`'
        )