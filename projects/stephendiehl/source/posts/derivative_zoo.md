---
date: "2018-05-01"
tags: ["mathematics"]
url: "https://www.stephendiehl.com/posts/derivative_zoo/"
---

# Derivative Zoo

There are many types of operators that we call derivatives.

1.  [Classical Derivative](#classical-derivative)
2.  [One-Sided Derivative](#one-sided-derivative)
3.  [Higher Order Derivative](#higher-order-derivative)
4.  [Implicit Derivative](#implicit-derivative)
5.  [Complex Derivative](#complex-derivative)
6.  [Partial Derivative](#partial-derivative)
7.  [Directional Derivative](#directional-derivative)
8.  [Covariant Derivative](#covariant-derivative)
9.  [Lie Derivative](#lie-derivative)
10. [Exterior Derivative](#exterior-derivative)
11. [Material Derivative](#material-derivative)
12. [Weak Derivative](#weak-derivative)
13. [Fréchet Derivative](#frechet-derivative)
14. [Gâteaux Derivative](#gateaux-derivative)
15. [Variational Derivative](#variational-derivative)
16. [Fractional Derivative](#fractional-derivative)
17. [Radon-Nikodym Derivative](#radon-nikodym-derivative)
18. [Stochastic Derivative](#stochastic-derivative)

## Classical Derivative

The classical derivative of a function $f: \mathbb{R} \to \mathbb{R}$ at a point $x$ is defined by the limit $f'(x) = \frac{df}{dx} = \lim_{h \to 0} \frac{f(x+h) - f(x)}{h}$. This value represents the instantaneous rate of change of the function $f$ with respect to its variable $x$, and geometrically corresponds to the slope of the tangent line to the graph of $f$ at $(x, f(x))$. Its derivation stems from considering the average rate of change over a shrinking interval $[x, x+h]$, represented by the slope of the secant line $\frac{f(x+h) - f(x)}{h}$, and taking the limit as the interval width $h$ approaches zero.

## One-Sided Derivative

One-sided derivatives consider the rate of change as $x$ is approached from only one direction. The right derivative is $f'_+(x) = \lim_{h \to 0^+} \frac{f(x+h) - f(x)}{h}$ (where $h$ approaches 0 from positive values), and the left derivative is $f'_-(x) = \lim_{h \to 0^-} \frac{f(x+h) - f(x)}{h}$ (where $h$ approaches 0 from negative values). These are derived using the same limit definition as the classical derivative, but restricting $h$ to be either strictly positive or strictly negative. The classical derivative exists if and only if both the left and right derivatives exist and are equal.

## Higher Order Derivative

Higher order derivatives are obtained by repeatedly applying the differentiation process. The $n$-th derivative, denoted $f^{(n)}(x)$ or $\frac{d^n f}{dx^n}$, is defined recursively as the derivative of the $(n-1)$-th derivative: $f^{(n)}(x) = \frac{d}{dx} \left( f^{(n-1)}(x) \right)$. For instance, the second derivative $f''(x) = \frac{d^2 f}{dx^2} = \lim_{h \to 0} \frac{f'(x+h) - f'(x)}{h}$ measures the rate of change of the first derivative (like acceleration) and relates to the function's concavity. Each higher order derivative measures the rate of change of the preceding one.

## Implicit Derivative

Implicit differentiation is a technique, rather than a specific formula, used when a relationship between $x$ and $y$ is given implicitly by an equation $F(x, y) = 0$, defining $y$ as a function of $x$ locally. To find $\frac{dy}{dx}$, one differentiates both sides of the equation $F(x, y(x)) = 0$ with respect to $x$, treating $y$ as $y(x)$ and applying the chain rule where necessary (e.g., $\frac{d}{dx}(y^2) = 2y \frac{dy}{dx}$). The resulting equation is then algebraically solved for $\frac{dy}{dx}$. This allows finding the derivative without explicitly solving for $y$.

## Complex Derivative

The derivative of a complex function $f: \mathbb{C} \to \mathbb{C}$ at $z_0 \in \mathbb{C}$ is defined similarly to the real case: $f'(z_0) = \lim_{h \to 0} \frac{f(z_0+h) - f(z_0)}{h}$. However, here $h \in \mathbb{C}$, and the limit must exist and yield the same value regardless of the path along which $h$ approaches 0 in the complex plane. This path-independence requirement is very strong, distinguishing complex differentiability (holomorphicity) from real differentiability in $\mathbb{R}^2$ and implying the function satisfies the Cauchy-Riemann equations. It represents the complex rate of change.

## Partial Derivative

For a multivariable function $f(x_1, x_2, ..., x_n)$, the partial derivative with respect to one variable $x_i$ at a point $\mathbf{a}$, denoted $\frac{\partial f}{\partial x_i}(\mathbf{a})$, measures the function's rate of change along the $x_i$ coordinate direction while holding all other variables constant. Its formula is $\frac{\partial f}{\partial x_i}(\mathbf{a}) = \lim_{h \to 0} \frac{f(a_1, ..., a_i+h, ..., a_n) - f(a_1, ..., a_i, ..., a_n)}{h}$. It is derived by applying the standard single-variable derivative definition with respect to $x_i$, treating all other $x_j$ (for $j \neq i$) as fixed parameters.

## Directional Derivative

The directional derivative generalizes the partial derivative, measuring the rate of change of a function $f: \mathbb{R}^n \to \mathbb{R}$ at a point $\mathbf{a}$ along an arbitrary direction specified by a unit vector $\mathbf{u}$. It is defined as $D_{\mathbf{u}}f(\mathbf{a}) = \lim_{h \to 0} \frac{f(\mathbf{a} + h\mathbf{u}) - f(\mathbf{a})}{h}$. If $f$ is differentiable at $\mathbf{a}$, this can be computed using the gradient $\nabla f = (\frac{\partial f}{\partial x_1}, ..., \frac{\partial f}{\partial x_n})$ via the dot product: $D_{\mathbf{u}}f(\mathbf{a}) = \nabla f(\mathbf{a}) \cdot \mathbf{u}$. The definition applies the limit concept along the line $\mathbf{a} + t\mathbf{u}$, while the gradient formula derives from the chain rule applied to $g(t) = f(\mathbf{a} + t\mathbf{u})$.

## Covariant Derivative

The covariant derivative, $\nabla_X T$, generalizes the directional derivative to tensor fields $T$ on curved spaces (manifolds) equipped with a connection $\nabla$. It measures the change of $T$ along a vector field $X$, accounting for the variation of the basis vectors or coordinate system. In local coordinates $x^i$ with Christoffel symbols $\Gamma^k_{ij}$, the covariant derivative of a vector field $V = V^k \frac{\partial}{\partial x^k}$ along $X = X^j \frac{\partial}{\partial x^j}$ has components $(\nabla_X V)^k = X^j (\partial_j V^k + \Gamma^k_{ij} V^i)$, often written in component form as $\nabla_j V^k = \partial_j V^k + \Gamma^k_{ij} V^i$. It is derived axiomatically or by demanding it measure the intrinsic change of the tensor's components plus the change induced by the varying basis, yielding a tensor of the same type.

## Lie Derivative

The Lie derivative, $\mathcal{L}_X T$, measures the rate of change of a tensor field $T$ as it is "dragged" along the flow $\phi_t$ generated by a vector field $X$. It is defined as $\mathcal{L}_X T = \lim_{t \to 0} \frac{\phi_{-t}^* T - T}{t}$, where $\phi_{-t}^* T$ denotes the pullback of $T$ along the flow. Unlike the covariant derivative, it doesn't require a metric or connection. For a scalar function $f$, it reduces to the directional derivative $\mathcal{L}_X f = X(f)$. For a vector field $Y$, it equals the Lie bracket $\mathcal{L}_X Y = [X, Y] = XY - YX$. Its derivation involves comparing the tensor at a point $p$ with the tensor at $\phi_t(p)$ appropriately transported back to $p$.

## Exterior Derivative

The exterior derivative $d$ is an operation mapping differential $k$-forms $\omega$ to $(k+1)$-forms $d\omega$ on a manifold. For a 0-form (function) $f$, $df = \frac{\partial f}{\partial x^i} dx^i$. For a $k$-form $\omega = \sum_{I} a_I dx^I$, $d\omega = \sum_{I} da_I \wedge dx^I$. It uniquely satisfies properties like linearity, the graded Leibniz rule, $d(d\omega) = 0$ ($d^2=0$), and consistency with the gradient. It provides a coordinate-independent generalization of grad, curl, and div, and is fundamental to the generalized Stokes' theorem: $\int_M d\omega = \int_{\partial M} \omega$. Its coordinate formula can be derived from these axioms.

## Material Derivative

The material derivative (or substantial derivative, total derivative), denoted $\frac{D\phi}{Dt}$, measures the total rate of change of a field $\phi(\mathbf{x}, t)$ (like temperature or density) experienced by a particle moving with a velocity field $\mathbf{u}(\mathbf{x}, t)$. Its formula is $\frac{D\phi}{Dt} = \frac{\partial \phi}{\partial t} + (\mathbf{u} \cdot \nabla) \phi$. It combines the local rate of change at a fixed point ($\partial \phi / \partial t$) with the convective rate of change due to the particle's movement into regions with different $\phi$ values ($\mathbf{u} \cdot \nabla \phi$). It is derived by applying the multivariable chain rule to $\phi(\mathbf{x}(t), t)$ where $\frac{d\mathbf{x}}{dt} = \mathbf{u}$.

## Weak Derivative

The weak derivative extends differentiation to functions (like those in Sobolev spaces) which may not be smooth enough for classical differentiation. A function $v \in L^1_{loc}(\Omega)$ is the $\alpha$-th weak derivative of $u \in L^1_{loc}(\Omega)$ (denoted $D^\alpha u = v$) if $\int_{\Omega} u(x) (-1)^{|\alpha|} D^\alpha \phi(x) dx = \int_{\Omega} v(x) \phi(x) dx$ holds for all smooth test functions $\phi$ with compact support in $\Omega$. This definition is motivated by the integration by parts formula, effectively transferring the derivative operation from the potentially non-smooth function $u$ onto the infinitely differentiable test function $\phi$.

## Fréchet Derivative

The Fréchet derivative generalizes the concept of the derivative to functions $f: U \to W$ between Banach spaces $V$ and $W$ ($U \subseteq V$ open). It is a bounded linear operator $A: V \to W$, denoted $Df(x)$, that provides the best linear approximation of $f$ near $x$. This means $\lim_{|h|_V \to 0} \frac{|f(x+h) - f(x) - A(h)|_W}{|h|_V} = 0$, or equivalently, $f(x+h) = f(x) + A(h) + o(|h|_V)$. The definition formalizes the idea $f(x+h) \approx f(x) + f'(x)h$ where $f'(x)h$ is replaced by the action of the linear map $A$ on the displacement $h$. If it exists, it's unique.

## Gâteaux Derivative

The Gâteaux derivative is another generalization of the derivative for functions $f: U \to W$ between topological vector spaces, focusing on directional rates of change. The Gâteaux derivative of $f$ at $x \in U$ in the direction $v \in V$ is $Df(x; v) = \lim_{t \to 0} \frac{f(x+tv) - f(x)}{t}$, provided the limit exists (where $t$ is a scalar). It directly generalizes the concept of the directional derivative in $\mathbb{R}^n$. It is a weaker notion than the Fréchet derivative; Fréchet differentiability implies Gâteaux differentiability along all directions, but the converse is not always true.

## Variational Derivative (Functional Derivative)

The variational derivative, $\frac{\delta F}{\delta f(x)}$, measures how a functional $F[f]$ (which maps functions to scalars) changes in response to an infinitesimal, localized variation $\phi(x)$ in its input function $f$ at point $x$. It is defined implicitly via the first variation: $\delta F = \int \frac{\delta F}{\delta f(x)} \phi(x) dx$, where $\delta F$ is the linear part (in $\phi$) of $F[f + \epsilon \phi] - F[f]$ as $\epsilon \to 0$. For functionals defined by an integral of a Lagrangian $L(x, f, f', ...)$, it is often computed using the Euler-Lagrange expression: $\frac{\delta F}{\delta f(x)} = \frac{\partial L}{\partial f} - \frac{d}{dx} \frac{\partial L}{\partial f'} + \dots$. It serves as the functional analogue of the gradient.

## Fractional Derivative

Fractional derivatives generalize the concept of differentiation to non-integer orders $\alpha$. There isn't a single universal definition, but common forms include the Riemann-Liouville derivative $_a D_t^\alpha f(t) = \frac{1}{\Gamma(n-\alpha)} \frac{d^n}{dt^n} \int_a^t (t-\tau)^{n-\alpha-1} f(\tau) d\tau$ and the Caputo derivative $^C_a D_t^\alpha f(t) = \frac{1}{\Gamma(n-\alpha)} \int_a^t (t-\tau)^{n-\alpha-1} f^{(n)}(\tau) d\tau$, where $n = \lceil \alpha \rceil$. These operators extend differentiation and integration to arbitrary real or complex orders. They are typically non-local (depending on the function's history) and find use in modeling phenomena with memory effects or fractal characteristics. Their derivation often involves generalizing properties of integer-order calculus, such as Cauchy's formula for repeated integration or Fourier transform properties.

## Radon-Nikodym Derivative

The Radon-Nikodym derivative $f = \frac{d\nu}{d\mu}$ relates two $\sigma$-finite measures $\mu$ and $\nu$ on a measurable space $(X, \Sigma)$. Provided that $\nu$ is absolutely continuous with respect to $\mu$ ($\nu \ll \mu$, meaning $\mu(A)=0 \implies \nu(A)=0$), the Radon-Nikodym theorem guarantees the existence of a non-negative measurable function $f$ such that $\nu(A) = \int_A f d\mu$ for all measurable sets $A$. This function $f$ acts as the density or rate of change of $\nu$ relative to $\mu$. If $\mu$ is Lebesgue measure, it generalizes concepts like probability density functions. Its existence is a deep result from measure theory, not derived from a simple formula but proven using functional analysis techniques.

## Stochastic Derivative

The term "stochastic derivative" often refers to the Malliavin derivative, used in Malliavin calculus for functionals of stochastic processes like Brownian motion ${W_t}$. The Malliavin derivative $D_t F$ of a random variable $F$ measures its sensitivity to infinitesimal changes in the path of the driving noise at time $t$. For a simple functional $F = f(W_{t_1}, ..., W_{t_n})$, $D_t F = \sum_{i=1}^n \frac{\partial f}{\partial x_i}(W_{t_1}, ..., W_{t_n}) \mathbf{1}_{[0, t_i]}(t)$. It is formally defined via an integration-by-parts formula on Wiener space, making the derivative operator $D$ the adjoint of the Skorokhod stochastic integral $\delta$. It plays a key role in stochastic analysis and quantitative finance.
