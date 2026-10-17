---
date: "2025-06-11"
tags: ["math", "philosophy", "mathematics"]
url: "https://www.stephendiehl.com/posts/future_math_weird/"
---

# The Future of Maths May Be Deeply Weird

In fiction there is a popular image of a mathematician as a lone solitary figure, a figure of lost in their own world in a world of intense concentration before a chalkboard dense with arcane symbols. That may have had some truth in the past but the reality is that mathematics these days is a far more social phenomenon and [future mathematicians](https://www.youtube.com/watch?v=Ch_cNbk39xY) may have a lot more in common with software engineers toiling away on pull requests on Github than they do with the archetype of a Andrew Wiles-like genius toiling away in isolation for a decade to prove Fermat's Last Theorem.

The field of automated theorem proving is a field of computer science that develops software systems capable of automatically constructing mathematical proofs or verifying the correctness of human-written proofs. These systems use formal logic, type systems, and symbolic reasoning to check that each step in a mathematical argument follows logically from the previous steps and established axioms. By encoding mathematical knowledge in a precise, machine-readable format, automated theorem provers can eliminate human error and provide absolute certainty about the validity of mathematical results.

For those that don't follow the theorem proving ecosystem ([Lean4](https://github.com/leanprover/lean4), Coq and their predecessors Isabelle, Agda, HOL4, etc) are tools that were once confined to the esoteric frontiers of computer science research, are poised to reshape the very practice and philosophy of mathematics. The future of the discipline may be far stranger, and more collaborative, than we can currently imagine. And with the simultaneous advent of language models, may be much more exciting and rapid than we can currently imagine.

Below is a [formalization of Euclid's theorem](https://github.com/leanprover-community/mathlib4/blob/7deb334c5f5104f4edad1a6396dd02a8cddefb86/Mathlib/Data/Nat/Prime/Infinite.lean#L27-L38) on the infinitude of primes in Lean. The proof follows Euclid's constructive approach: given any natural number `n`, it constructs a prime `p ≥ n` by considering the smallest prime factor of `n! + 1`. The Lean proof uses several [tactics](https://github.com/haruhisa-enomoto/mathlib4-all-tactics/blob/main/all-tactics.md) and lemmas: `minFac` finds the minimal prime factor, `factorial_pos` establishes that factorials are positive, `minFac_prime` proves the minimal factor is indeed prime, and `dvd_factorial` handles divisibility properties. The crucial insight, formalized through `Nat.dvd_add_iff_right`, shows that if a prime divides both `n!` and `n! + 1`, it must divide their difference (which is 1), leading to a contradiction since primes cannot divide 1. This is the proof that every undergraduate mathematics student learns in their first year.

```
/-- Euclid's theorem on the **infinitude of primes**.
Here given in the form: for every `n`, there exists a prime number `p ≥ n`. -/
theorem exists_infinite_primes (n : ℕ) : ∃ p, n ≤ p ∧ Prime p :=
  let p := minFac (n ! + 1)
  have f1 : n ! + 1 ≠ 1 := ne_of_gt <| succ_lt_succ <| factorial_pos _
  have pp : Prime p := minFac_prime f1
  have np : n ≤ p :=
    le_of_not_ge fun h =>
      have h₁ : p ∣ n ! := dvd_factorial (minFac_pos _) h
      have h₂ : p ∣ 1 := (Nat.dvd_add_iff_right h₁).2 (minFac_dvd _)
      pp.not_dvd_one h₂
  ⟨p, np, pp⟩
```

The formalization of the standard undergraduate mathematics curriculum in a format like this is no longer a distant dream. It is an achievable, near-term goal. Need only have a look at the standard library [Mathlib](https://leanprover-community.github.io/mathlib4_docs/Mathlib/Init.html) and the amount of progress on the standard set of theorems is now incredibly vast and expressive. Another good example of this progress is the formalization of Terence Tao's textbook, [*Analysis I*](https://github.com/teorth/analysis) which has been meticulously translated into an explorable, crosslinked and machine-checkable set of theorems. A future in which is every textbook has a digitial equivelent and any theorem or lemma can be ["double clicked"](https://teorth.github.io/analysis/sec21/) to expand its context and definitions is hardly science fiction anymore. It's essentially here.

If you want to experiment with Lean, you can get started very quickly with the following steps:

1.  Install Visual Studio Code.
2.  Install Lean + Elan:

``` sh
curl https://elan.lean-lang.org/elan-init.sh -sSf | sh
elan default leanprover/lean4:stable
```

3.  Install the Lean 4 VS Code Extension: Open VS Code → Extensions → search for Lean 4 (author: `leanprover.lean4`)
4.  Initialize a new Lean project:

``` sh
lake init my_project
cd my_project
lake build
```

The Lean ecosystem also has a burgeoning flora of open source tools. The [Mathlib](https://leanprover-community.github.io/mathlib4_docs/) library is a massive, collaborative repository of formalized mathematics, growing daily. To navigate this vast digital library, a suite of search engines has emerged. [Moogle](https://moogle-morphlabs.vercel.app/) provides semantic search, allowing one to find theorems based on conceptual meaning rather than exact phrasing. [Loogle](https://loogle.lean-lang.org/) specializes in finding expressions that match a specific logical structure, and [LeanSearch](https://leansearch.net/) offers natural language queries. For those wishing to experiment, the [Lean 4 Playground](https://live.lean-lang.org/) provides an interactive environment to write and check proofs directly in a web browser.

Consider the educational landscape this might enable. Imagine a future where every first-year mathematics student downloads a single, 200-megabyte file containing the entire verified foundation of their subject. They would be guided by an LLM-assisted tutor, a system capable of planning lessons, verbalizing complex proofs, and providing instant, theorem prover checkable feedback on their own attempts at proofs. This is not science fiction; the technology for curriculum planning and natural language explanation is largely within our grasp today. Of course, this vision must be tempered with realism. Science is intrinsically a social phenomenon. A great deal of mathematical research is about developing taste, an aesthetic for what constitutes an elegant proof, and the intuition for which problems are worth pursuing. These are intangible qualities of the craft, born from culture and collaboration, which a machine may never fully capture.

As we move from established knowledge to the open frontiers of research, these tools present new and exciting possibilities. While the body of formalized proofs is growing, there remains a relative scarcity of formalized open conjectures. A project like the Deepmind's [index of formal conjectures](https://github.com/google-deepmind/formal-conjectures) is valuable for several reasons. First, it provides a clear and challenging set of benchmarks for the development of AI-assisted automated theorem provers. Second, the very act of formalization forces mathematicians to clarify the precise, unambiguous meaning of a conjecture. It is almost certain that we will soon see a powerful, neurosymbolic system, a kind of next generation AlphaGeometry, unleashed upon this list in an attempt to systematically push the boundaries of machine-generated mathematics.

The artifacts produced by such research will themselves be objects of profound study. Picture a future where one can download the twenty-gigabyte weights of a neural network that has been trained through reinforcement learning to achieve mastery over the entirety of undergraduate mathematics, using theorem prover feedback as its reward function. The existence of such a model would provoke fascinating theoretical and philosophical questions. Is there a smaller, more elegant model that can achieve the same result? By examining its failure modes, what can we learn about the complexity hurdles in mathematics? How would it handle alternative axiom sets, and could it ever develop an "understanding" of Gödel's incompleteness, intuiting when a problem is fundamentally independent of the given axioms? **There are many deep meta-mathematical questions we could ask if we had an existence proof of a giant, inscrutable matrix that can "itself" do mathematics**. There is no guarantee such a thing can exist (and the usual Gödel, Turing, and Hilbert's Program analyses still apply), but if it can — even to a limited extent — that opens up a whole new world of questions.

Stretching our imagination further, we can envision a far future where this technology reaches its zenith. A dedicated data center could be tasked with a grand challenge, like the Riemann hypothesis. After weeks of processing, it might return a positive result: a complete, machine-checkable proof. Yet, the proof's internal structure could be so byzantine, so alien in its logic, that no human could ever hope to grasp its intuitive thrust, much like the computer-assisted proof of the Four Color Theorem or the controversy surrounding the Mochizuki proof of the ABC conjecture. This would usher in a disquieting new reality, a world where machines, not unlike the [hyper-intelligent Minds](https://www.youtube.com/watch?v=g2T-TStvRNM) from Iain M. Banks' Culture series, venture into intellectual territories forever beyond human comprehension. We would "know" (epistomigical concerns aside) and accept certain mathematical truths, but as humans we would never intuit or understand why they are true. And that is a very different world in which science happens.

The prospect of inscrutable machine-generated proofs touches on fundamental questions about the nature of knowledge and understanding. Bill Thurston, in his essay ["On Proof and Progress in Mathematics,"](https://www.math.toronto.edu/mccann/199/thurston.pdf) captured this tension:

> The rapid advance of computers has helped dramatize this point, because computers and people are very different. For instance, when Appel and Haken completed a proof of the 4-color map theorem using a massive automatic computation, it evoked much controversy. I interpret the controversy as having little to do with doubt people had as to the veracity of the theorem or the correctness of the proof. Rather, it reflected a continuing desire for human understanding of a proof, in addition to knowledge that the theorem is true.

New developments also raise teleological questions about the very purpose of of the maths discipline. Mathematics has never been merely about accumulating true statements—it's about developing insight, building intuition, and creating frameworks that illuminate the underlying structure of reality. The process of discovery, the "aha!" moments of understanding, the elegant connections between seemingly disparate areas: these constitute much of what makes mathematics meaningful to its practitioners. As Feynman once said: "Physics is like sex. Sure, it may give some practical results, but that's not why we do it."

To be honest, it's hard not to get emotional about this prospect. In that future, many of us would still care deeply about learning *why* theorems are true. The process of answering those questions represents one of the most beautiful and fulfilling aspects of intellectual life. Maths as a practice is more than a matter of checking theorems off a cosmic to-do list. But if you buy into this premise, and if you're a burgeoning model designer, [all you have](https://github.com/google-deepmind/formal-conjectures/blob/9d70f49b89d6b2697af4234d377012ea26cfbb64/FormalConjectures/Wikipedia/RiemannZetaValues.lean#L76) to do win a Nobel (and several other prizes) is just train your stochastic parrot to autocomplete the next $n$ tokens such that the Lean theorem validates your answer.

```
theorem infinite_irrational_at_odd :
    { n : ℕ | ∃ x, Irrational x ∧ riemannZeta (2 * n + 1) = x }.Infinite := by
    ...
```

You'll also land yourself a cool million dollars from the Millennium Foundation in the process, which might pay for 1/100 of your GPU bill. Or maybe just make the [AI-slop Infinite Jest](https://www.stephendiehl.com/posts/ai_slop_2027/) version of TikTok, because alas, that's probably a much better short-term return on investment than solving Riemann and pushing the frontiers of human knowledge.

But joking aside, we're not there yet. Not even close. Need only watch Terence Tao's [recent YouTube streams](https://www.youtube.com/watch?v=zZr54G7ec7A) with Claude and o4 on Lean theorem proving reveal the stark limitations of our current systems when faced with even moderately challenging mathematical problems. The models struggle with basic proof strategies, make elementary logical errors, and often produce syntactically correct but mathematically meaningless Lean code. Yet watching these experiments unfold, it doesn't require a tremendous leap of imagination to envision how rapidly this landscape might change. The combination of increasingly sophisticated language models, improved integration with theorem provers, and better [training methodologies](https://github.com/deepseek-ai/DeepSeek-Prover-V2) specifically designed for formal mathematical reasoning suggests that today's clumsy first attempts may give way to genuinely more capable models. But of course, progress is never certain. There could be a lot more hurdles to overcome.

This prospect may seem to diminish the human element of mathematics, but perhaps that fear is unfounded. Computers have decisively solved the games of chess and Go. Stockfish and AlphaGo can defeat any human player on Earth with ease. Yet, chess has never been more popular. These powerful systems have not killed the game; they have become indispensable tools for training, analysis, and appreciation, making the game more accessible to more people than ever before. It is possible the same will hold true for mathematics. As these strange and powerful new tools proliferate, they may not render the human mathematician obsolete. Instead, they might democratize access to mathematical reasoning, allowing more people to participate in the oldest and most abstract of sciences. Perhaps the frontiers of knowledge will widen at an incredible pace, but the path to that frontier will be more accessible for everyone. And maybe that is a very bright future indeed.
