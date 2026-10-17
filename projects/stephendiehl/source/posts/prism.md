---
date: "2026-06-20"
tags: ["compilers", "formal", "fp", "rust"]
url: "https://www.stephendiehl.com/posts/prism/"
---

<div class="post-subtitle">

Updated 2026-07-30.

</div>

<img src="https://www.stephendiehl.com/images/prism.png" width="150" alt="Prism" />

# Prism: An Impure Functional Language With Typed Effects

This is going to be a very nerdy post, so bear with me. Here is the shortest version of the pitch:

``` rust
fn fib(n) =
  var a := 0
  var b := 1
  var i := 0
  while i < n do
    let t = a + b
    a := b
    b := t
    i += 1
  a
```

This is a mutable loop with the type `Int -> Int`. The mutation is real, but it is local and unobservable, so the function is pure from the caller's point of view. Prism lowers `var` and `while` through its effect machinery, proves the private state cannot escape, and then compiles the loop to the in-place code you meant. It is the code you would write in Python, with the signature you would want in OCaml, and without turning the middle of the function into a small religious ceremony.

That example used to be the whole story. At v0.15 it is better understood as the small end of a much larger design. Prism is a proof-of-concept functional language built around one contract: observable behavior must be determined by checked code and explicit, pinned inputs. Effects say what a computation may ask of the world, handlers decide what those requests mean, checked intermediate representations preserve the decision, content identities name it, and one canonical observation trace judges every way of running it. The language remains a toy, now with a much wider design than the original effects demo that got slightly out of hand.

