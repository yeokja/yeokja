---
date: "2024-11-14"
tags: ["mathematics"]
url: "https://www.stephendiehl.com/posts/rank_29_elliptic_curve/"
---

# New Rank 29 Elliptic Curve

In August 2024, mathematicians Noam Elkies and Zev Klagsbrun [discovered](https://listserv.nodak.edu/cgi-bin/wa.exe?A2=NMBRTHRY;b9d018b1.2409&S=b) an elliptic curve with the highest known rank over rational numbers: 29. This breakthrough surpassed the previous record of rank 28 that had stood since 2006.

If we recall from [undergraduate number theory](https://www.amazon.co.uk/Elementary-Number-Springer-Undergraduate-Mathematics/dp/3540761977), an elliptic curve is a special type of cubic equation, similar to the quadratic equations you might remember from algebra (like $y = x^2 + 2x + 1$), but with one term raised to the third power. While quadratic equations are relatively straightforward to solve, elliptic curves exhibit far more complex behavior that we're still working to understand. The particular equation we're looking at has the form:

$$y^2 + xy = x^3 + ax + b$$

When we look for rational number solutions to these equations — solutions where $x$ and $y$ are rational numbers — we find something fascinating: these solutions can be grouped into families, where new solutions can be derived from existing ones through mathematical operations. The number of independent families of solutions is called the "rank" of the curve.

Think of rank as measuring how many different "starting points" you need to generate all possible rational solutions. Most elliptic curves have small ranks (0, 1, or 2), which makes this rank-29 curve exceptionally unique. The specific coefficients for this curve are:

$$a = -27006183241630922218434652145297453784768054621836357954737385$$

$$b = 55258058551342376475736699591118191821521067032535079608372404779149413277716173425636721497$$

Here's a toy Python implementation using the `sympy` library to do the arbitrary precision arithmetic needed to work with this curve:

``` python
from sympy import Symbol, solve, Rational

def E29():
    a = -27006183241630922218434652145297453784768054621836357954737385
    b = 55258058551342376475736699591118191821521067032535079608372404779149413277716173425636721497

    class EllipticCurve:
        def __init__(self):
            self.a = a
            self.b = b

        def get_y_coordinates(self, x):
            """Find y coordinates for a given x on the curve"""
            x = Rational(x)
            # Convert equation to standard form: y^2 + xy - (x^3 + ax + b) = 0
            y = Symbol('y')
            eq = y**2 + x*y - (x**3 + self.a*x + self.b)
            return solve(eq, y)

        def is_on_curve(self, x, y):
            """Check if point (x,y) lies on the curve"""
            x, y = Rational(x), Rational(y)
            return y**2 + x*y == x**3 + self.a*x + self.b

    return EllipticCurve()

# Example usage:
curve = E29()

# One of the known rational points from the discovery
x = 2891195474228537189458255536634
y_coords = curve.get_y_coordinates(x)

# Verify the point lies on the curve
if len(y_coords) > 0:
    print(f"Found point: ({x}, {y_coords[0]})")
    print(f"Verifying point lies on curve: {curve.is_on_curve(x, y_coords[0])}")
```

So why does this matter? This discovery is significant because no one knows if there's an upper limit to how large the rank of an elliptic curve can be. Finding such a limit (or proving there isn't one) remains one of the major open problems in mathematics. Each new record, like this rank-29 curve, provides some new insights into whether an upper bound exists.

Proving the existence of an upper bound for the rank of elliptic curves would be a monumental achievement worthy of a Fields Medal for several reasons. It would resolve a fundamental result related to the Birch and Swinnerton-Dyer conjecture, one of the seven Millennium Prize Problems. It would also provide deep insights into the mysterious relationship between the algebraic and analytic properties of L-functions, a connection that lies at the core of modern number theory. The techniques required to prove such a bound would likely revolutionize our understanding of arithmetic geometry and potentially unlock new approaches to other major open problems in number theory.
