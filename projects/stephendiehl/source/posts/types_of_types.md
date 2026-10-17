---
date: "2024-09-04"
tags: ["formal"]
url: "https://www.stephendiehl.com/posts/types_of_types/"
---

# Types of Types: Common → Exotic

Programming languages offer various ways to model data through their type systems. Let's explore these concepts using Lean, which has a particularly rich type system supporting four fundamental kinds of types: function types (Pi types), product types, sum types (inductive types), and quotient types. In Lean, mathematical objects exist in a three-level hierarchy:

1.  **Universes** (e.g., `Type`, `Prop`)
2.  **Types** (elements of universes)
3.  **Terms** (elements of types)

A **type** is a term of a universe (like `Type` or `Prop`) that classifies other terms and enforces computational and logical rules about how they can be used. A **term** is a mathematical object that has a unique type, denoted using `:` (e.g., `37 : ℝ` means 37 is a term of type real numbers). This forms a strict hierarchy: terms are elements of types, and types are themselves terms of universes, but to avoid paradoxes like Russell's, there is no "type of all types" - instead, we have an infinite hierarchy of universes.

In Lean, simple types are foundational and include basic types like `nat` for natural numbers and `bool` for booleans. These types are first-class citizens, meaning they can be manipulated and passed around just like any other data. Lean's type system allows for the construction of complex types from these simple ones, such as function types (`α → β`) and product types (`α × β`).

The `#check` command in Lean is a powerful tool used to query the type of an expression. It helps verify the types of constants, functions, and expressions, ensuring they conform to expected types. For example:

```
constant m : nat    -- m is a natural number
constant n : nat    -- n is a natural number
#check m            -- output: nat
#check n + 0        -- nat
#check tt           -- boolean "true"
```

While set theory uses `∈` for membership, type theory uses `:` to denote that a term has a particular type:

```
-- In set theory: 37 ∈ ℝ
-- In type theory:
#check 37 : ℝ        -- 37 is a term of type ℝ
#check ℝ : Type      -- ℝ is a term of type Type
```

To avoid [Russell's Paradox](https://en.wikipedia.org/wiki/Russell%27s_paradox), there isn't a type of all types. Instead, we have universes:

- `Type`: Universe containing mathematical objects (e.g., `ℝ`, `ℕ`, groups, rings)
- `Prop`: Universe containing propositions (true/false statements)

For types `X` and `Y`, the type of functions from `X` to `Y` is written `X → Y`:

```
def square : ℝ → ℝ := λ x, x^2
#check square        -- ℝ → ℝ
```

The famous [Curry-Howard correspondence](https://en.wikipedia.org/wiki/Curry%E2%80%93Howard_correspondence) states:

- Propositions are types in `Prop`
- Proofs are terms of these types
- True propositions have exactly one term
- False propositions have zero terms

```
-- P → Q represents both:
-- 1. A function type from proofs of P to proofs of Q
-- 2. The logical implication P implies Q
```

**Function types**, also known as **Pi types**, represent mappings between types. They can either be dependent or non-dependent. For example:

```
-- Simple function type (non-dependent)
def square : ℝ → ℝ := λ x, x^2

-- Dependent function type (Pi type)
def vector_length : Π (n : Nat), Vector ℝ n → ℝ
| 0 _ := 0
| (n+1) v := real.sqrt (dot_product v v)
```

The Pi type notation `Π (x : A), B x` represents a dependent function type where the return type `B x` can depend on the input value `x`. When the return type doesn't depend on the input, we use the simpler arrow notation `A → B`.

**Product types** represent combinations of multiple values:

```
structure Point :=
  (x : Int)
  (y : Int)

structure Person :=
  (name : String)
  (age : Int)
  (address : String)
```

**Sum types**, also known as **inductive types**, represent alternatives:

```
inductive Shape
| circle (radius : Float)
| rectangle (width : Float) (height : Float)
| triangle (base : Float) (height : Float)

-- Example of using the equation compiler with sum types
def area : Shape → Float
| Shape.circle r := Float.pi * r * r
| Shape.rectangle w h := w * h
| Shape.triangle b h := b * h / 2
```

**Quotient types** represent values that are equivalent under some relation:

```
-- Define an equivalence relation for angles
def angle_equiv (a b : Float) : Prop :=
  ∃ k : Int, a = b + k * 360

-- Create a quotient type for angles mod 360
def Angle := quotient (setoid.mk angle_equiv)

-- Constructor for angles
def mk_angle (degrees : Float) : Angle :=
  quotient.mk degrees
```

**Dependent types** allow types to depend on values:

```
-- A vector type whose length is part of its type
inductive Vector (α : Type) : Nat → Type
| nil : Vector α 0
| cons : α → {n : Nat} → Vector α n → Vector α (n + 1)

-- A function whose return type depends on its input
def get_type (b : Bool) : Type :=
  if b then Nat else String
```

**Difference types** represent the "subtraction" of one type from another. While not directly supported in Lean, we can encode them using dependent types and proof obligations. They're particularly useful when we want to express "Type A without elements satisfying property B."

```
-- Non-zero numbers: ℝ "minus" 0
def NonZero := {x : ℝ // x ≠ 0}

-- Non-empty lists: List α "minus" empty list
def NonEmptyList (α : Type) := {l : List α // l ≠ []}

-- Even numbers: ℕ "minus" odd numbers
def Even := {n : ℕ // ∃ k, n = 2 * k}

-- Injective functions: (α → β) "minus" non-injective functions
def Injection (α β : Type) := {f : α → β // ∀ x y, f x = f y → x = y}
```

The syntax `{x : A // P x}` represents a subtype of `A` where elements satisfy predicate `P`. This is Lean's way of encoding difference types through dependent pairs (also known as Σ-types).

```
-- Creating terms of difference types
def two_nonzero : NonZero := ⟨2, by norm_num⟩

-- Extracting the underlying value
def get_value (n : NonZero) : ℝ := n.val

-- The proof of the property is also accessible
def get_proof (n : NonZero) : n.val ≠ 0 := n.property
```

For example, division can be made total (removing the divide-by-zero case) by using `NonZero`:

```
def safe_div (a : ℝ) (b : NonZero) : ℝ := a / b.val
```

This may all sound abstract, but these types are used in many practical applications, broadly the uses of these types fall into the following categories:

**Function Types (Pi Types)**

- Higher-order functions
- Dependent functions in proof assistants
- Type families
- Polymorphic functions

**Product Types**

- Configuration records
- Data structures
- Complex domain models

**Sum Types**

- Error handling
- State machines
- Abstract syntax trees

**Quotient Types**

- Modular arithmetic
- Equivalence classes
- Mathematical structures

**Difference Types**

- Enforcing invariants at compile time
- Removing edge cases from algorithms
- Making implicit assumptions explicit
- Strengthening function preconditions
