# Borrowing and Workspaces

Calling `borrow()` on a `RefCell` checks at run time that no mutable borrow is
active. A borrow taken this way ends when the returned guard is dropped, not at
the end of the enclosing block.

The `resolver` field in the root manifest selects which resolver version the
workspace uses. Setting `resolver = "2"` changes how feature flags are unified
across the dependency graph.

Every member listed under `[workspace] members` shares one lockfile and one
output directory. The `workspace` key itself is ignored inside a member manifest.

The `cache` subcommand prints the location of the cache, while `cache --clear`
removes every entry that no lockfile in the workspace still references.

A build script is any file named `build.rs` at the package root. Its output is
cached like any other dependency, but the `build_script` metadata key can point
to a different path.

Press <kbd>Ctrl</kbd>+<kbd>C</kbd> once to cancel the *current* job, or twice to
stop the **whole** build; the partial output under `target/` stays valid, as
explained in [interrupting builds](https://example.org/build/interrupt).

The flag `--jobs=<N>` (short form `-j`) limits parallelism to *N* processes. The
default is the number of logical CPUs reported by `available_parallelism()`, but
`JOBS=1` in the environment overrides it<sup>1</sup>.

Use **`--offline`** together with *`--frozen`* when you need a build that
neither touches the network nor rewrites `Cargo.lock`; see the
[`frozen` reference](https://example.org/build/frozen) for the exact checks.
