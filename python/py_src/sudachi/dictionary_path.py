from importlib import import_module
from importlib.util import find_spec
from pathlib import Path


def get_absolute_dict_path(dict_type: str) -> str:
    pkg_path = Path(import_module('sudachidict_' + dict_type).__file__).parent
    dic_path = pkg_path / 'resources' / 'system.dic'
    return str(dic_path.absolute())


def find_dict_path(dict_type='core'):
    is_installed = find_spec('sudachidict_{}'.format(dict_type))
    if is_installed:
        return get_absolute_dict_path(dict_type)
    else:
        raise ModuleNotFoundError(
            'Package `sudachidict_{}` dose not exist. '
            'You may install it with a command `$ pip install sudachidict_{}`'.format(
                dict_type, dict_type)
        )
