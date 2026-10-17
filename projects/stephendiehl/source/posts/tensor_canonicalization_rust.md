---
date: "2025-06-29"
tags: ["math", "rust", "mathematics"]
url: "https://www.stephendiehl.com/posts/tensor_canonicalization_rust/"
---

# Fast Tensor Canonicalization in Rust

I recently had a need to do fast tensor canonicalization, but the library ecosystem for this was lacking. In physics and differential geometry, manipulating tensors and especially bringing them into canonical form under their index symmetries is a recurring and computationally intensive task. Every undergrad who takes their first relativity class understands the tensor gymnastics required can often be daunting, and in real-life problems they get even more gnarly.

- [Github Source Code](https://github.com/sdiehl/butler-portugal)
- `git clone https://github.com/sdiehl/butler-portugal.git`

While existing systems like Mathematica offer some support, and the [xPerm](https://www.xact.es/xPerm/index.html) library provides a high-performance C implementation, there was no, let's just say "modern" and open source solution. So I built a new [`butler-portugal` library](https://docs.rs/butler-portugal/latest/butler_portugal/), a pure Rust implementation of the Butler-Portugal algorithm for tensor canonicalization, with bindings for C and Python.

The canonical algorithm for tensor canonicalization (har har) is the little known **Butler-Portugal algorithm**, which is a method for bringing tensors into a unique, minimal form under their symmetries. The algorithm recasts tensor canonicalization in the language of permutation group theory, specifically as the search for a minimal representative in a double coset of the symmetric group. Write a tensor as a map $g$ from slots to index labels. The slot symmetries $S$ (permutations of slots that leave the tensor invariant up to sign) act on the right, and the dummy symmetries $D$ (renaming contracted index pairs, and swapping the upper and lower ends of a pair when the metric allows it) act on the left. Every equivalent way of writing the tensor is then some $d \cdot g \cdot s$ in the double coset $DgS$, and the canonical form is the lexicographically minimal element of that double coset. If the same arrangement can be reached with both signs, the tensor is identically zero.

The trick that keeps this tractable is that neither group is ever enumerated. The **Schreier-Sims algorithm** builds a stabilizer chain for $S$ with base $0, 1, \dots, n-1$, and the search walks that chain one slot at a time. At each level it tries every transversal element, lowers the label that lands in the current slot to the minimum of its orbit under $D$, keeps whichever choices achieve the smallest label, and moves on to the next slot. Only ties cause branching, and survivors that write the tensor identically are merged, so the search stays small even for groups with millions of elements.

Let's step back and consider what symbolic tensor manipulation actually means. Unlike in machine learning, in physics tensors aren't just multidimensional arrays. They're objects with named indices, each carrying a variance (covariant and contravariant, or downstairs and upstairs respectively), and they transform in specific ways under coordinate changes. Their symmetries, like being symmetric or antisymmetric under index exchange or more elaborate patterns as in the Riemann tensor, are essential algebraic properties that encode physical laws.

Take general relativity as a motivating example. The Einstein field equations,

$$G_{\mu\nu} = 8\pi T_{\mu\nu}$$

relate the Einstein tensor $G_{\mu\nu}$, which encodes spacetime curvature, to the stress-energy tensor $T_{\mu\nu}$. Both are symmetric rank-2 tensors meaning we can permute their indices freely in our computations:

$$G_{\mu\nu} = G_{\nu\mu}, \qquad T_{\mu\nu} = T_{\nu\mu}$$

When working with these objects symbolically, you want to be able to manipulate expressions, contract indices, and simplify results, all while respecting these symmetries. For instance, the Riemann curvature tensor $R_{\mu\nu\rho\sigma}$ is a rank-4 object with a rich set of symmetries:

$$R_{\mu\nu\rho\sigma} = -R_{\nu\mu\rho\sigma} = -R_{\mu\nu\sigma\rho} = R_{\rho\sigma\mu\nu}$$

and it satisfies the so-called [first Bianchi identity](https://en.wikipedia.org/wiki/Curvature_form#Bianchi_identities),

$$R_{\mu[\nu\rho\sigma]} = 0$$

where the brackets denote antisymmetrization over the enclosed indices. These symmetry identities are crucial for simplifying calculations and for ensuring that derived quantities, like the Ricci tensor or scalar, have the right properties.

Contraction is another fundamental operation. It reduces the rank of a tensor by summing over a pair of indices, one upper and one lower:

$$T^\mu_{\ \mu} = \sum_{\mu} T^\mu_{\ \mu}$$

This is how you build invariants, like the Ricci scalar which is a single real number assigned to each point on a Riemannian manifold that measures the degree to which the geometry near that point deviates from flatness:

$$R = g^{\mu\nu} R_{\mu\nu}$$

Contracted indices are where canonicalization gets genuinely hard. The name of a dummy index carries no meaning, so \\( R\^a{}*{bac} \\) and \\( R\^z{}*{bzc} \\) are the same object, and a canonicalizer has to account for renaming dummies on top of the slot symmetries. That is exactly the $D$ side of the double coset. To make this concrete, here's a minimal Rust example using `butler-portugal` to canonicalize the Ricci contraction of the Riemann tensor:

``` rust
use butler_portugal::*;

fn main() -> Result<()> {
    // R^a_{cab}, a contraction of the Riemann tensor
    let mut r = Tensor::new(
        "R",
        vec![
            TensorIndex::contravariant("a", 0),
            TensorIndex::covariant("c", 1),
            TensorIndex::covariant("a", 2),
            TensorIndex::covariant("b", 3),
        ],
    );
    r.add_symmetry(Symmetry::antisymmetric(vec![0, 1]));
    r.add_symmetry(Symmetry::antisymmetric(vec![2, 3]));
    r.add_symmetry(Symmetry::symmetric_pairs(vec![(0, 1), (2, 3)]));

    let canonical = canonicalize(&r)?;
    println!("{canonical}"); // R_b_a_c^a
    println!("{}", canonical.coefficient());
    Ok(())
}
```

The repeated name `a`, appearing once upstairs and once downstairs, is detected as a contraction automatically. The output is the unique, minimal representative of the tensor under its symmetry group with the accumulated sign folded into the coefficient, and contracted indices renamed to the sorted dummy names. A tensor that vanishes by symmetry, like $F_{aa}$ for an antisymmetric $F$, comes back with coefficient zero.

At the core of the library is a representation of tensors as objects with named indices, each carrying variance, position, and an optional index type (so that spacetime and spinor indices never pair with each other). Each index type can declare a metric that is symmetric, antisymmetric (raising and lowering costs a sign, as with the spinor metric), or absent. Symmetries are encoded as generators of a group of signed permutations. Signs are handled with a neat trick from xPerm: a signed permutation of $n$ slots is encoded as an ordinary permutation of $n + 2$ points, where a sign of $-1$ swaps the two extra points. An ordinary Schreier-Sims chain over $n + 2$ points then describes the signed group, and the tensor is identically zero exactly when the group contains the element that swaps only the two sign points.

The symmetries of a tensor are encoded as group actions on its indices. The most common types are:

- **Symmetric**: invariant under exchange of specified indices, $$T_{ab} = T_{ba}$$
- **Antisymmetric**: changes sign under exchange, $$F_{ab} = -F_{ba}$$
- **Cyclic**: invariant under cyclic permutations, $$C_{abc} = C_{bca} = C_{cab}$$
- **Pair symmetry**: exchange of index pairs, as in the Riemann tensor, $$R_{abcd} = R_{cdab}$$
- **Custom**: arbitrary signed generators, from which Schreier-Sims builds the full group.

Products of tensors are handled the same way. `Tensor::product` glues factors into one commuting monomial, adding the symmetry that exchanges identical factors, so something like $F_{ab} F^{ab}$ or a Kretschmann scalar $R_{abcd} R^{abcd}$ canonicalizes with contractions running between factors. Alternatively the library also supports projection onto irreducible representations specified by [Young tableaux](https://en.wikipedia.org/wiki/Young_tableau), enabling the decomposition of tensors into symmetry types corresponding to the representations of the symmetric group. I won't explain this here, but it's well-documented elsewhere and I put the references in the repository.

My implementation draws inspiration from xPerm, which is widely regarded as the gold standard for tensor canonicalization in symbolic computation. However, porting the algorithm to Rust was quite a challenge as the C and Mathematica code is quite dated and not particularly well-documented. The test suite checks against a reference corpus of a thousand canonical forms, a brute-force double coset search over small groups, and the classic Riemann identities. On Riemann monomials it matches xPerm's output and runs faster. So hopefully if you're a practitioner in the field, you can look at my implementation, compare it with the reference, and this makes it easier for some poor future physics grad student who doesn't need to build the same thing again to do their research.

The library also builds as a C shared library, with a header in `include/butler_portugal.h`, so it can be linked from anything that speaks the C ABI:

``` bash
cargo build --release
clang -Iinclude -Ltarget/release -lbutler_portugal app.c -o app
```

And there is a small Python wrapper over the C API, with a fluent interface:

``` python
from butler_portugal import Tensor

r = Tensor("R", ["^a", "c", "a", "b"]).antisymmetric(0, 1).antisymmetric(2, 3)
print(r.symmetric_pairs((0, 1), (2, 3)).canonicalize())  # R_b_a_c^a
```

So that's `butler-portugal`, which hopefully brings high-performance tensor canonicalization to the Rust ecosystem. You can install it with:

``` bash
cargo add butler-portugal
```

Or for Python with uv:

``` bash
uv add git+https://github.com/sdiehl/butler-portugal --tag v0.4.0
```

The library is quite efficient, MIT-licensed and hopefully useful in any domain where the manipulation of tensors with complex symmetries is required, and hopefully useful for at least one other person besides myself!