- **[Prism Source Code](https://github.com/sdiehl/prism)** - the compiler, standard library, and tooling on GitHub
- **[Prism Documentation](https://sdiehl.github.io/prism/)** - the language guide, reference, and browser playground

## Effects Are Authority

An algebraic effect is an interface for a computation's requests, and a handler is the implementation and authority supplied by its context. The row after `!` is therefore more than an honesty label: `! {Console, FileSystem}` is the exact part of the world a function may observe, while an empty row means no such authority is required. Rows compose structurally, handlers may discharge only the operations they own, and everything else tunnels outward. The continuation in a resumable clause is written after `resume`, while a `never` clause has no continuation at all, so generators, exceptions, state, failure, restarts, streams, and backtracking all share the same small mechanism. On the fast path the compiler turns effect evidence into direct calls and avoids allocating a free-monad node per operation, which keeps the abstraction from becoming a performance apology.

``` rust
effect Gen
  yield(Int) : Unit

fn produce(n) : Unit ! {Gen} =
  if n == 0 then
    ()
  else
    yield(n)
    produce(n - 1)

fn total(n) =
  handle produce(n) with
    yield(v) resume k => v + k(())
    return r => 0
```

## Determinism Is the Contract

Prism requires implementation choices to stay unobservable. The interpreter, LLVM backend, MLIR backend, optimizer levels, and forced effect-lowering tiers are compared through one ordered observation trace containing outputs, capability events, committed files, faults, and results. A different scheduler policy or faster lowering may change cost. Meaning stays fixed. Transcendental floating-point operations route through the same vendored musl routines, keeping their result bits stable across host libraries. Time, randomness, environment variables, and filesystem reads enter as capability effects, and once their results are pinned, the run becomes a function of the program and that trace. Determinism supplies the common execution that backend parity, replay, caching, and every later claim refer to.

## One Checked Semantic Spine

The compiler has one semantic pipeline shared by every backend. Surface syntax resolves into a checked HIR carrying the type, effect, binding, pattern, and source facts needed by elaboration. Elaboration transcribes those facts into a call-by-push-value, administrative-normal-form Core. Typed Core carries witnesses that an independent verifier checks after every transformation. The checker decides and later stages transcribe its facts. An optimization may simplify or lower a term, but it must produce another independently valid Core artifact before an interpreter or native emitter is allowed to see it. CBPV makes the boundary between values and effectful computations explicit, which gives effect lowering, ownership insertion, and backend emission one place to agree. The compiler can still contain bugs, obviously, but every backend receives the same checked language even on Tuesdays.

## Content Identity and Lineage

Every checked definition has a scheme-tagged identity derived from canonical Core and the exact semantic dependencies its compilation reads. Files, canonical syntax trees, binders, type shapes, module interfaces, standard-library roots, build queries, and native artifacts have related but deliberately distinct identities, because confusing source text with checked meaning is how build systems become theology. The same Merkle structure drives incremental checking and the persistent build cache: an implementation edit whose interface stays stable leaves its importers checked, and an unchanged query can reuse its previous artifact. More unusually, the compiler records lineage facts about those decisions, so `prism lineage why-recompiled` can explain which semantic input moved and retire the traditional build-system shrug. A `core-identity` dump exposes the material entering a definition hash, and v0.15 includes a Prism consumer that reproduces the compiler's result. Its scope is intensional identity of canonical checked code and dependencies, shared by the checker, cache, artifacts, and execution gates.

## Replay and Reified Execution

Once observations are explicit and execution is deterministic, a recorded capability trace can stand in for the outside world: replay supplies the same results without performing the real operations and must reproduce the same canonical observations. Prism also reifies interpreter continuations into versioned envelopes bound to code identity, so a suspended computation can be inspected, stored, and resumed only against the semantic world it names. Both features come directly from the language-level effect machine. The guarantees stay separate and honest: content identity says which checked computation this is, replayability says which observations may be supplied from a trace, portability says whether a continuation may cross the boundary, and each has its own checker or witness. Execution becomes data with stricter terms than a process image, arbitrary socket, or yesterday's deployment could ever satisfy.

## Deterministic Concurrency

Concurrency remains a standard-library handler over an `Async` effect, leaving function types with one consistent effect vocabulary. Fibers, channels, structured joins, cancellation cleanup, and scheduler policy are interpreted by the `Concurrent` modules. Effects performed inside a fiber remain in the caller's row and pass through the scheduler to their outer handlers. Cooperative scheduling makes each yield point explicit, FIFO and LIFO policies are swappable handlers, and a logical `Clock` can sit outside the scheduler so tests advance deterministic time without consulting the operating system. Put a replay handler around the same program and its scheduled observations are judged by the same trace as an ordinary sequential run. Hard concurrent problems remain, but the runtime, clock, and scheduler all become visible parts of the program's meaning.

## Functional Performance

The runtime story is one paragraph because the representation choices matter more than the plumbing. Prism uses Perceus-style deterministic reference counting, frame-limited cell reuse, unboxed products, flat buffers, arenas, and checked `fip` or `fbip` usage contracts to make functional code run with predictable memory behavior and no tracing collector. A uniquely owned list cell or record spine can be reused in place while the source retains ordinary value semantics. Shared values take the allocating path, and both routes must produce the same result. Effect lowering similarly chooses between direct evidence passing, specialized state, and a general reified fallback while keeping that choice unobservable. LLVM and MLIR emit native code against a small modular C runtime, while the interpreter remains the semantic oracle. The compiler names the exact conditions under which allocation, reuse, fusion, and constant-stack execution are guaranteed, then checks them.

## Modern Types and Surface Syntax

The surface is intentionally familiar: bidirectional higher-rank inference, row-polymorphic effects, named Lean-style typeclass instances with one coherent canonical choice, explicit `using` overrides, deriving, dot chains, layout blocks, comprehensions, lenses, failure operators, and imperative sugar over the same small Core. v0.15 tightened the notation around the actual model: result types precede rows as `: Int ! {E}`, resumable clauses say `op(x) resume k`, and resumption grades are the bluntly useful `never`, `once`, and `many`. It also added alternation at any pattern depth plus constructor, record, and tuple patterns in parameter position, and expanded `deriving` with `ToJson`, `FromJson`, and `Plate`. `Plate` is particularly useful because a compiler pass can now be written as one local analysis or rewrite rule plus a traversal strategy over derived children-and-rebuild, saving the world from visitor number forty-seven. The approachable syntax earns its keep by exposing the underlying model cleanly.

``` rust
type Pair = Pair(Int, Int)
type Shape = Dot | Line(Int, Int) | Ring(Int)

fn sum_pair(Pair(a, b)) = a + b

fn extent(s) =
  match s of
    Line(0 | 1, _) => 1
    Line(n, _) | Ring(n) => n
    Dot => 0
```

## Coeffects

Effects describe what a computation may do to its world. Coeffects describe what the surrounding world may do with a value. Prism writes the first after `!` and the second after `@`, so `!` reports outward while `@` places a requirement inward at the boundary that consumes the value. Four usage facts are checked in v0.15. `noalloc` certifies that a function's whole call tree allocates no fresh heap cell. `once` lets a closure be consumed at most once. `portable` restricts a closure to content-addressed code and data safe to move to a fresh runtime. `noescape` prevents a value from being returned, stored, or captured past its boundary. Other words in the reserved usage vocabulary are rejected until they have real checkers. These explicit contracts live at API boundaries and erase before Core, restricting which programs compile while leaving accepted programs with the same runtime representation. `borrow`, `fip`, `fbip`, allocation certificates, and continuation grades are all different views of the same general question: what may the context do with this thing?

## The Three Posets

A Prism signature carries three partially ordered systems. Effect rows are ordered by set inclusion: sequencing takes their union, the join, while handling removes a label back toward the pure empty row, so doing more is always expressible. Coeffect rows are products of smaller usage axes with silence at the top. Moving downward strengthens a promise that somebody must prove, moving upward merely forgets one, and some exclusive claims such as `once` and `many` have no common meet because they genuinely contradict. Operation grades form the simple total lattice `never < once < many`: a handler clause may discard a continuation, resume it exactly once in tail position, or capture and resume it freely, but its own behavior must be no greater than the operation's declared grade. This is the type system's slightly alarming unification. Effects always need joins, coeffects sometimes refuse meets, and continuation use is a quantity. An effect operation then turns out to be a coeffect on its own continuation, which is deadpan category-theory nonsense right up until the compiler checks it.

## Verification With Receipts

Prism keeps several kinds of evidence beside the same checked subject. `logic fn`, `requires`, and `ensures` produce SMT obligations and explicit solver receipts. `total fn` checks termination where the implemented method can establish it and reports an honest pending result where it cannot. `test fn` gives deterministic discovery and isolated execution. Allocation and mobility annotations have their own contracts. A subset of Core is separately modeled in Lean 4 with a deterministic small-step relation, a total CEK step function, and a theorem connecting the machine to its big-step semantics, then exercised against the Rust interpreter on a fixture corpus. The theorem's proved scope is the Lean model. The Rust typechecker, optimizer, and whole compiler remain outside it. Every guarantee names the witness that supports it, and differential parity remains the gate where mechanization currently ends.

## Prism in Prism

The long game is a self-hosting compiler built one verified representation boundary at a time. The Rust compiler is stage 0. It compiles the Prism-in-Prism source into stage 1, then stage 1 compiles the same source into stage 2. The bootstrap reaches its fixed point when stages 1 and 2 produce byte-identical artifacts. Each compiler phase begins as an ordinary Prism library beside its Rust oracle, exchanges versioned artifacts with the rest of the pipeline, and earns authority after the differential gates close. v0.15 occupies the first seam with the Prism lexer, interpolation scanner, and layout pass, giving the project its first successful bootstrap layer while Rust continues to supply stage 0. The same approach can carry syntax, checking, elaboration, and code generation across in turn, with content identities pinning every input and independent verifiers checking every output. The destination is an open compiler whose source, intermediate forms, tools, and reproduction proof all live under the same deterministic contract as the programs it compiles. Rebuilding the floor while standing on it is hard enough. Requiring every floorboard to be content-addressed and independently checked is ridiculous, and would be extremely cool.

## Runs In Browser Through WebAssembly

The browser build packages the front end and interpreter as WebAssembly, so the playground can check and run Prism without an installation and expose inferred signatures, checked HIR, Core, observations, and simulation state. It runs the same interpreter and checked standard library used by the rest of the tooling, giving the browser the same semantic vocabulary. Native Prism programs still go through LLVM or MLIR. Direct Prism-to-WebAssembly code generation remains future work. The playground provides a convenient window into the semantic pipeline and a good home for deterministic simulations whose state can be reconstructed at step N from pinned inputs.

- [Prism documentation](https://sdiehl.github.io/prism/)
- [Prism playground](https://sdiehl.github.io/prism/play/)

## Nerd Stuff

For completeness, if you have read this far you have clearly made some very questionable life choices (hi fellow traveller!) so here's the PL nerd stuff:

- Type inference is the usual SOTA bidirectional and higher-rank, the complete-and-easy Dunfield-Krishnaswami algorithm, so rank-N polymorphism works without you annotating your way out of it.
- Typeclasses with Lean-style named instances (`instance ordInt : Ord(Int)`), a `canonical` designation that keeps overlapping instances coherent, explicit override (`sort_by_ord(xs, using ordRev)`), and `deriving (Eq, Ord, Show)`.
- A mostly OCaml-ish-shaped surface syntax : layout blocks, dot chains (`xs.over(f).keep(g).sum()`), `with` sugar for continuation-passing code, string interpolation, effect row aliases.
- Deep recursion runs in constant stack, both natively (tail calls, and tail recursion modulo a constructor) and in the interpreter (it is essentially a more advanced version of the old CEK machine, so nothing in your program can blow the host stack).

And, for the genuinely afflicted, the full intellectual lineage. Obviously standing on the work of many FP giants, but the ones that are most directly inspired Prism's design are:

- The core IR is call-by-push-value (Levy, [*Call-by-Push-Value: A Functional/Imperative Synthesis*](https://scholar.google.com/scholar?q=Call-by-Push-Value+A+Functional+Imperative+Synthesis), 2001), whose split between values and computations is the thing that makes both the effect lowering and the reference counting analysis clean rather than heroic.
- The fast effect path is evidence passing (Xie and Leijen, [*Generalized Evidence Passing for Effect Handlers*](https://scholar.google.com/scholar?q=Generalized+Evidence+Passing+for+Effect+Handlers), ICFP 2021, and [*Effect Handlers, Evidently*](https://scholar.google.com/scholar?q=Effect+Handlers+Evidently), ICFP 2020), the same compilation strategy Koka uses, with the free-monad encoding (Swierstra's [*Data Types a la Carte*](https://scholar.google.com/scholar?q=Data+Types+a+la+Carte), and Kiselyov's extensible effects) kept only as the fallback.
- The memory model is Perceus (Reinking, Xie, de Moura, Leijen, [*Perceus: Garbage Free Reference Counting with Reuse*](https://scholar.google.com/scholar?q=Perceus+Garbage+Free+Reference+Counting+with+Reuse), PLDI 2021) plus frame-limited reuse (Lorenzen and Leijen, [*Reference Counting with Frame-Limited Reuse*](https://scholar.google.com/scholar?q=Reference+Counting+with+Frame-Limited+Reuse), ICFP 2021) and fully-in-place programming (Lorenzen, Leijen, Swierstra, [*FP\^2: Fully in-Place Functional Programming*](https://scholar.google.com/scholar?q=FP2+Fully+in-Place+Functional+Programming), ICFP 2023), which is also what turns the lens sugar into pointer writes.
- The effect rows are row polymorphism with scoped labels (Wand 1987, Leijen, [*Extensible Records with Scoped Labels*](https://scholar.google.com/scholar?q=Extensible+Records+with+Scoped+Labels), 2005, and [*Type Directed Compilation of Row-Typed Algebraic Effects*](https://scholar.google.com/scholar?q=Type+Directed+Compilation+of+Row-Typed+Algebraic+Effects), POPL 2017), and handlers themselves are Plotkin and Pretnar ([*Handlers of Algebraic Effects*](https://scholar.google.com/scholar?q=Handlers+of+Algebraic+Effects), ESOP 2009) by way of Eff and Koka.
- Pattern matching compiles to decision trees with a usefulness matrix for exhaustiveness (Maranget, [*Compiling Pattern Matching to Good Decision Trees*](https://scholar.google.com/scholar?q=Compiling+Pattern+Matching+to+Good+Decision+Trees), 2008, and [*Warnings for Pattern Matching*](https://scholar.google.com/scholar?q=Warnings+for+Pattern+Matching), 2007), and the `pattern` forms are view patterns crossed with GHC's [*Pattern Synonyms*](https://scholar.google.com/scholar?q=Pattern+Synonyms+Haskell) (2016).
- The failure layer is the Verse calculus (Augustsson, Peyton Jones, Steele, Sweeney, et al., [*The Verse Calculus*](https://scholar.google.com/scholar?q=The+Verse+Calculus+A+Core+Calculus+for+Deterministic+Functional+Logic+Programming), ICFP 2023), recovered entirely from `never` handlers with no new core.
- A subset of the core is mirrored in a Lean 4 model (`models/Prism.lean`) with a machine-checked determinism theorem, and the three backends are held byte-identical by treating the interpreter as a differential oracle.

The thesis, if a toy project gets to have one, is that "purely functional" was always a slightly defensive name for a good idea. The good idea is that effects should be visible, typed, and composable. You do not need to forbid them to get that. You need to track them. And once you are tracking them honestly, the compiler has enough information to make them free, which is the part the purity narrative never promised you, because it was too busy not making eye contact with the IO monad.

This compiler is essentially my love letter to the old 2010-20s era of functional programming which was a much simpler and happier time (or maybe that's just the rose-tinted glasses of youth in the ZIRP era). This compiler is a toy, it is for all intents and purposes useless except maybe as kind of a piece of art from a bygone time before the Transformer-era of programming. The world we're headed to doesn't really have a place for this kind of thing anymore. We're increasingly headed to a world in which maximally probabilistic hilariously-uninteresting Typescript increasingly runs most of the world as we babysit these token extruders while the economy and markets become increasingly automated by agents. While this makes me kind of sad, it doesn't mean we still can't build this kind of thing [just for fun](https://www.goodreads.com/quotes/10260174-they-are-the-motivational-factors-for-everything-in-your-life-for) and that's what I did here. Maybe the future of functional programming languages is increasingly niche-but-economically-irrelevant and becomes more like a hobby pursuit for the few of us who still love it for the intellectual beauty of the underlying ideas. So I built it anyways, and it runs, and it was a blast to build. And just maybe that's enough, in this brave new world of software.
