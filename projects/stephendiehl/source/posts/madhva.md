---
date: "2024-09-01"
tags: ["mathematics"]
url: "https://www.stephendiehl.com/posts/madhva/"
---

# Pi from String Amplitudes

From the [paper by Arnab Priya Saha and Aninda Sinha](https://journals.aps.org/prl/pdf/10.1103/PhysRevLett.132.221601). A new series for $\pi$ and its convergence.

$$\pi = 4 + \sum_{n=1}^{\infty} \frac{1}{n!} \left( \frac{1}{n + \lambda} - \frac{4}{2n + 1} \right) \left( \frac{(2n + 1)^2}{4(n + \lambda)} - n \right)_{n-1}$$

Where $(a)_{n-1}$ is the Pochhammer symbol.

$$(a)_{n-1} = \frac{\Gamma(a + n - 1)}{\Gamma(a)}$$

$\lambda$ is a free parameter for optimizing convergence. As $\lambda$ approaches infinity, the series reduces to the classic Madhava series. With $\lambda$ between 10 and 100, the series takes 30 terms to converge to ten decimal places. Which is far more efficient than the classic [Madhava series](https://en.wikipedia.org/wiki/Madhava_series), which takes 5 billion terms to converge to ten decimal places.

This can be implemented in Python as follows:

``` python
import mpmath as mp

def mp_calculate_pi(lambda_val, iters):
    mp.dps = 100
    pi = mp.mpf(4)
    n = 1
    prev_pi = 0
    factorial = mp.mpf(n)

    for i in range(iters):
        term1 = 1.0 / factorial
        term2 = 1.0 / (n + lambda_val) - 4 / (2 * n + 1)
        term3 = ((2 * n + 1) ** 2) / (4 * (n + lambda_val)) - n

        pochhammer = 1
        for i in range(n - 1):
            pochhammer *= term3 + i

        prev_pi = pi
        pi += term1 * term2 * pochhammer

        # print to 30 decimal places using mpmath
        approx = mp.nstr(pi, 100)

        # Compare to mp.pi to calcualte the delta
        delta = abs(pi - mp.pi)

        # numbers of digits that pi approximate is correct to
        accuracy = round(abs(mp.log(abs(pi - mp.pi), 10)))

        # Print a line with the current iteration, pi, delta
        print(f'{n} {approx} {delta} {accuracy}')

        n += 1
        factorial = factorial * n

    return pi, n - 1

lambda_val = 42
pi, n = mp_calculate_pi(lambda_val, 30)
```
