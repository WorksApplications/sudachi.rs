from typing import ClassVar, Iterator, List, Tuple, Union, Callable, Iterable, Optional, Literal, Set

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

FieldSet = Optional[Set[Literal["surface", "pos", "normalized_form", "dictionary_form", "reading_form",
                                "word_structure", "split_a", "split_b", "synonym_group_id"]]]

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
    def __init__(cls) -> None: ...


class Dictionary:
    """
    A sudachi dictionary.
    """

    @classmethod
    def __init__(cls, config_path: Optional[str] = ..., resource_dir: Optional[str] = ..., dict: Optional[str] = None, *, dict_type: Optional[str] = None) -> None:
        """
        Creates a sudachi dictionary.

        If both config.systemDict and dict_type are not given, `sudachidict_core` is used.
        If both config.systemDict and dict_type are given, dict_type is used.

        :param config_path: path to the configuration JSON file
        :param resource_dir: path to the resource directory folder
        :param dict: type of pre-packaged dictionary, referring to sudachidict_<dict> packages on PyPI: https://pypi.org/search/?q=sudachidict.
            Also, can be an _absolute_ path to a compiled dictionary file.
        :param dict_type: deprecated alias to dict
        """
        ...

    def close(self) -> None:
        """
        Close this dictionary.
        """
        ...

    def create(self,
               mode: SplitMode = SplitMode.C,
               fields: FieldSet = None) -> Tokenizer:
        """
        Creates a Sudachi Tokenizer.

        :param mode: sets the analysis mode for this Tokenizer
        :param fields: load only a subset of fields.
            See https://worksapplications.github.io/sudachi.rs/python/topics/subsetting.html
        """
        ...

    def pos_matcher(self, target: Union[Iterable[PartialPOS], Callable[[POS], bool]]) -> PosMatcher:
        """
        Creates a new POS matcher.

        If target is a function, then it must return whether a POS should match or not.
        If target a list, it should contain partially specified POS.
        By partially specified it means that it is possible to omit POS fields or
        use None as a sentinel value that matches any POS.

        For example, ('名詞',) will match any noun and
        (None, None, None, None, None, '終止形') will match any word in 終止形 conjugation form.

        :param target: can be either a function or a list of POS tuples.
        """
        ...

    def pre_tokenizer(self,
                      mode: SplitMode = SplitMode.C,
                      fields: FieldSet = None,
                      handler: Optional[Callable[[int, object, MorphemeList], list]] = None) -> object:
        """
        Creates HuggingFace Tokenizers-compatible PreTokenizer.
        Requires package `tokenizers` to be installed.

        :param mode: Use this split mode (C by default)
        :param fields: ask Sudachi to load only a subset of fields. See https://worksapplications.github.io/sudachi.rs/python/topics/subsetting.html
        :param handler: custom callable to transform MorphemeList into list of tokens. See https://github.com/huggingface/tokenizers/blob/master/bindings/python/examples/custom_components.py
        First two parameters are the index (int) and HuggingFace NormalizedString.
        The handler must return a List[NormalizedString]. By default, just segment the tokens.
        """
        ...

    def pos_of(self, pos_id: int) -> Optional[POS]:
        """
        Returns POS with the given id.

        :param pos_id: POS id
        :return: POS tuple with the given id.
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

    def get_word_info(self) -> WordInfo:
        """
        Returns the word info.
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

    def part_of_speech(self) -> POS:
        """
        Returns the part of speech.
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

    def split(self, mode: SplitMode, out: Optional[MorphemeList] = None, add_single: bool = True) -> MorphemeList:
        """
        Returns sub-morphemes in the provided split mode.

        :param mode: mode of new split
        :param out: write results to this MorhpemeList instead of creating new one
            See https://worksapplications.github.io/sudachi.rs/python/topics/out_param.html for
            more information on output parameters.
            Returned MorphemeList will be invalidated if this MorphemeList is used as an output parameter.
        :param add_single: return lists with the current morpheme if the split hasn't produced any elements.
            When False is passed, empty lists are returned instead.
        """
        ...

    def surface(self) -> str:
        """
        Returns the surface.
        """
        ...

    def synonym_group_ids(self) -> List[int]:
        """
        Returns the list of synonym group ids.
        """
        ...

    def word_id(self) -> int:
        """
        Returns word id of this word in the dictionary.
        """
        ...

    def __len__(self) -> int:
        """
        Returns morpheme length in codepoints
        """


class MorphemeList:
    """
    A list of morphemes.
    An object can not be instantiated manually.
    Use Tokenizer.tokenize("") to create an empty morpheme list.
    """
    def __init__(self) -> None: ...

    @classmethod
    def empty(cls, dict) -> MorphemeList:
        """
        Returns an empty morpheme list with dictionary.
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

    def __getitem__(self, index) -> Morpheme: ...
    def __iter__(self) -> Iterator[Morpheme]: ...
    def __len__(self) -> int: ...





class Tokenizer:
    SplitMode: ClassVar[SplitMode] = ...
    @classmethod
    def __init__(cls) -> None: ...

    def tokenize(self, text: str,
                 mode: SplitMode = ...,
                 out: Optional[MorphemeList] = None) -> MorphemeList:
        """
        Break text into morphemes.

        SudachiPy 0.5.* had logger parameter, it is accepted, but ignored.

        :param text: text to analyze
        :param mode: analysis mode.
            This parameter is deprecated.
            Pass the analysis mode at the Tokenizer creation time and create different tokenizers for different modes.
            If you need multi-level splitting, prefer using :py:meth:`Morpheme.split` method instead.
        :param out: tokenization results will be written into this MorphemeList, a new one will be created instead.
            See https://worksapplications.github.io/sudachi.rs/python/topics/out_param.html for details.
        """
        ...


class WordInfo:
    a_unit_split: ClassVar[List[int]] = ...
    b_unit_split: ClassVar[List[int]] = ...
    dictionary_form: ClassVar[str] = ...
    dictionary_form_word_id: ClassVar[int] = ...
    head_word_length: ClassVar[int] = ...
    normalized_form: ClassVar[str] = ...
    pos_id: ClassVar[int] = ...
    reading_form: ClassVar[str] = ...
    surface: ClassVar[str] = ...
    synonym_group_ids: ClassVar[List[int]] = ...
    word_structure: ClassVar[List[int]] = ...
    @classmethod
    def __init__(self) -> None: ...
    def length(self) -> int: ...

class PosMatcher:
    def __iter__(self) -> Iterator[POS]: ...
    def __len__(self) -> int: ...
    def __call__(self, m: Morpheme) -> bool:
        """
        Checks whether a morpheme has matching POS
        :param m: morpheme
        :return: if morpheme has matching POS
        """
        ...

    def __or__(self, other: PosMatcher) -> PosMatcher:
        """
        Returns a POS matcher which matches a POS if any of two matchers would match it
        :return: PosMatcher
        """
        ...

    def __and__(self, other: PosMatcher) -> PosMatcher:
        """
        Returns a POS matcher which matches a POS if both matchers would match it at the same time
        :return: PosMatcher
        """
        ...

    def __sub__(self, other: PosMatcher) -> PosMatcher:
        """
        Returns a POS matcher which matches a POS if self would match the POS and other would not match the POS
        :return: PosMatcher
        """
        ...

    def __invert__(self) -> PosMatcher:
        """
        Returns a POS matcher which matches all POS tags except ones defined in the current POS matcher
        :return: PosMatcher
        """
        ...