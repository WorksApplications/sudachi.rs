#   Copyright (c) 2024-2026 Works Applications Co., Ltd.
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
from typing import ClassVar, Iterator, List, Tuple, Union, Callable, Iterable, Optional, Literal, Set
from .config import Config

# Part Of Speech
POS = Tuple[str, str, str, str, str, str]
# POS element
PE = Optional[str]
PartialPOS = Union[
    Tuple[PE, PE, PE, PE, PE, PE],
    Tuple[PE, PE, PE, PE, PE],
    Tuple[PE, PE, PE, PE],
    Tuple[PE, PE, PE],
    Tuple[PE, PE],
    Tuple[PE],
    Tuple[()],
]

"""
Fields that can be specified for partial dictionary loading.
See https://worksapplications.github.io/sudachi.rs/python/topics/subsetting.html.
"""
FieldSet = Optional[Set[Literal["surface", "pos", "normalized_form", "dictionary_form", "reading_form",
                                "word_structure", "split_a", "split_b", "synonym_group_id",
                                "user_data"]]]


"""
Strings that can be parsed as SplitMode
"""
SplitModeStr = Literal["A", "a", "B", "b", "C", "c"]


class SplitMode:
    """
    Unit to split text.

    A == short mode

    B == middle mode

    C == long mode
    """

    A: ClassVar[SplitMode] = ...
    B: ClassVar[SplitMode] = ...
    C: ClassVar[SplitMode] = ...

    @classmethod
    def __init__(cls, mode: Optional[SplitModeStr] = "C") -> None:
        """
        Creates a split mode from a string value.

        :param mode: string representation of the split mode. One of [A,B,C] in captital or lower case.
            If None, returns SplitMode.C.
        """
        ...


class Dictionary:
    """
    A sudachi dictionary.
    """

    @classmethod
    def __init__(cls, config_path: Optional[str | Config] = ..., resource_dir: Optional[str] = ..., dict: Optional[str] = None,
                 dict_type: Optional[str] = None, *, config: Optional[str | Config] = ...) -> None:
        """
        Creates a sudachi dictionary.

        If both config.systemDict and dict are not given, `sudachidict_core` is used.
        If both config.systemDict and dict are given, dict is used.
        If dict is an absolute path to a file, it is used as a dictionary.

        :param config_path: path to the configuration JSON file, config json as a string, or a [sudachipy.Config] object.
        :param config: alias to config_path, only one of them can be specified at the same time.
        :param resource_dir: path to the resource directory folder.
        :param dict: type of pre-packaged system dictionary, referring to sudachidict_<dict> packages on PyPI: https://pypi.org/search/?q=sudachidict.
            Also, can be an _absolute_ path to a compiled dictionary file.
        :param dict_type: deprecated alias to dict.
        """
        ...

    def close(self) -> None:
        """
        Close this dictionary.
        """
        ...

    def create(self,
               mode: Union[SplitMode, SplitModeStr, None] = SplitMode.C,
               fields: Optional[FieldSet] = None,
               *,
               projection: Optional[str] = None) -> Tokenizer:
        """
        Creates a sudachi tokenizer.

        :param mode: sets the analysis mode for this Tokenizer
        :param fields: load only a subset of fields.
            See https://worksapplications.github.io/sudachi.rs/python/topics/subsetting.html.
        :param projection: Projection override for created Tokenizer. See Config.projection for values.
        """
        ...

    def text_normalizer(self) -> TextNormalizer:
        """
        Creates a text normalizer from this dictionary.

        The returned normalizer applies the same input-text plugins that this
        dictionary uses before tokenization.
        """
        ...

    def pos_matcher(self, target: Union[Iterable[PartialPOS], Callable[[POS], bool]]) -> PosMatcher:
        """
        Creates a new POS matcher.

        If target is a function, then it must return whether a POS should match or not.
        If target is a list, it should contain partially specified POS.
        By partially specified it means that it is possible to omit POS fields or use None as a sentinel value that matches any POS.

        For example, ('名詞',) will match any noun and
        (None, None, None, None, None, '終止形') will match any word in 終止形 conjugation form.

        :param target: can be either a list of POS partial tuples or a callable which maps POS to bool.
        """
        ...

    def pre_tokenizer(self,
                      mode: Union[SplitMode, SplitModeStr, None] = SplitMode.C,
                      fields: Optional[FieldSet] = None,
                      handler: Optional[Callable[[
                          int, object, MorphemeList], list]] = None,
                      *,
                      projection: Optional[str] = None) -> object:
        """
        Creates HuggingFace Tokenizers-compatible PreTokenizer.
        Requires package `tokenizers` to be installed.

        :param mode: Use this split mode (C by default)
        :param fields: ask Sudachi to load only a subset of fields. See https://worksapplications.github.io/sudachi.rs/python/topics/subsetting.html.
        :param handler: a custom callable to transform MorphemeList into list of tokens. If None, simply use surface as token representations.
            It should be a `function(index: int, original: NormalizedString, morphemes: MorphemeList) -> List[NormalizedString]`.
            See https://github.com/huggingface/tokenizers/blob/master/bindings/python/examples/custom_components.py.
        :param projection: Projection override for created Tokenizer. See Config.projection for values.
        """
        ...

    def pos_of(self, pos_id: int) -> Optional[POS]:
        """
        Returns POS with the given id.

        :param pos_id: POS id
        :return: POS tuple with the given id or None for non existing id.
        """
        ...

    def entries(self) -> Iterator[Morpheme]:
        """
        Iterates over public lexicon entries as morphemes.

        Entries referred to from other entries, such as split or constituent
        units, are included even when they are not indexed for normal lookup.
        Internal entries automatically generated for literal normalized forms
        are not exposed. The iteration order is not part of the public contract.
        """
        ...

    def lookup(self, surface: str, out: Optional[MorphemeList] = None) -> MorphemeList:
        """
        Look up morphemes in the binary dictionary without performing the analysis.

        The given surface is normalized before lookup.
        All morphemes from the dictionary with the normalized surface string are returned,
        with the last user dictionary searched first and the system dictionary searched last.
        Inside a dictionary, morphemes are outputted in-binary-dictionary order.
        Morphemes which are not indexed are not returned.

        :param surface: input surface; normalized before indexed lookup.
        :param out: if passed, reuse the given morpheme list instead of creating a new one.
            See https://worksapplications.github.io/sudachi.rs/python/topics/out_param.html for details.
        """
        ...

    def lookup_all_entries(self, surface: str, out: Optional[MorphemeList] = None) -> MorphemeList:
        """
        Look up morphemes by scanning all dictionary entries.

        The given surface is normalized before matching. This scans public
        lexicon entries and can find entries which are not indexed for normal
        lookup. This can be slow on large dictionaries; use `lookup()` for
        normal indexed lookup.

        :param surface: find all morphemes whose normalized surface matches this value
        :param out: if passed, reuse the given morpheme list instead of creating a new one.
            See https://worksapplications.github.io/sudachi.rs/python/topics/out_param.html for details.
        """
        ...

    def oov_morpheme(
        self,
        pos_id: int,
        surface: str,
        reading: Optional[str] = None,
        normalized_form: Optional[str] = None,
        dictionary_form: Optional[str] = None,
    ) -> Morpheme:
        """
        Create an out-of-vocabulary morpheme from the POS id and string forms.

        Begin/end are set from the surface. When optional string forms are not
        provided, the surface is used for them.

        :param pos_id: part-of-speech id of the morpheme
        :param surface: surface of the morpheme
        :param reading: reading form of the morpheme
        :param normalized_form: normalized form of the morpheme
        :param dictionary_form: dictionary form of the morpheme
        """
        ...


