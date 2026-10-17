---
date: "2024-02-01"
tags: ["programming", "rust", "compilers"]
url: "https://www.stephendiehl.com/posts/egraphs/"
---

# E-Graphs in Rust

E-Graphs (aka *equality graphs*) are one of the most exciting and rapidly evolving areas in programming language engineering. Originally developed in Gregory Nelson's PhD Thesis in 1980, they have become a foundational data structure that maintains equivalence relations over expressions. While they were initially created to power SMT solvers, their applications have expanded dramatically in recent years.

This renaissance in e-graph applications is driven by techniques like equality saturation, which allows for more efficient exploration of equivalent expressions. The key innovation of E-Graphs is their ability to solve the so-called "choice problem" in term rewriting. Traditional term rewriting is destructive - once you transform an expression, the original is lost. This forces you to commit to specific optimization choices that might be locally optimal but globally suboptimal. For example, while `x + x` can be optimized to `x * 2`, applying this to `(x + x) - x` would prevent discovering that one of the additions and the subtraction cancel out to just `x`.

E-Graphs solve this by maintaining all possible equivalent expressions simultaneously in a compact form. They consist of:

- **E-Classes**: Sets of equivalent expressions
- **E-Nodes**: Operators with children that point to e-classes (not directly to other e-nodes)
- **E-Class IDs**: Unique identifiers for e-classes

The power of E-Graphs comes from two key properties:

1.  **Congruence**: If `x` is equivalent to `y`, then `f(x)` must be equivalent to `f(y)`. This property is maintained automatically as equivalences are discovered.

2.  **Compact Representation**: E-Graphs can represent exponentially many equivalent expressions in linear space by sharing structure between similar expressions.

When using E-Graphs for optimization through a technique called *equality saturation*, the process works by:

1.  Converting the initial expression into an E-Graph
2.  Repeatedly applying all possible rewrites to discover equivalences
3.  Extracting the optimal expression according to some cost function

To get started with E-Graphs in Rust, we will use the `egg` library, which provides a powerful and ergonomic API for working with E-Graphs. First, add the `egg` crate to your `Cargo.toml`:

``` toml
[dependencies]
egg = "0.6"
```

## Congruence in E-Graphs

A fundamental property of E-Graphs is that they maintain a congruence relation, not just an equivalence relation. Congruence means that if two expressions `x` and `y` are equivalent, then any larger expression containing `x` must be equivalent to the same expression with `x` replaced by `y`. More formally, if `x ≡ y`, then `f(x) ≡ f(y)` for any context `f`.

Let's see this in action with a simple example:

``` rust
use egg::*;

define_language! {
    enum SimpleLanguage {
        Num(i32),
        "+" = Add([Id; 2]),
        Symbol(Symbol),
    }
}

fn congruence_example() {
    let mut egraph = EGraph::<SimpleLanguage, ()>::default();

    // Create expressions: (+ a x) and (+ a y)
    let a = egraph.add(SimpleLanguage::Symbol("a".into()));
    let x = egraph.add(SimpleLanguage::Symbol("x".into()));
    let y = egraph.add(SimpleLanguage::Symbol("y".into()));

    let expr1 = egraph.add(SimpleLanguage::Add([a, x]));  // (+ a x)
    let expr2 = egraph.add(SimpleLanguage::Add([a, y]));  // (+ a y)

    // Initially, these are in different e-classes
    assert_ne!(egraph.find(expr1), egraph.find(expr2));

    // When we declare x ≡ y...
    egraph.union(x, y);
    egraph.rebuild();

    // ...congruence ensures (+ a x) ≡ (+ a y)
    assert_eq!(egraph.find(expr1), egraph.find(expr2));
}
```

In this example:

1.  We start with two expressions `(+ a x)` and `(+ a y)` in different e-classes
2.  When we declare `x` and `y` equivalent using `union`
3.  The e-graph automatically merges the e-classes containing `(+ a x)` and `(+ a y)`

This congruence property is maintained automatically by the e-graph as equivalences are discovered. When the e-graph performs a `rebuild` operation, it:

1.  Finds all e-nodes that have become equivalent due to congruence
2.  Merges their containing e-classes
3.  Repeats until no more merges are possible

This automatic maintenance of congruence is what makes e-graphs so powerful for term rewriting and program optimization. It ensures that all equivalent expressions are properly identified, even when the equivalence is indirect through substitution in larger expressions.

## Understanding the Core Concepts

Before diving into code, let's understand the key abstractions in `egg`:

1.  **Language Definition**: You must define your expression language using `egg`'s `define_language!` macro. Each variant represents an operator or leaf node in your expressions.

2.  **Pattern Matching**: `egg` uses a simple pattern language with variables like `?x` to match subexpressions. These patterns power the rewrite rules.

