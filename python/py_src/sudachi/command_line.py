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

import argparse
import fileinput
import logging
import os
import sys

from . import __version__
from . import Dictionary, SplitMode


def _set_default_subparser(self, name, args=None):
    """
    copy and modify code from https://bitbucket.org/ruamel/std.argparse
    """
    subparser_found = False
    for arg in sys.argv[1:]:
        if arg in ['-h', '--help']:  # global help if no subparser
            break
    else:
        for x in self._subparsers._actions:
            if not isinstance(x, argparse._SubParsersAction):
                continue
            for sp_name in x._name_parser_map.keys():
                if sp_name in sys.argv[1:]:
                    subparser_found = True
        if not subparser_found:
            # insert default in first position, this implies no
            # global options without a sub_parsers specified
            if args is None:
                sys.argv.insert(1, name)
            else:
                args.insert(0, name)


argparse.ArgumentParser.set_default_subparser = _set_default_subparser


def run(tokenizer, mode, input_, print_all, stdot_logger, enable_dump):
    for line in input_:
        line = line.rstrip('\n')
        # Note: Current version of the tokenizer ignores logger
        for m in tokenizer.tokenize(line, mode, stdot_logger if enable_dump else None):
            list_info = [
                m.surface(),
                ",".join(m.part_of_speech()),
                m.normalized_form()]
            if print_all:
                list_info += [
                    m.dictionary_form(),
                    m.reading_form(),
                    str(m.dictionary_id()),
                    '[{}]'.format(','.join([str(synonym_group_id) for synonym_group_id in m.synonym_group_ids()]))]
                if m.is_oov():
                    list_info.append("(OOV)")
            stdot_logger.info("\t".join(list_info))
        stdot_logger.info("EOS")


def _input_files_checker(args, print_usage):
    for file in args.in_files:
        if not os.path.exists(file):
            print_usage()
            print('{}: error: {} doesn\'t exist'.format(
                __name__, file), file=sys.stderr)
            exit(1)


def _command_tokenize(args, print_usage):
    if args.version:
        print_version()
        return

    _input_files_checker(args, print_usage)

    if args.mode == "A":
        mode = SplitMode.A
    elif args.mode == "B":
        mode = SplitMode.B
    else:
        mode = SplitMode.C

    stdout_logger = logging.getLogger(__name__)
    output = sys.stdout
    if args.fpath_out:
        output = open(args.fpath_out, "w", encoding="utf-8")
    handler = logging.StreamHandler(output)
    handler.setLevel(logging.DEBUG)
    stdout_logger.addHandler(handler)
    stdout_logger.setLevel(logging.DEBUG)
    stdout_logger.propagate = False

    print_all = args.a
    enable_dump = args.d

    try:
        dict_ = Dictionary(config_path=args.fpath_setting,
                           dict_type=args.system_dict_type)
        tokenizer_obj = dict_.create()
        input_ = fileinput.input(
            args.in_files, openhook=fileinput.hook_encoded("utf-8"))
        run(tokenizer_obj, mode, input_, print_all, stdout_logger, enable_dump)
    finally:
        if args.fpath_out:
            output.close()


def _command_build(args, print_usage):
    raise NotImplementedError(
        "Build dictionary feature is not yet implemented. Please use sudachipy<0.6.")


def _command_user_build(args, print_usage):
    raise NotImplementedError(
        "Build dictionary feature is not yet implemented. Please use sudachipy<0.6.")


def print_version():
    print('sudachipy {}'.format(__version__))


def main():
    parser = argparse.ArgumentParser(
        description="Japanese Morphological Analyzer")

    subparsers = parser.add_subparsers(description='')

    # root, tokenizer parser
    parser_tk = subparsers.add_parser(
        'tokenize', help='(default) see `tokenize -h`', description='Tokenize Text')
    parser_tk.add_argument("-r", dest="fpath_setting",
                           metavar="file", help="the setting file in JSON format")
    parser_tk.add_argument(
        "-m", dest="mode", choices=["A", "B", "C"], default="C", help="the mode of splitting")
    parser_tk.add_argument("-o", dest="fpath_out",
                           metavar="file", help="the output file")
    parser_tk.add_argument("-s", dest="system_dict_type", metavar='string', choices=["small", "core", "full"],
                           help="sudachidict type")
    parser_tk.add_argument("-a", action="store_true",
                           help="print all of the fields")
    parser_tk.add_argument("-d", action="store_true",
                           help="print the debug information")
    parser_tk.add_argument("-v", "--version", action="store_true",
                           dest="version", help="print sudachipy version")
    parser_tk.add_argument("in_files", metavar="file",
                           nargs=argparse.ZERO_OR_MORE, help='text written in utf-8')
    parser_tk.set_defaults(handler=_command_tokenize,
                           print_usage=parser_tk.print_usage)

    # build dictionary parser
    parser_bd = subparsers.add_parser(
        'build', help='see `build -h`', description='Build Sudachi Dictionary')
    parser_bd.add_argument('-o', dest='out_file', metavar='file', default='system.dic',
                           help='output file (default: system.dic)')
    parser_bd.add_argument('-d', dest='description', default='', metavar='string', required=False,
                           help='description comment to be embedded on dictionary')
    required_named_bd = parser_bd.add_argument_group(
        'required named arguments')
    required_named_bd.add_argument('-m', dest='matrix_file', metavar='file', required=True,
                                   help='connection matrix file with MeCab\'s matrix.def format')
    parser_bd.add_argument("in_files", metavar="file", nargs=argparse.ONE_OR_MORE,
                           help='source files with CSV format (one of more)')
    parser_bd.set_defaults(handler=_command_build,
                           print_usage=parser_bd.print_usage)

    # build user-dictionary parser
    parser_ubd = subparsers.add_parser(
        'ubuild', help='see `ubuild -h`', description='Build User Dictionary')
    parser_ubd.add_argument('-d', dest='description', default='', metavar='string', required=False,
                            help='description comment to be embedded on dictionary')
    parser_ubd.add_argument('-o', dest='out_file', metavar='file', default='user.dic',
                            help='output file (default: user.dic)')
    parser_ubd.add_argument('-s', dest='system_dic', metavar='file', required=False,
                            help='system dictionary path (default: system core dictionary path)')
    parser_ubd.add_argument("in_files", metavar="file", nargs=argparse.ONE_OR_MORE,
                            help='source files with CSV format (one or more)')
    parser_ubd.set_defaults(handler=_command_user_build,
                            print_usage=parser_ubd.print_usage)

    parser.set_default_subparser('tokenize')

    args = parser.parse_args()

    if hasattr(args, 'handler'):
        args.handler(args, args.print_usage)
    else:
        parser.print_help()


if __name__ == '__main__':
    main()