class TextNormalizer:
    """
    A text normalizer.

    Applies input-text plugins to raw input text. This does not perform
    morphological analysis or return morpheme normalized forms.

    Create using ``Dictionary.text_normalizer()`` or by passing a ``Dictionary``
    to this class. Without a dictionary, this uses the default input-text
    normalization.
    """

    def __init__(self, dictionary: Dictionary | None = None) -> None:
        """
        Creates a text normalizer.

        When dictionary is provided, this applies the same input-text plugins
        used by that dictionary before tokenization. Without a dictionary, this
        applies the default input-text normalization.
        """
        ...

    def normalize(self, text: str) -> str:
        """
        Normalize text using input-text plugins.

        This normalizes tokenizer input text, not the dictionary-normalized form
        returned by ``Morpheme.normalized_form()``.

        :param text: text to normalize.
        """
        ...


class Morpheme:
    """
    A morpheme (basic semantic unit of language).
    """

    def __init__(self) -> None: ...

    def begin(self) -> int:
        """
        Returns the begin index of this in the input text.
        """
        ...

    def dictionary_form(self) -> str:
        """
        Returns the dictionary form.
        """
        ...

    def dictionary_form_morpheme(self) -> Morpheme:
        """
        Returns the morpheme corresponding to this morpheme's dictionary form.
        """
        ...

    def dictionary_id(self) -> int:
        """
        Returns the dictionary id which this word belongs.
        """
        ...

    def end(self) -> int:
        """
        Returns the end index of this in the input text.
        """
        ...

    def is_oov(self) -> bool:
        """
        Returns whether if this is out of vocabulary word.
        """
        ...

    def normalized_form(self) -> str:
        """
        Returns the normalized form.
        """
        ...

    def normalized_form_morpheme(self) -> Morpheme:
        """
        Returns the morpheme corresponding to this morpheme's normalized form.
        """
        ...

    def part_of_speech(self) -> POS:
        """
        Returns the part of speech as a six-element tuple.
        Tuple elements are four POS levels, conjugation type and conjugation form.
        """
        ...

    def part_of_speech_id(self) -> int:
        """
        Returns the id of the part of speech in the dictionary.
        """
        ...

    def reading_form(self) -> str:
        """
        Returns the reading form.
        """
        ...

    def split(self,
              mode: Union[SplitMode, SplitModeStr],
              out: Optional[MorphemeList] = None,
              add_single: bool = True) -> MorphemeList:
        """
        Returns sub-morphemes in the provided split mode.

        :param mode: mode of new split.
        :param out: write results to this MorphemeList instead of creating new one.
            See https://worksapplications.github.io/sudachi.rs/python/topics/out_param.html for
            more information on output parameters.
            Returned MorphemeList will be invalidated if this MorphemeList is used as an output parameter.
        :param add_single: return lists with the current morpheme if the split hasn't produced any elements.
            When False is passed, empty lists are returned instead.
        """
        ...

    def surface(self) -> str:
        """
        Returns the substring of input text corresponding to the morpheme, or a projection if one is configured.

        See `Config.projection`.
        """
        ...

    def raw_surface(self) -> str:
        """
        Returns the substring of input text corresponding to the morpheme regardless the configured projection.

        See `Config.projection`.
        """
        ...

    def synonym_group_ids(self) -> List[int]:
        """
        Returns the list of synonym group ids.
        """
        ...

    def user_data(self) -> str:
        """
        Returns user-defined data associated with this morpheme.
        """
        ...

    def word_id(self) -> int:
        """
        Returns word id of this word in the dictionary.
        """
        ...

    def __len__(self) -> int:
        """
        Returns morpheme length in codepoints.
        """


