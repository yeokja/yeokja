---
date: "2019-02-01"
tags: ["mathematics", "formal"]
url: "https://www.stephendiehl.com/posts/lean/"
---

# Proving Trivial Theorems in Lean

When starting with a proof assistant like Lean 4, it's best to begin with the most elementary proofs possible. While these may seem trivially obvious to a mathematician, implementing them in a formal system helps build intuition for how the proof assistant works. By starting with basic logical statements that we know are true, we can focus on learning the syntax and mechanics of formal verification without getting lost in complex mathematics. Let's look at some of these foundational proofs to get comfortable with Lean's proof system.

## Example 1: Identity

Our first theorem shows that any proposition implies itself:

```
theorem self_implies_self (p : Prop) :
    p → p := fun h => h
```

This can also be written using the `λ` notation or with an explicit proof:

```
theorem self_implies_self' (p : Prop) :
    p → p :=
    λ h => h

theorem self_implies_self'' (p : Prop) :
    p → p := by
  intro h
  exact h
```

## Example 2: And Elimination

We can prove that if we have `p ∧ q`, then we have `p`:

```
theorem and_elim_left (p q : Prop) : p ∧ q → p := by
  intro h
  exact h.left
```

## Example 3: Or Introduction

We can prove that if we have `p`, then we have `p ∨ q`:

```
theorem or_intro_left (p q : Prop) : p → p ∨ q := by
  intro h
  exact Or.inl h
```

## Example 4: Double Negation Introduction

We can prove that if `p` is true, then `¬¬p` is also true:

```
theorem double_neg_intro (p : Prop) : p → ¬¬p := by
  intro h
  intro hn
  exact hn h
```

## Example 5: Law of Excluded Middle

In classical logic, for any proposition `p`, either `p` is true or `¬p` is true. This is an axiom in Lean's classical logic:

```
theorem excluded_middle (p : Prop) : p ∨ ¬p := by
  exact Classical.em p
```

Note that this is different from our previous examples. While the other theorems could be proved constructively, the law of excluded middle requires the `Classical` axiom. In intuitionistic logic, this principle is not assumed. We use `Classical.em` to access this axiom in Lean.

You can also write this using pattern matching:

```
theorem excluded_middle' (p : Prop) : p ∨ ¬p :=
  match Classical.em p with
  | Or.inl h  => Or.inl h    -- if p is true, return p
  | Or.inr h  => Or.inr h    -- if ¬p is true, return ¬p
```
