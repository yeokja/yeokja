---
date: "2018-05-02"
tags: ["mathematics"]
url: "https://www.stephendiehl.com/posts/integral_zoo/"
---

# Integral Zoo

Similar to the [derivative zoo](derivative_zoo.md), there are many different constructions we call "integrals". Here are some of the most interesting ones:

1.  [Classical Integral](#classical-integral)
2.  [Line Integral](#line-integral)
3.  [Surface Integral](#surface-integral)
4.  [Contour Integral](#contour-integral)
5.  [Stieltjes Integral](#stieltjes-integral)
6.  [Lebesgue Integral](#lebesgue-integral)
7.  [Fractional Integral](#fractional-integral)
8.  [Path Integral](#path-integral)
9.  [Bochner Integral](#bochner-integral)
10. [Ito Integral](#ito-integral)
11. [Hadamard Integral](#hadamard-integral)
12. [Darboux Integral](#darboux-integral)
13. [Wiener Integral](#wiener-integral)

## Classical Integral

The indefinite integral, or antiderivative, of a function $f(x)$ represents the family of functions $F(x)$ whose derivative is $f(x)$. It is denoted by $\int f(x) dx = F(x) + C$, where $F'(x) = f(x)$ and $C$ is an arbitrary constant of integration, reflecting that the derivative of a constant is zero. This concept arises as the inverse operation to differentiation and is fundamentally linked to definite integrals via the Fundamental Theorem of Calculus.

## Line Integral

A line integral generalizes the definite integral to integration along a curve $C$ in 2D or 3D space. There are two main types. For a scalar field $f$, the line integral with respect to arc length measures the "mass" or total value along the curve: $\int_C f ds = \int_a^b f(\mathbf{r}(t)) |\mathbf{r}'(t)| dt$. For a vector field $\mathbf{F}$, the line integral measures the work done by the field along the curve: $\int_C \mathbf{F} \cdot d\mathbf{r} = \int_a^b \mathbf{F}(\mathbf{r}(t)) \cdot \mathbf{r}'(t) dt$. Both are calculated by parametrizing the curve $C$ by $\mathbf{r}(t)$ for $t \in [a, b]$ and converting the integral into a standard definite integral in terms of the parameter $t$.

## Surface Integral

A surface integral extends integration to surfaces $S$ in 3D space, analogous to how line integrals extend definite integrals to curves. For a scalar field $g$, the surface integral $\iint_S g dS = \iint_D g(\mathbf{r}(u,v)) |\mathbf{r}_u \times \mathbf{r}_v| dA$ computes the total value or "mass" over the surface. For a vector field $\mathbf{F}$, the surface integral (often called flux) measures the net flow of the vector field through the surface: $\iint_S \mathbf{F} \cdot d\mathbf{S} = \iint_S \mathbf{F} \cdot \mathbf{n} dS = \iint_D \mathbf{F}(\mathbf{r}(u,v)) \cdot (\mathbf{r}_u \times \mathbf{r}_v) dA$. These are computed by parametrizing the surface $S$ by $\mathbf{r}(u,v)$ over a domain $D$ in the $uv$-plane and evaluating a double integral over $D$.

## Contour Integral

A contour integral is essentially a line integral performed in the complex plane. For a complex function $f(z)$ and a curve (contour) $C$ parametrized by $\gamma(t)$ for $t \in [a, b]$, the integral is defined as $\oint_C f(z) dz = \int_a^b f(\gamma(t)) \gamma'(t) dt$. Note the complex multiplication $f(\gamma(t))\gamma'(t)$, which differs from the dot product used in real vector field line integrals. Contour integrals are fundamental in complex analysis, forming the basis for Cauchy's Integral Theorem, Cauchy's Integral Formula, and the Residue Theorem, which are powerful tools for evaluating integrals and analyzing complex functions.

## Stieltjes Integral

The Riemann-Stieltjes integral, denoted $\int_a^b f(x) d\alpha(x)$, generalizes the Riemann integral by integrating a function $f$ with respect to another function $\alpha$ (called the integrator), rather than just with respect to $x$ (which corresponds to $\alpha(x) = x$). It is defined via limits of sums like $\sum f(c_i) [\alpha(x_i) - \alpha(x_{i-1})]$. This allows assigning different "weights" or "measures" to different parts of the interval, determined by the changes in $\alpha$. It is particularly useful in probability and statistics for calculating expected values with respect to cumulative distribution functions (if $\alpha$ is a CDF) and has applications in physics and functional analysis. The Lebesgue-Stieltjes integral is a more powerful measure-theoretic generalization.

## Lebesgue Integral

The Lebesgue integral is a more general and powerful construction of the integral than the Riemann integral, based on measure theory. Instead of partitioning the domain $[a, b]$ (as in Riemann), the Lebesgue integral essentially partitions the *range* of the function $f$ and measures the size (using a measure $\mu$, typically Lebesgue measure) of the sets in the domain where $f$ takes values in each part of the range partition. For a non-negative measurable function $f$, it's defined as $\int_E f d\mu = \sup { \int_E s d\mu \mid 0 \le s \le f, s \text{ is a simple function} }$, and extended to general functions via $f = f^{+} - f^{-}$. It can integrate a wider class of functions (including highly discontinuous ones) and has superior convergence theorems (like the Dominated Convergence Theorem and Monotone Convergence Theorem), making it standard in modern analysis and probability theory.

## Fractional Integral

Fractional integrals generalize the notion of repeated integration to non-integer orders $\alpha > 0$. The most common form is the Riemann-Liouville fractional integral, defined as $_a I_t^\alpha f(t) = \frac{1}{\Gamma(\alpha)} \int_a^t (t-\tau)^{\alpha-1} f(\tau) d\tau$, where $\Gamma$ is the Gamma function. This formula can be motivated by generalizing Cauchy's formula for $n$-fold integration. Fractional integrals act as inverse operators (in a sense) to fractional derivatives and appear alongside them in the study of fractional calculus, used to model systems exhibiting memory or non-local behavior.

## Path Integral

The path integral, particularly the Feynman path integral used in quantum mechanics and quantum field theory, is a concept for computing quantum amplitudes or statistical mechanical partition functions by summing (or "integrating") over all possible trajectories (paths) a system can take between initial and final states. Schematically, it might look like $\int \mathcal{D}[x(t)] e^{i S[x(t)]/\hbar}$ or $\int \mathcal{D}[\phi] e^{-S[\phi]}$, where $S$ is the action functional and $\mathcal{D}[x(t)]$ represents a "measure" on the space of paths. It's not a standard mathematically rigorous integral in the sense of Riemann or Lebesgue; its definition typically relies on discretization (time-slicing) and taking limits, or other specialized formalisms.

## Bochner Integral

The Bochner integral extends the Lebesgue integral to functions $f: X \to B$ that take values in a Banach space $B$ (a complete normed vector space), defined over a measure space $(X, \Sigma, \mu)$. Similar to the Lebesgue construction, it is defined using limits of simple functions (which take finitely many values in $B$). The resulting integral $\int_X f d\mu$ is an element of the Banach space $B$. It is a crucial tool in functional analysis, vector measure theory, and the study of PDEs with values in Banach spaces.

## Ito Integral

The Itô integral is a fundamental concept in stochastic calculus, defining integration with respect to a stochastic process, typically Brownian motion (or a Wiener process) $W_t$. For a suitable (non-anticipating or adapted) stochastic process $H_t$, the Itô integral $\int_0^T H_t dW_t$ is defined as a limit of sums: $\lim_{n\to\infty} \sum_{i=0}^{n-1} H_{t_i} (W_{t_{i+1}} - W_{t_i})$. A key feature is that the integrand $H_{t_i}$ is evaluated at the *beginning* of the time interval $[t_i, t_{i+1}]$. The result is a random variable, and Itô integrals do not follow classical calculus rules; for instance, the chain rule is replaced by Itô's lemma. It's essential for modeling systems driven by random noise, particularly in finance and physics.

## Hadamard Integral

The Hadamard finite part integral (or Hadamard regularization) is a method for assigning a finite value to certain types of improper integrals that diverge in the usual sense due to strong singularities. For example, for an integral $\int_a^b \frac{f(x)}{(x-x_0)^k} dx$ where $f$ is smooth and $x_0 \in (a,b)$, the Hadamard finite part involves subtracting the divergent terms derived from the Taylor expansion of $f$ around $x_0$. It's formally defined using limits, analytic continuation, or within the theory of distributions. It provides a way to regularize divergent integrals that appear in physics and applied mathematics, particularly in boundary element methods and quantum field theory.

## Darboux Integral

The Darboux integral is a rigorous approach to defining the Riemann integral using upper and lower sums. For a bounded function $f$ on an interval $[a,b]$, we partition $[a,b]$ into subintervals and define the upper sum $U(f,P) = \sum_{i=1}^{n} M_i(x_i - x_{i-1})$ and lower sum $L(f,P) = \sum_{i=1}^{n} m_i(x_i - x_{i-1})$, where $M_i$ and $m_i$ are the supremum and infimum of $f$ on each subinterval. The upper Darboux integral is defined as $\overline{\int_a^b} f(x) dx = \inf{U(f,P) : P \text{ is a partition of } [a,b]}$, and the lower Darboux integral as $\underline{\int_a^b} f(x) dx = \sup{L(f,P) : P \text{ is a partition of } [a,b]}$. A function is Darboux integrable if these upper and lower integrals are equal, in which case this common value is the Darboux integral. This approach provides an equivalent alternative to the Riemann integral definition, with the advantage of clearly separating the approximation process from the limiting process, making certain theoretical properties more transparent.

## Wiener Integral

The Wiener integral generally refers to integration with respect to the Wiener measure, which is a probability measure on the space of continuous paths (often starting at 0, denoted $C_0[0, T]$). This concept is central to the mathematical theory of Brownian motion. It can mean calculating the expectation of a functional $F$ of a Wiener process $W$, written as $E[F(W)] = \int_{C_0[0,T]} F(\omega) dP(\omega)$, where $P$ is the Wiener measure. The term can sometimes also informally encompass stochastic integrals like the Itô or Stratonovich integrals, which integrate *with respect to* sample paths of a Wiener process, or path integrals (in the Feynman sense) where the measure is formally related to Wiener measure in Euclidean time.