3.  **Rewrite Rules**: Rules are bidirectional transformations between equivalent expressions, defined using the `rewrite!` macro.

4.  **E-Class Analysis**: Custom analyses can be attached to e-classes to maintain additional information about the equivalence classes.

Let's see these concepts in action by defining a simple expression language:

``` rust
use egg::*;

// Define our expression language
define_language! {
    enum SimpleLanguage {
        Num(i32),
        "+" = Add([Id; 2]),
        "*" = Mul([Id; 2]),
        Symbol(Symbol),
    }
}

// Define our rewrite rules
fn make_rules() -> Vec<Rewrite<SimpleLanguage, ()>> {
    vec![
        rewrite!("commute-add"; "(+ ?a ?b)" => "(+ ?b ?a)"),
        rewrite!("commute-mul"; "(* ?a ?b)" => "(* ?b ?a)"),
        rewrite!("add-0"; "(+ ?a 0)" => "?a"),
        rewrite!("mul-0"; "(* ?a 0)" => "0"),
        rewrite!("mul-1"; "(* ?a 1)" => "?a"),
    ]
}
```

Now let's build a simple function that uses these rules to simplify expressions:

``` rust
/// Simplify an expression using egg
fn simplify(s: &str) -> String {
    // Parse the expression
    let expr: RecExpr<SimpleLanguage> = s.parse().unwrap();

    // Create a Runner and apply our rules
    let runner = Runner::default()
        .with_expr(&expr)
        .run(&make_rules());

    // Extract the smallest equivalent expression
    let extractor = Extractor::new(&runner.egraph, AstSize);
    let (best_cost, best) = extractor.find_best(runner.roots[0]);

    println!("Simplified {} to {} with cost {}", expr, best, best_cost);
    best.to_string()
}

#[test]
fn simple_tests() {
    assert_eq!(simplify("(* 0 42)"), "0");
    assert_eq!(simplify("(+ 0 (* 1 foo))"), "foo");
}
```

## Core Functions

There are four core functions that you will use most often when working with E-Graphs:

``` rust
pub fn union(&mut self, id1: Id, id2: Id) -> bool
```

The `union` function merges the e-classes containing `id1` and `id2`. It returns `true` if the e-classes were merged, and `false` if they were already in the same e-class.

``` rust
pub fn find(&self, id: Id) -> Id
```

The `find` function returns the representative of the e-class containing `id`. The representative is the smallest expression in the e-class.

``` rust
pub fn rebuild(&mut self) -> usize
```

The `rebuild` function performs a single round of congruence closure. It finds all e-nodes that have become equivalent due to congruence and merges their containing e-classes. It returns the number of e-classes that were merged.

## Advanced Features

The `egg` library also provides several advanced features:

1.  **Custom Analysis**: You can define custom analyses that maintain additional information about e-classes:

``` rust
#[derive(Default)]
struct ConstantFolding;

impl Analysis<Math> for ConstantFolding {
    type Data = Option<i32>;

    fn merge(&mut self, a: &mut Self::Data, b: Self::Data) -> DidMerge {
        if a == &b {
            DidMerge(false, false)
        } else {
            *a = None;
            DidMerge(true, false)
        }
    }

    fn make(egraph: &EGraph<Math, Self>, enode: &Math) -> Self::Data {
        match enode {
            Math::Num(n) => Some(*n),
            Math::Add([a, b]) => {
                let a = egraph[*a].data;
                let b = egraph[*b].data;
                a.zip(b).map(|(a, b)| a + b)
            }
            // ... other operations ...
            _ => None,
        }
    }
}
```

2.  **Conditional Rewrites**: Rules can include conditions that must be satisfied:

``` rust
rw!("div-cancel"; "(/ ?a ?a)" => "1" if is_not_zero("?a"))
```

## Using the Runner API

Let's look at a complete example of using the Runner API to perform equality saturation:

``` rust
use egg::{*, rewrite as rw};

// Define our rewrite rules
let rules: &[Rewrite<SymbolLang, ()>] = &[
    rw!("commute-add"; "(+ ?x ?y)" => "(+ ?y ?x)"),
    rw!("commute-mul"; "(* ?x ?y)" => "(* ?y ?x)"),
    rw!("add-0"; "(+ ?x 0)" => "?x"),
    rw!("mul-0"; "(* ?x 0)" => "0"),
    rw!("mul-1"; "(* ?x 1)" => "?x"),
];

// Parse the initial expression
let start = "(+ 0 (* 1 a))".parse().unwrap();

// Run equality saturation
let runner = Runner::default().with_expr(&start).run(rules);

// Extract the best expression using AstSize cost function
let extractor = Extractor::new(&runner.egraph, AstSize);
let (best_cost, best_expr) = extractor.find_best(runner.roots[0]);

// The expression simplifies to just "a" with cost 1
assert_eq!(best_expr, "a".parse().unwrap());
assert_eq!(best_cost, 1);
```

