sudachipy package
=================

config.Config
---------------------

.. autoclass:: sudachipy.config.Config
   :members:

Dictionary
----------------------

* Dictionary does not provide an access to the grammar and lexicon.

.. autoclass:: sudachipy.Dictionary
   :members:


TextNormalizer
----------------------

``TextNormalizer`` applies input-text plugins to raw input text.
It does not perform morphological analysis or return morpheme normalized forms.
Without a dictionary, it uses the default input-text normalization.

.. autoclass:: sudachipy.TextNormalizer
   :members:


SplitMode
----------------------

.. autoclass:: sudachipy.SplitMode


Tokenizer
----------------------

.. autoclass:: sudachipy.Tokenizer
   :members:
   :undoc-members:


Morpheme
----------------------

* Class method ``MorphemeList.empty() -> MorphemeList`` is deprecated.
   * Use ``Tokenizer.tokenize("")`` if you need.

.. autoclass:: sudachipy.MorphemeList
   :members:


.. autoclass:: sudachipy.Morpheme
   :members:


WordInfo
----------------------

.. autoclass:: sudachipy.WordInfo
   :members:
   :undoc-members:
