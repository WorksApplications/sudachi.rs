Output Parameters and Memory Reuse
==================================

By default, SudachiPy creates a new :py:class:`sudachipy.MorphemeList` for each tokenization run.
That incurs measurable memory allocation overhead.
Instead, it is possible to reuse MorphemeLists for multiple analysis runs.

The basic usage pattern is to pass a :py:class:`sudachipy.MorphemeList` as an out parameter to
:py:meth:`sudachipy.Tokenizer.tokenize()` method::

    tok = dic.create(Mode.A)
    morphemes = tok.tokenize("")
    for line in data:
        tok.tokenize(line, out=morphemes)
        process(morphemes)

New analysis data will replace old analysis data in this case, reusing the memory.

:py:meth:`sudachipy.Morpheme.split` also supports memory reuse.
In it's case, you should be careful because the resulting MorphemeList will refer to the
data of the parent MorphemeList and will be invalidated when using the parent list as an
output parameter::

    ml1 = tok.tokenize("外国人参政権")
    subl1 = ml1[0].split(SplitMode.A)
    tok.tokenize("something", out=ml1)
    subl1[0].surface() # can raise an exception!