Note that while we're working with what appears to be numbers, `SymbolLang` actually stores everything as strings. This is just for demonstration purposes.

## Extraction and Cost Functions

After running the E-Graph to saturation, we need to extract the "best" expression from potentially many equivalent ones. The `egg` library provides an `Extractor` that can find the optimal expression according to a cost function.

Here's a complete example showing extraction in action:

``` rust
use egg::*;

define_language! {
    enum SimpleLanguage {
        Num(i32),
        "+" = Add([Id; 2]),
        "*" = Mul([Id; 2]),
    }
}

fn extraction_example() {
    let rules: &[Rewrite<SimpleLanguage, ()>] = &[
        rewrite!("commute-add"; "(+ ?a ?b)" => "(+ ?b ?a)"),
        rewrite!("commute-mul"; "(* ?a ?b)" => "(* ?b ?a)"),
        rewrite!("add-0"; "(+ ?a 0)" => "?a"),
        rewrite!("mul-0"; "(* ?a 0)" => "0"),
        rewrite!("mul-1"; "(* ?a 1)" => "?a"),
    ];

    // Start with (+ 0 (* 1 10))
    let start = "(+ 0 (* 1 10))".parse().unwrap();
    let runner = Runner::default().with_expr(&start).run(rules);
    let (egraph, root) = (runner.egraph, runner.roots[0]);

    // Extract the smallest equivalent expression
    let mut extractor = Extractor::new(&egraph, AstSize);
    let (best_cost, best) = extractor.find_best(root);

    // The expression simplifies to just "10" with cost 1
    assert_eq!(best_cost, 1);
    assert_eq!(best, "10".parse().unwrap());
}
```

The `Extractor` uses a cost function to determine which expression to choose. In this example, we use `AstSize` which simply counts the number of nodes in the expression. The expression `(+ 0 (* 1 10))` simplifies to just `10`, which has a cost of 1 (a single number node).

You can also define custom cost functions by implementing the `CostFunction` trait:

``` rust
#[derive(Clone)]
struct CustomCost;

impl CostFunction<SimpleLanguage> for CustomCost {
    type Cost = usize;

    fn cost<C>(&mut self, enode: &SimpleLanguage, mut costs: C) -> Self::Cost
    where C: FnMut(Id) -> Self::Cost
    {
        match enode {
            SimpleLanguage::Num(_) => 1,
            SimpleLanguage::Add([a, b]) => costs(*a) + costs(*b) + 2, // penalize additions more
            SimpleLanguage::Mul([a, b]) => costs(*a) + costs(*b) + 1,
        }
    }
}
```

This custom cost function penalizes additions more heavily than multiplications, which could be useful if multiplications are cheaper in your target platform.

## Understanding Explanations

One powerful feature of `egg` is its ability to explain why two terms are equivalent in the e-graph. This is particularly useful when debugging rewrite rules or validating transformations. The explanations API can show the exact sequence of rewrites that transform one expression into another.

To enable explanations, use the `with_explanations_enabled()` method when creating a runner:

``` rust
use egg::*;

fn explanation_example() {
    let rules: &[Rewrite<SymbolLang, ()>] = &[
        rewrite!("commute-add"; "(+ ?a ?b)" => "(+ ?b ?a)"),
        rewrite!("add-0"; "(+ ?a 0)" => "?a"),
    ];

    let start = "(+ (+ a 0) b)".parse().unwrap();
    let end = "(+ b a)".parse().unwrap();

    let mut runner = Runner::default()
        .with_explanations_enabled()
        .with_expr(&start)
        .run(rules);

    // Get explanation of how start transforms to end
    println!("{}", runner.explain_equivalence(&start, &end).get_flat_string());
}
```

The explanation output shows each step of the transformation with annotations indicating which rewrite rules were applied:

```
(+ (+ a 0) b)
(+ (Rewrite=> add-0 a) b)
(Rewrite=> commute-add (+ b a))
```

Explanations come in two forms:

1.  **FlatExplanation**: A linear sequence of rewrites (shown above), which is most human-readable
2.  **TreeExplanation**: A more compact representation that can share common sub-explanations

The tree format is particularly useful when dealing with large expressions that have many shared subterms. Here's an example of a tree explanation:

``` rust
let tree_explanation = runner.explain_equivalence(&start, &end);
println!("{}", tree_explanation.get_tree_string());
```

You can also use `explain_existence()` to understand why a particular term exists in the e-graph:

``` rust
// Find out how a specific term came to exist
let term = "(+ a b)".parse().unwrap();
if let Some(explanation) = runner.explain_existence(&term) {
    println!("Term exists because: {}", explanation.get_flat_string());
}
```

This is particularly valuable when debugging unexpected equivalences or validating that optimizations are working as intended.
