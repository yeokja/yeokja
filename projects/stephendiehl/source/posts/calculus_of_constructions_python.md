---
date: "2024-11-18"
tags: ["mathematics", "logic", "fp"]
url: "https://www.stephendiehl.com/posts/calculus_of_constructions_python/"
---

# Dependent Types in 200 Lines of Python

The Calculus of Constructions (shortened as CoC) is one of the most common typed lambda calculi, forming the theoretical foundation for dependently typed programming languages like Coq. In this post, we'll implement a type checker for CoC in 200 lines of Python, breaking down the key components and explaining how they work. This is mostly a code-golf exercise, so the implementation is not super robust but it is fun that we can even implement this in Python.

Some background, the CoC was developed by Thierry Coquand in 1985, and is a higher-order typed lambda calculus that combines two key ideas:

1.  Dependent types - types that can depend on terms
2.  Type polymorphism - the ability to quantify over types

The system has two sorts: `*` (the type of types) and `☐` (the type of `*`). It supports the following terms:

- Variables
- Lambda abstractions (λx:A.t)
- Function applications (t u)
- Dependent product types (Πx:A.B)
- Type annotations (t : A)

Let's break down our implementation into its core components. We have the term representation, in which we approximate algebraic data types using Python classes.

**Understanding De Bruijn Indices**

Before diving into the implementation, let's understand De Bruijn indices, which we use for variable lookup. In De Bruijn notation, variables are represented by numbers that indicate how many binders you need to go up to find the variable's binding site. For example:

```
# Traditional notation:
λx. λy. x y

# De Bruijn notation:
λ. λ. 1 0    # 1 refers to x (up one binder), 0 refers to y (current binder)
```

More examples:

```
# Traditional:        # De Bruijn:          # Explanation:
λx. x                 λ. 0                  # x is bound by the current (0th) binder
λx. λy. y             λ. λ. 0               # y is bound by the current binder
λx. λy. λz. x y z     λ. λ. λ. 2 1 0        # x=2, y=1, z=0 binders up
Πx:*. λy. x           Π*. λ. 1              # x is one binder up from y
```

In our implementation, we use De Bruijn levels (counting from the outside) rather than indices (counting from the inside) for the context lookup. This is why we use `lvl - x - 1` to access the context, converting from levels to indices.

**Type Term Hierarchy**

``` python
@dataclass
class Term:
    pass

@dataclass
class Lam(Term):
    f: Callable[[Term], Term]

@dataclass
class Pi(Term):
    a: Term
    f: Callable[[Term], Term]

@dataclass
class Appl(Term):
    m: Term
    n: Term

@dataclass
class Ann(Term):
    m: Term
    a: Term

@dataclass
class FreeVar(Term):
    x: int

@dataclass
class Star(Term):
    pass

@dataclass
class Box(Term):
    pass
```

For term evaluation, we implement beta reduction, the core computational rule of lambda calculus. The `eval` function implements call-by-value evaluation strategy and is crucial for type checking since dependent types require comparing types for computational equality.

``` python
def eval(term: Term) -> Term:
    match term:
        case Lam(f):
            return Lam(lambda n: eval(f(n)))
        case Pi(a, f):
            return Pi(eval(a), lambda n: eval(f(n)))
        case Appl(m, n):
            m_eval = eval(m)
            n_eval = eval(n)
            match m_eval:
                case Lam(f):
                    return f(n_eval)
                case _:
                    return Appl(m_eval, n_eval)
        case Ann(m, _):
            return eval(m)
        case FreeVar() | Star() | Box():
            return term
```

The type checker is split into three mutually recursive functions. Some important implementation details to note:

- The `infer_ty` and `check_ty` functions always return evaluated types
- The typing context (`ctx`) must contain only evaluated types and be well-formed (every entry must be of some sort)
- Bindings are added to the beginning of the context and indexed by De Bruijn indices
- Since `FreeVar` uses De Bruijn levels, we use `lvl - x - 1` to access the context

``` python
def infer_ty(lvl: int, ctx: List[Term], term: Term) -> Term:
    match term:
        case Pi(a, f):
            _s1 = infer_sort(lvl, ctx, a)
            s2 = infer_sort(lvl + 1, [eval(a)] + ctx, unfurl(lvl, f))
            return s2
        case Appl(m, n):
            m_ty = infer_ty(lvl, ctx, m)
            match m_ty:
                case Pi(a, f):
                    _ = check_ty(lvl, ctx, (n, a))
                    return f(n)
                case _:
                    return panic(lvl, m, f"Want a Pi type, got {print_term(lvl, m_ty)}")
        case Ann(m, a):
            _s = infer_sort(lvl, ctx, a)
            return check_ty(lvl, ctx, (m, eval(a)))
        case FreeVar(x):
            return ctx[lvl - x - 1]
        case Star():
            return Box()
        case Box():
            return panic(lvl, Box(), "Has no type")
```

The `infer_ty` function implements bidirectional type checking:

