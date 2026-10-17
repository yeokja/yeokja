---
date: "2024-08-05"
tags: ["mathematics", "logic"]
url: "https://www.stephendiehl.com/posts/diaconescu/"
---

# Diaconescu's Theorem

Diaconescu's Theorem, discovered by Radu Diaconescu in 1975, reveals a fascinating and surprising connection between two fundamental principles in mathematics: the Axiom of Choice and the Law of Excluded Middle. In simple terms, the theorem states that if you accept the Axiom of Choice, you must also accept the Law of Excluded Middle. The theorem was independently discovered by Goodman and Myhill in 1978, and is sometimes called the Diaconescu-Goodman-Myhill theorem. Interestingly, the Errett Bishop had posed this result as an exercise in his 1967 book on constructive analysis, before Diaconescu's publication.

As background, the Law of Excluded Middle states that for any proposition $P$, either $P$ is true or $P$ is false - there is no middle condition. Or symbolically, $P \lor \neg P$. For example, either a number is even or it's not even. This seems intuitive, but not all mathematical frameworks accept this principle!

The Axiom of Choice states that given any collection of non-empty sets, it's possible to select exactly one element from each set to form a new set. While this also seems intuitive, it has some surprising consequences.

Diaconescu's Theorem shows that:

$$\text{Axiom of Choice} \implies \text{Law of Excluded Middle}$$

Before diving into the proof, let's establish some important concepts:

- A set is called **decidable** if for all elements, we can determine whether they belong to the set or not. Formally, for any element $x$, either $x \in S$ or $x \notin S$.
- A set is called a **choice set** if every surjection onto it splits (meaning there exists a map going back such that their composition is the identity).

The proof works through the following steps:

1.  Start with any proposition $P$ that we want to prove must be either true or false

2.  Construct two sets:

    - $U = {x \in {0,1} \mid (x = 0) \lor P}$
    - $V = {x \in {0,1} \mid (x = 1) \lor P}$

3.  Both sets are guaranteed to be non-empty:

    - $U$ contains 0 (because $x = 0$ is true for $x = 0$)
    - $V$ contains 1 (because $x = 1$ is true for $x = 1$)

4.  Form the set $X = {U, V}$. By the Axiom of Choice, since both $U$ and $V$ are non-empty, we can select one element from each set.

5.  Let $f$ be our choice function. Since equality is decidable in choice sets (a key lemma from the formal proof), we can analyze $f(U)$ and $f(V)$:

    - If $f(U) = 0$, then $P$ must be false ($\neg P$)
    - If $f(U) = 1$, then $P$ must be true
    - The same analysis applies to $f(V)$

6.  Through this analysis, we can conclude $P \lor \neg P$ must hold.

We can formalize this theorem in Lean. The proof uses function extensionality and propositional extensionality alongside choice. The Lean proof follows the same logical structure as our mathematical proof, but works with propositions instead of sets. It uses function and propositional extensionality to handle equality of predicates, and the Axiom of Choice appears in the form of the `choose` function, which selects witnesses from existential statements.

First, we state the theorem as the type declaration of `em`:

```
theorem em (p : Prop) : p ∨ ¬p :=
```

Let's break down how this formal proof works:

1.  First, we define two predicates `U` and `V` that take a proposition `x` and return another proposition:

    ```
    let U (x : Prop) : Prop := x = True ∨ p
    let V (x : Prop) : Prop := x = False ∨ p
    ```

    These correspond to the sets U and V in our mathematical proof, but work with propositions instead of {0,1}.

2.  We prove that both U and V are inhabited (non-empty):

    ```
    have exU : ∃ x, U x := ⟨True, Or.inl rfl⟩   -- True satisfies U because True = True
    have exV : ∃ x, V x := ⟨False, Or.inl rfl⟩  -- False satisfies V because False = False
    ```

3.  Using the Axiom of Choice (via `choose`), we select witnesses u and v from U and V:

    ```
    let u : Prop := choose exU
    let v : Prop := choose exV
    have u_def : U u := choose_spec exU  -- u satisfies U
    have v_def : V v := choose_spec exV  -- v satisfies V
    ```

4.  The key insight is that either u ≠ v, or p must be true:

    ```
    have not_uv_or_p : u ≠ v ∨ p :=
      match u_def, v_def with
      | Or.inr h, _ => Or.inr h        -- If U(u) because of p, then p is true
      | _, Or.inr h => Or.inr h        -- If V(v) because of p, then p is true
      | Or.inl hut, Or.inl hvf =>      -- Otherwise, u = True and v = False
        have hne : u ≠ v := by simp [hvf, hut, true_ne_false]
        Or.inl hne
    ```

5.  We then prove that if p is true, then u = v:

    ```
    have p_implies_uv : p → u = v :=
      fun hp =>
      have hpred : U = V :=    -- If p is true, then U and V are the same predicate
        funext fun x =>        -- Using function extensionality
          have hl : (x = True ∨ p) → (x = False ∨ p) :=
            fun _ => Or.inr hp
          have hr : (x = False ∨ p) → (x = True ∨ p) :=
            fun _ => Or.inr hp
          show (x = True ∨ p) = (x = False ∨ p) from
            propext (Iff.intro hl hr)   -- Using propositional extensionality
    ```

6.  Finally, we conclude p ∨ ¬p by case analysis:

    ```
    match not_uv_or_p with
    | Or.inl hne => Or.inr (mt p_implies_uv hne)  -- If u ≠ v, then ¬p
    | Or.inr h   => Or.inl h                      -- If p, then p
    ```

Diaconescu's theorem demonstrates a profound connection between choice principles and classical logic. This result has significant metamathematical implications: it reveals that choice principles are inherently non-constructive, as they force classical reasoning even in otherwise constructive settings. From a foundational perspective, this shows that choice principles are not logically neutral - they implicitly commit us to classical logic. For constructive mathematics, this necessitates careful consideration of choice principles, as even seemingly innocuous uses of AC can inadvertently classical-ize an otherwise constructive development. The theorem thus exemplifies how foundational axioms can have subtle but far-reaching consequences for the logical character of mathematical theories. You could say that when it comes to the Axiom of Choice and the Law of Excluded Middle, there's no choosing the middle ground!
