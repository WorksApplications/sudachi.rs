WordInfo subsetting
===================

It is possible to ask Suachi to return only a subset of fields in the WordInfo.
To do that, you can use the ``fields`` parameter of the :py:meth:`sudachipy.Dictionary.create()` method.
The parameter accepts a set of strings, each one representing a field to be returned.
By default, all fields are returned.

Allowed values:

* ``surface``: in-dictionary surface word form.
    :py:meth:`sudachipy.Morpheme.surface()` method returns the slice of the input text and is not affected by that flag.
* ``pos`` or ``pos_id``: part-of-speech tag.
* ``normalized_form``
* ``dictionary_form``
* ``reading_form``
* ``word_structure``
* ``synonym_group_id``
* ``splits_a``
* ``splits_b``

.. note::
    If you want only tokenization (e.g. use only :py:meth:`sudachipy.Morpheme.surface()`,
    passing empty set is allowed.

You need to load splits if you want to use :py:meth:`sudachipy.Morpheme.split` method.
If performing the tokenization with non-default mode, the required splits will be loaded automatically and can be omitted.::

    dic.create(SplitMode.B, fields={}) # implicitly becomes fields={'splits_b'}

.. warning::
    Using a field not included in the passed subset will produce an incorrect result without any warning.
    Use tests to ensure that the needed fields are loaded when using this parameter.