class MorphemeList:
    """
    A list of morphemes.

    An object can not be instantiated manually.
    Use Tokenizer.tokenize("") to create an empty morpheme list.
    """

    def __init__(self) -> None: ...

    @classmethod
    def empty(cls, dict: Dictionary) -> MorphemeList:
        """
        Returns an empty morpheme list with dictionary.

        .. deprecated::
            Use Tokenizer.tokenize("") if you need.
        """
        ...

    def get_internal_cost(self) -> int:
        """
        Returns the total cost of the path.
        """
        ...

    def size(self) -> int:
        """
        Returns the number of morpheme in this list.
        """
        ...

    def __getitem__(self, index: int) -> Morpheme: ...
    def __iter__(self) -> Iterator[Morpheme]: ...
    def __len__(self) -> int: ...


class Tokenizer:
    """
    A sudachi tokenizer

    Create using Dictionary.create method.
    """
    SplitMode: ClassVar[SplitMode] = ...

    @classmethod
    def __init__(cls) -> None: ...

    def tokenize(self,
                 text: str,
                 mode: Union[SplitMode, SplitModeStr, None] = None,
                 out: Optional[MorphemeList] = None) -> MorphemeList:
        """
        Break text into morphemes.

        :param text: text to analyze.
        :param mode: analysis mode.
            This parameter is deprecated.
            Pass the analysis mode at the Tokenizer creation time and create different tokenizers for different modes.
            If you need multi-level splitting, prefer using :py:meth:`Morpheme.split` method instead.
        :param logger: Arg for v0.5.* compatibility. Ignored.
        :param out: tokenization results will be written into this MorphemeList, a new one will be created instead.
            See https://worksapplications.github.io/sudachi.rs/python/topics/out_param.html for details.
        """
        ...

    @property
    def mode(self) -> SplitMode:
        """
        Get the current analysis mode
        :return: current analysis mode
        """
        ...


class PosMatcher:
    """
    A part-of-speech matcher which checks if a morpheme belongs to a set of part of speech.

    Create using Dictionary.pos_matcher method.
    """

    def __iter__(self) -> Iterator[POS]: ...
    def __len__(self) -> int: ...

    def __call__(self, /, m: Morpheme) -> bool:
        """
        Checks whether a morpheme has matching POS.

        :param m: a morpheme to check.
        :return: if morpheme has matching POS.
        """
        ...

    def __or__(self, other: PosMatcher) -> PosMatcher:
        """
        Returns a POS matcher which matches a POS if any of two matchers would match it.
        """
        ...

    def __and__(self, other: PosMatcher) -> PosMatcher:
        """
        Returns a POS matcher which matches a POS if both matchers would match it at the same time.
        """
        ...

    def __sub__(self, other: PosMatcher) -> PosMatcher:
        """
        Returns a POS matcher which matches a POS if self would match the POS and other would not match the POS.
        """
        ...

    def __invert__(self) -> PosMatcher:
        """
        Returns a POS matcher which matches all POS tags except ones defined in the current POS matcher.
        """
        ...
