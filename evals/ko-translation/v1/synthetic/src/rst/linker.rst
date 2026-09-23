.. _linker-overview:

Static Linking
==============

The linker combines object files into a single executable. It resolves every
undefined symbol against the definitions it has seen so far, as described in
:ref:`symbol-resolution`, and reports the first symbol it cannot resolve.

Archives are searched lazily: a member is pulled in only when it defines a symbol
that is still undefined. Pass ``--whole-archive`` to disable this behavior for a
single archive, or see `the archive format notes <https://example.org/ld/ar>`_ for
the details of the index table.

.. note::

   The order of archives on the command line matters. An archive that is listed
   before the object that needs it will not be searched again.

- Section garbage collection is enabled with ``--gc-sections`` and removes any
  section that is unreachable from the entry point.
- Identical code folding merges functions whose machine code is byte-for-byte
  equal [#icf]_, which can confuse debuggers that rely on distinct addresses.

.. _symbol-resolution:

Symbol Resolution
-----------------

A weak definition loses to any strong definition of the same name, but two strong
definitions are an error. The rules are summarized in :ref:`linker-overview` and
formalized in the :doc:`appendix <appendix>` for readers who want the full table.

.. [#icf] Folding is only safe when no code compares the addresses of two
   functions, which the linker cannot prove in general.
