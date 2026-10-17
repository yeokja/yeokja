---
date: "2016-02-01"
tags: ["physics", "relativity", "mathematics"]
url: "https://www.stephendiehl.com/posts/taylor/"
---

# From Newton to Einstein and Back

One of my favourite proofs from undergraduate physics is the derivation of the classical kinetic energy formula from Einstein's energy-momentum relation. Specifically, we'll see how the classical expression for kinetic energy emerges as a first-degree Taylor approximation of the relativistic energy equation when velocities are much smaller than the speed of light $c$.

According to Einstein's theory of relativity, the total energy $E$ of a particle with rest mass $m_0$ is given by:

$$E = \gamma m_0 c^2,$$

where $\gamma$, the Lorentz factor, is defined as:

$$\gamma = \frac{1}{\sqrt{1 - \frac{v^2}{c^2}}}$$

where $v$ is the velocity of the particle and $c$ is the speed of light in a vacuum. The relationship simplifies significantly when velocities are much smaller than $c$.

The kinetic energy $K$ of a particle is defined as the difference between its total energy and its rest energy:

$$K = E - E_0,$$

where $E_0 = m_0 c^2$ is the rest energy of the particle. Plugging in the expression for $E$:

$$K = \gamma m_0 c^2 - m_0 c^2 = (\gamma - 1) m_0 c^2.$$

Now we need to simplify $\gamma$ for small $v$.

To obtain a first-order approximation, we can expand $\gamma$ using a Taylor series around $v = 0$. The Taylor expansion of $\gamma$ at $v = 0$ is given by:

$$\gamma(v) = \frac{1}{\sqrt{1 - \frac{v^2}{c^2}}} \approx 1 + \frac{1}{2} \frac{v^2}{c^2} + \text{higher order terms}.$$

For our purposes, since we are interested in the first-order approximation, we can ignore the $\frac{1}{2} \frac{v^2}{c^2}$ term and retain just the constant term:

$$\gamma(v) \approx 1.$$

Thus, the kinetic energy expression simplifies to:

$$K \approx \left(1 - 1\right) m_0 c^2 = 0 + \frac{1}{2} m_0 \frac{v^2}{c^2} = \frac{1}{2} m_0 v^2.$$

We can now express the kinetic energy $K$ for small velocities as:

$$K \approx \frac{1}{2} m_0 v^2$$

which is precisely the classical formula for the kinetic energy of an object. More generally, we can derive Newton's equations from Einstein's field equations, we consider the weak-field, slow-motion limit of general relativity. In the weak-field limit, we assume that spacetime is nearly flat, with only small perturbations due to gravity. The metric can be expressed as:

$$g_{\mu\nu} = \eta_{\mu\nu} + h_{\mu\nu}$$

where $\eta*{\mu\nu}$ is the Minkowski metric of flat spacetime, and $h*{\mu\nu}$ is a small perturbation. The Einstein field equations in their general form are:

$$G_{\mu\nu} + \Lambda g_{\mu\nu} = \kappa T_{\mu\nu}$$

where $G*{\mu\nu}$ is the Einstein tensor, $\Lambda$ is the cosmological constant, $g*{\mu\nu}$ is the metric tensor, $T_{\mu\nu}$ is the stress-energy tensor, and $\kappa = 8\pi G/c^4$ is the Einstein gravitational constant. To simplify these equations and recover Newton's law, we need to make several key assumptions. First, we neglect the cosmological constant $\Lambda = 0$. This is reasonable for local gravitational phenomena, as $\Lambda$ only becomes significant at cosmological scales where it accounts for the universe's accelerating expansion.

Second, we consider only static, weak gravitational fields. We mean "static" when the gravitational field doesn't change with time, and "weak" means we can treat spacetime as nearly flat with small perturbations. This is why we can write:

$$g_{\mu\nu} = \eta_{\mu\nu} + h_{\mu\nu}$$

where $|h_{\mu\nu}| \ll 1$ represents small deviations from flat spacetime. Third, we assume non-relativistic matter where $v \ll c$. This means the stress-energy tensor simplifies dramatically, with the energy density term dominating:

$$T_{00} \approx \rho c^2$$

while other components become negligible. This approximation is valid for most everyday situations on Earth and even for most astrophysical scenarios in our solar system. Under these assumptions, in the weak-field limit, the 00-component of the Ricci tensor can be approximated as:

$$R_{00} \approx \frac{1}{c^2} \nabla^2 \Phi$$

where $\Phi$ is the gravitational potential. The Ricci scalar becomes:

$$R \approx \frac{2}{c^2} \nabla^2 \Phi$$

For non-relativistic matter, the dominant component of the stress-energy tensor is $T_{00} \approx \rho c^2$, where $\rho$ is the mass density. Substituting these into the 00-component of the Einstein field equations:

$$R_{00} - \frac{1}{2}R g_{00} = \kappa T_{00}$$

we get:

$$\frac{1}{c^2} \nabla^2 \Phi - \frac{1}{c^2} \nabla^2 \Phi = \frac{8\pi G}{c^4} \rho c^2$$

Simplifying leads to Poisson's equation for Newtonian gravity:

$$\nabla^2 \Phi = 4\pi G \rho$$

For a point mass $M$ at the origin, where $\rho = M \delta(\mathbf{r})$, solving Poisson's equation in spherical coordinates gives:

$$\Phi(r) = -\frac{GM}{r}$$

The gravitational field is then:

$$\mathbf{g} = -\nabla \Phi = -\frac{GM}{r^2} \hat{r}$$

This is Newton's law of gravitation. Einstein's field equations reduce to Newton's law of gravity in the weak-field, slow-motion limit.
