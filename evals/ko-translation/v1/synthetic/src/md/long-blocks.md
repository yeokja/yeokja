# The Code Generation Pipeline

The backend receives a control-flow graph in static single-assignment form. It
first lowers each high-level operation into a sequence of target instructions
that still use an unlimited supply of virtual registers. This representation is
then scheduled, which reorders independent instructions to hide memory latency.
Register allocation follows and maps every virtual register to either a physical
register or a stack slot. It is the most expensive stage for large functions.
After allocation, a peephole pass removes redundant moves and folds short jump
chains. Finally, the emitter encodes each instruction and records relocations for
the linker.

Three kinds of values never reach the allocator. The first is a constant small
enough to be encoded directly in an instruction. The second is the frame pointer,
which is reserved for the whole function. The third is any value whose only use
is as the address of a load in the same basic block, because the selector folds
it into an addressing mode. This matters for benchmarks. It means that a loop
which looks register-heavy in the source may in fact need very few registers.

Error messages are produced in two phases. The checker collects diagnostics while
it walks the program but does not print them. This lets it discard a follow-up
error that is only a consequence of an earlier one. Once the walk finishes, the
remaining diagnostics are sorted by source position. They are then rendered with
the offending line, a caret under the relevant span, and an optional suggestion.
Suggestions are machine-applicable only when the fix is unambiguous. Otherwise
they are shown as plain text.

The package cache is a content-addressed directory. Every archive is stored under
the hash of its contents, not under its name or version. Two packages that happen
to ship identical archives therefore share a single entry. This saves space in
monorepos, where dozens of internal packages often vendor the same helper. It also
makes the cache safe to share between users. Nobody can overwrite an entry with
different bytes, because different bytes would produce a different key. Cleanup
is a separate command and never runs implicitly.

A test runner that promises deterministic output has to control more than the
order of tests. It must seed every random number generator from a single value
printed at the start of the run. It must replace the wall clock with a virtual
clock that advances only when the test asks it to. It must also sort directory
listings, since file systems do not agree on an order. Environment variables are
the last source of drift, and they are cleared except for an explicit allow list.
With all of this in place, a failing seed can be replayed on another machine.
