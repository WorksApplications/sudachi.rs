sudachi package
===============


Dictionary
----------------------

* Dictionary does not provide an access to the grammar and lexicon.

.. autoclass:: sudachi.Dictionary
   :members:


SplitMode
----------------------

.. autoclass:: sudachi.SplitMode


Tokenizer
----------------------

.. autoclass:: sudachi.Tokenizer
   :members:
   :undoc-members:


Morpheme
----------------------

* Class method ``MorphemeList.empty() -> MorphemeList`` is deprecated.
   * Use ``Tokenizer.tokenize("")`` if you need.

.. autoclass:: sudachi.MorphemeList
   :members:


* Method ``Morpheme.get_word_info(self) -> WordInfo`` is deprecated.

.. autoclass:: sudachi.Morpheme
   :members:


WordInfo
----------------------

.. autoclass:: sudachi.WordInfo
   :members:
   :undoc-members:

