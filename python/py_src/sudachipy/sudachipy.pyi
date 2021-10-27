from typing import ClassVar,  Iterator, List
import sudachipy


class Dictionary:
    """
    A sudachi dictionary.
    """

    @classmethod
    def __init__(self, config_path: str = ..., resource_dir: str = ..., dict_type: str = None) -> None:
        """
        Creates a sudachi dictionary.

        If both config.systemDict and dict_type are not given, `sudachidict_core` is used.
        If both config.systemDict and dict_type are given, dict_type is used.
        """
        ...

    def close(self) -> None:
        """
        Close this dictionary.
        """
        ...

    def create(self, mode: SplitMode = SplitMode.C) -> Tokenizer:
        """
        Creates a sudachi tokenizer.

        Provide mode to set tokenizer's default split mode (C by default).
        """
        ...


class Morpheme:
    """
    A morpheme (basic semantic unit of language).
    """
    @classmethod
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

    def part_of_speech(self) -> List[str]:
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

    def split(self, mode: SplitMode) -> MorphemeList:
        """
        Returns a list of morphemes splitting itself with given split mode.
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


class MorphemeList:
    """
    A list of morphemes.
    """
    @classmethod
    def __init__(self) -> None: ...

    @classmethod
    def empty(dict) -> MorphemeList:
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
    def __init__(self) -> None: ...


class Tokenizer:
    SplitMode: ClassVar[sudachipy.SplitMode] = ...
    @classmethod
    def __init__(self) -> None: ...

    def tokenize(self, text: str, mode: sudachipy.SplitMode = ...) -> MorphemeList:
        """
        Break text into morphemes.

        By default tokenizer's split mode is used.
        The logger provided is ignored.
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