- For Pi types, ensures both the domain and codomain are sorts
- For applications, infers the function type and checks the argument
- For annotations, verifies the type is valid and checks the term against it
- For variables, looks up their type in the context
- For `*`, returns `☐`
- `☐` has no type (it's at the top of the hierarchy)

``` python
def infer_sort(lvl: int, ctx: List[Term], a: Term) -> Term:
    ty = infer_ty(lvl, ctx, a)
    match ty:
        case Star() | Box():
            return ty
        case _:
            return panic(lvl, a, f"Want a sort, got {print_term(lvl, ty)}")
```

The `infer_sort` function ensures a term has type `*` or `☐`:

- Infers the type of the term
- Verifies it's a sort
- Returns the sort for use in type formation rules

``` python
def check_ty(lvl: int, ctx: List[Term], pair: Tuple[Term, Term]) -> Term:
    t, ty = pair
    match (t, ty):
        case (Lam(f), Pi(a, g)):
            _ = check_ty(lvl + 1, [a] + ctx, unfurl2(lvl, (f, g)))
            return Pi(a, g)
        case (Lam(f), _):
            return panic(lvl, Lam(f), f"Want a Pi type, got {print_term(lvl, ty)}")
        case _:
            got_ty = infer_ty(lvl, ctx, t)
            if equate(lvl, (ty, got_ty)):
                return ty
            return panic(lvl, t, f"Want type {print_term(lvl, ty)}, got {print_term(lvl, got_ty)}")
```

The `equate` function implements structural equality checking. When checking two terms for beta-convertibility (computational equality), we must first evaluate them before calling `equate`. In `check_ty`, when comparing the inferred type (`got_ty`) with the expected type (`ty`), both are already in normal form so no additional evaluation is needed.

``` python
def equate(lvl: int, terms: Tuple[Term, Term]) -> bool:
    def plunge(pair: Tuple[Callable[[Term], Term], Callable[[Term], Term]]) -> bool:
        return equate(lvl + 1, unfurl2(lvl, pair))

    match terms:
        case (Lam(f), Lam(g)):
            return plunge((f, g))
        case (Pi(a, f), Pi(b, g)):
            return equate(lvl, (a, b)) and plunge((f, g))
        case (Appl(m, n), Appl(m2, n2)):
            return equate(lvl, (m, m2)) and equate(lvl, (n, n2))
        case (Ann(m, a), Ann(m2, b)):
            return equate(lvl, (m, m2)) and equate(lvl, (a, b))
        case (FreeVar(x), FreeVar(y)):
            return x == y
        case (Star(), Star()) | (Box(), Box()):
            return True
        case _:
            return False
```

The `equate` function implements structural equality checking:

- Compares terms recursively, handling all term constructors
- Uses HOAS comparison for bound variables
- Implements alpha-equivalence through the level counting mechanism

``` python
def unfurl(lvl: int, f: Callable[[Term], Term]) -> Term:
    return f(FreeVar(lvl))

def unfurl2(lvl: int, pair: Tuple[Callable[[Term], Term], Callable[[Term], Term]]) -> Tuple[Term, Term]:
    f, g = pair
    return (unfurl(lvl, f), unfurl(lvl, g))
```

These utility functions handle HOAS variable manipulation:

- `unfurl` converts a HOAS function to a term with a free variable
- `unfurl2` does the same for pairs of functions
- The level parameter ensures proper scoping of variables

``` python
def print_term(lvl: int, term: Term) -> str:
    def plunge(f: Callable[[Term], Term]) -> str:
        return print_term(lvl + 1, unfurl(lvl, f))

    match term:
        case Lam(f):
            return f"(λ{plunge(f)})"
        case Pi(a, f):
            return f"(Π{print_term(lvl, a)}.{plunge(f)})"
        case Appl(m, n):
            return f"({print_term(lvl, m)} {print_term(lvl, n)})"
        case Ann(m, a):
            return f"({print_term(lvl, m)} : {print_term(lvl, a)})"
        case FreeVar(x):
            return str(x)
        case Star():
            return "*"
        case Box():
            return "☐"
```

The `print_term` function converts terms to readable string representations:

Now let's look at two "practical" examples of CoC in action: Church numerals and length-indexed vectors. To make the examples more readable, we use some helper functions:

``` python
def curry2(f):
    return Lam(lambda x: Lam(lambda y: f(x, y)))

def curry3(f):
    return Lam(lambda x: curry2(lambda y, z: f(x, y, z)))

def curry4(f):
    return Lam(lambda x: curry3(lambda y, z, w: f(x, y, z, w)))

def curry5(f):
    return Lam(lambda x: curry4(lambda y, z, w, v: f(x, y, z, w, v)))

def appl(f: Term, args: List[Term]) -> Term:
    return reduce(lambda m, n: Appl(m, n), args, f)
```

Church numerals represent natural numbers as functions. In CoC, we can give them precise types:

``` python
# The type of Church numerals
n_ty = Pi(Star(), lambda a:
          Pi(Pi(a, lambda _x: a), lambda _f:
             Pi(a, lambda _x: a)))

# Zero is the identity function
zero = Ann(curry3(lambda _a, _f, x: x), n_ty)

# Successor applies f one more time
succ = Ann(
    curry4(lambda n, a, f, x: Appl(f, appl(n, [a, f, x]))),
    Pi(n_ty, lambda _n: n_ty)
)

# Addition combines the function applications
add = Ann(
    curry5(lambda n, m, a, f, x:
          appl(n, [a, f, appl(m, [a, f, x])])),
    Pi(n_ty, lambda _n: Pi(n_ty, lambda _m: n_ty))
)
```

Here's what's happening:

- A Church numeral takes a type `a`, a function `f: a → a`, and a value `x: a`
- It applies `f` to `x` n times, where n is the number being represented
- `zero` returns x unchanged
- `succ n` applies f one more time than n does
- `add n m` applies f n+m times

We can also use dependent types to create vectors whose length is statically known:

``` python
# Vector context setup
vect_ctx = [
    # Vector type constructor: (n: Nat) → (a: Type) → Type
    Pi(n_ty, lambda _n: Pi(Star(), lambda _a: Star())),

    # Item type and value
    Star(),
    item_ty,

    # replicate: (n: Nat) → (a: Type) → a → Vec n a
    Pi(n_ty, lambda n:
       Pi(Star(), lambda a:
          Pi(a, lambda _x: vect_ty(n, a)))),

    # concat: (n m: Nat) → (a: Type) → Vec n a → Vec m a → Vec (n+m) a
    Pi(n_ty, lambda n:
       Pi(n_ty, lambda m:
          Pi(Star(), lambda a:
             Pi(vect_ty(n, a), lambda _x:
                Pi(vect_ty(m, a), lambda _y:
                   vect_ty(appl(add, [n, m]), a)))))),

    # zip: (n: Nat) → (a b: Type) → Vec n a → Vec n b → Vec n (Pair a b)
    Pi(n_ty, lambda n:
       Pi(Star(), lambda a:
          Pi(Star(), lambda b:
             Pi(vect_ty(n, a), lambda _x:
                Pi(vect_ty(n, b), lambda _y:
                   vect_ty(n, appl(pair, [a, b])))))))
]
```

The vector operations demonstrate dependent typing:

- `replicate` creates a vector of length n filled with a value
- `concat` combines vectors, with the result length being the sum
- `zip` combines two vectors of the same length

Here's how we can use these operations:

``` python
# Create vectors of different lengths
vect_one = replicate(one, item)
vect_three = replicate(three, item)
vect_four = concat(one, three, vect_one, vect_three)

# This will type check
zip(four, vect_four, vect_four)

# This will fail type checking - mismatched lengths
zip(four, vect_one, vect_four)  # TypeError!
```

The type system ensures:

- Vector lengths are tracked precisely
- Operations maintain length invariants
- Length mismatches are caught statically

These examples demonstrate how CoC can express both computation and static guarantees in a unified system. The type checker ensures our operations respect these guarantees while still allowing flexible programming patterns. Dependent types are cool.

Let's look at the implementation of these vector operations in more detail:

**Replicate**

``` python
# replicate: (n: Nat) → (a: Type) → a → Vec n a
def replicate(n: Term, x: Term) -> Term:
    return appl(FreeVar(3), [n, item_ty, x])
```

The replicate function creates a vector of length `n` where every element is `x`. The type ensures that the resulting vector's length matches the input number `n`. In a real implementation, this would construct a list of n copies of x, but in our type-only implementation we just track the types.

**Concatenate**

``` python
# concat: (n m: Nat) → (a: Type) → Vec n a → Vec m a → Vec (n+m) a
def concat(n: Term, m: Term, x: Term, y: Term) -> Term:
    return appl(FreeVar(4), [n, m, item_ty, x, y])
```

The concat function combines two vectors. Its type is particularly interesting because it shows how dependent types can track arithmetic relationships - the output vector's length is `n + m`, the sum of the input lengths. This is enforced statically by the type system.

**Zip**

``` python
# zip: (n: Nat) → (a b: Type) → Vec n a → Vec n b → Vec n (Pair a b)
def zip(n: Term, x: Term, y: Term) -> Term:
    return appl(FreeVar(5), [n, item_ty, item_ty, x, y])
```

The zip function combines two vectors element-wise into a vector of pairs. The type system enforces that both input vectors must have the same length `n`, and guarantees the output vector will also have length `n`. If you try to zip vectors of different lengths, you'll get a type error at compile time rather than a runtime error.

These operations form a small vector library with static length checking. The type system ensures we can't:

- Concatenate vectors and get the length wrong
- Zip vectors of different lengths
- Create a vector with a length that doesn't match its type

This demonstrates how dependent types can encode precise specifications directly in the type system. While our implementation is just a type checker (we don't actually construct the vectors), it shows how these ideas work in languages like Coq or Agda where you would have real implementations.
