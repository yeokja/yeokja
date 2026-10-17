---
date: "2025-08-15"
tags: ["compilers", "type theory", "formal methods"]
url: "https://www.stephendiehl.com/posts/typechecker_zoo/"
---

# Typechecker Zoo

As a fun side project, I've built four literate Rust implementations spanning 50 years of type theory research, from Milner's Algorithm W to modern dependent type systems. I've tried to make each implementation as readable and understandable as possible, with detailed explanations and comments throughout the code and to make it as succinct and short as possible so you can pull it into your own projects as a place to start when building any type of system somewhere on the lambda cube.

- [Github Repo](https://github.com/sdiehl/typechecker-zoo) \| [Documentation](https://sdiehl.github.io/typechecker-zoo/)

-Since it's a zoo, I've also given each one a cute animal mascot to represent them:

<table>
<colgroup>
<col style="width: 50%" />
<col style="width: 50%" />
</colgroup>
<tbody>
<tr>
<td><a href="https://sdiehl.github.io/typechecker-zoo/algorithm-w/lambda-calculus.html"><img src="https://www.stephendiehl.com/images/lambda.png" style="max-width:256px; height:auto;" alt="Algorithm W" /></a></td>
<td><strong>Algorithm W (Hindley-Milner Type System)</strong><br />
<br />
Implements Milner's 1978 unification-based type inference using structural recursion. The algorithm generates constraints between type variables during expression traversal, then solves these constraints through Robinson's unification algorithm. Features let-polymorphism enabling rank-1 polymorphic type inference without explicit type annotations. The implementation demonstrates classical occurs check handling and substitution propagation through type environments.<br />
<br />
<strong>Reference:</strong><br />
<em>A Theory of Type Polymorphism in Programming</em> by Robin Milner.<br />
<a href="https://www.research.ed.ac.uk/en/publications/a-theory-of-type-polymorphism-in-programming">PDF</a></td>
</tr>
<tr>
<td><a href="https://sdiehl.github.io/typechecker-zoo/system-f/system-f.html"><img src="https://www.stephendiehl.com/images/ocaml.png" style="max-width:256px; height:auto;" alt="OCaml" /></a></td>
<td><strong>System F (Second-Order Lambda Calculus)</strong><br />
<br />
Second-order lambda calculus with parametric polymorphism using bidirectional type checking. Bidirectional type checker implementing Dunfield-Krishnaswami algorithm for System F with universal quantification. Splits type checking into synthesis (⇒) and checking (⇐) modes, enabling polymorphic type inference with existential variable solving. Handles parametric polymorphism through universal types (∀α. τ), with careful context management for type variable scoping. Includes comprehensive constraint solving for existential variables and subtyping relations between polymorphic types.<br />
<br />
<strong>Reference:</strong><br />
<em>Complete and Easy Bidirectional Typechecking for Higher-rank Polymorphism</em> by Dunfield and Krishnaswami.<br />
<a href="https://arxiv.org/abs/1306.6032">PDF</a></td>
</tr>
<tr>
<td><a href="https://sdiehl.github.io/typechecker-zoo/system-f-omega/system-f-omega.html"><img src="https://www.stephendiehl.com/images/haskell.png" style="max-width:256px; height:auto;" alt="Haskell" /></a></td>
<td><strong>System F-ω (Higher-Kinded Polymorphism)</strong><br />
<br />
A miniature Haskell-lite. Extends System F with type constructors and kind checking, implementing higher-rank polymorphism with algebraic data types. Features kind inference alongside type inference, managing both type-level and kind-level environments. Supports type constructor application and polymorphic kind assignments. Implements constraint generation for higher-kinded unification problems, demonstrating how kind systems prevent malformed type applications.<br />
<br />
Complete implementation of System F-ω with higher-kinded types, DK bidirectional type checking, existential type variables, polymorphic constructor applications, pattern matching, and lambda expressions with type inference.<br />
<br />
<strong>Reference:</strong><br />
<em>A Mechanical Formalization of Higher-Ranked Polymorphic Type Inference</em> by Joshua Dunfield.<br />
<a href="https://i.cs.hku.hk/~bruno/papers/icfp2019.pdf">PDF</a></td>
</tr>
<tr>
<td><a href="https://sdiehl.github.io/typechecker-zoo/coc/calculus-of-constructions.html"><img src="https://www.stephendiehl.com/images/lean.png" style="max-width:256px; height:auto;" alt="Lean" /></a></td>
<td><strong>Calculus of Constructions (Dependent Types)</strong><br />
<br />
The Calculus of Constructions with a hierarchy of non-cumulative universes and inductive types. Full dependent type system unifying types and terms, implementing universe polymorphism with constraint-based inference. Has inductive type definitions, dependent function types (Pi-types, Sigma-types), and universe polymorphism. Uses meta-variable solving for partial solving of higher-order unification problems using Miller pattern technique.<br />
<br />
<strong>Reference:</strong><br />
<em>A universe polymorphic type system</em> by Vladimir Voevodsky.<br />
<a href="https://ncatlab.org/ufias2012/files/Universe+polymorphic+type+sytem.pdf">PDF</a></td>
</tr>
</tbody>
</table>

------------------------------------------------------------------------

The examples are implemented in fairly idiomatic Rust with a full parse and test suite (though no evaluator), using the usual compiler libraries such as larlpop, logos, ariande, etc. They are obviously simplified and code golfed versions of the full implementations so that they can be easily understood and modified. But they should be a LOT easier to understand that trying to read the Haskell or Coq typecheckers which are easily 100k+ LOC each. Hopefully useful to any young, budding language enthusiasts out there. I wish I had had this when I was learning these topics a decade ago so I figured I'd build the resources I would want so that others don't have to struggle so much!
