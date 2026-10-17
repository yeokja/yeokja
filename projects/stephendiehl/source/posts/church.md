---
date: "2017-08-10"
tags: ["python", "lambda calculus"]
url: "https://www.stephendiehl.com/posts/church/"
---

# The Y-Combinator in Python

Lambda calculus holds a revered place as a foundational system in computation theory, influencing programming languages and paradigms. One fascinating application of lambda calculus is Church encoding, which allows representing data and operators using only functions. This blog post delves into how Church encoding can be implemented in Python to represent boolean values, basic arithmetic operations, and even complex functions like the Fibonacci sequence.

Church encoding is a means of representing data and operators in lambda calculus, which uses functions instead of data types. Typical constructs such as numerals, boolean values, and basic arithmetic operations can be represented purely through functional abstractions and applications.

Let's start by encoding boolean values and essential logical operations using lambda functions in Python.

In Church encoding, TRUE and FALSE can be defined as functions taking two parameters, returning the first and the second parameter respectively:

``` python
TRUE = lambda x: lambda y: x
FALSE = lambda x: lambda y: y
```

Using these definitions, we can define logical operations. For instance, the AND operation can be represented as:

``` python
And = lambda x: lambda y: x(y)(x)
```

Arithmetic operations can also be represented in this system. For example, the successor of a number, addition, and multiplication are defined as follows:

``` python
Succ = lambda n: lambda f: lambda x: f(n(f)(x))
Add = lambda m: lambda n: lambda f: lambda x: m(f)(n(f)(x))
Mul = lambda m: lambda n: lambda f: m(n(f))
```

Now, for a more complex example, we use the Fibonacci sequence, implementing it through lambda calculus constructs in Python. We employ the Y combinator to handle recursion since lambda calculus inherently does not support recursion:

``` python
Y = lambda f: (lambda x: f(lambda v: x(x)(v)))(lambda x: f(lambda v: x(x)(v)))
Fib = Y(lambda f: lambda n: Add(f(Pred(n)))(f(Sub(n)(TWO))) if Gt(n)(ONE) else n)
```

Implementing and running this encoded logic in Python serves as an excellent tool for understanding the deeper foundations of computation. Let’s see how it evaluates some Fibonacci positions:

``` python
print(Fib(1))  # Output: 1
print(Fib(2))  # Output: 1
print(Fib(8))  # Output: 21
```

The whole code provided for your twisted amusement is listed below:

``` python
TRUE = lambda x: lambda y: x
FALSE = lambda x: lambda y: y
Product = lambda x: lambda y: lambda z: (z)(x)(y)
And = lambda x: lambda y: (x)(y)(x)
Not = lambda x: lambda y: lambda z: (x)(z)(y)
Boolean = (Product)(True)(False)
Succ = lambda x: lambda y: lambda z: (y)((x)(y)(z))
Pred = lambda x: lambda y: lambda z: x(lambda i: lambda j: j(i(y))) (lambda k: z) (lambda k: k)
Y = lambda g: (lambda z:z(z)) (lambda f: g(lambda arg: f(f)(arg)))
Nth = Y(lambda f: lambda n: Succ(f(n-1)) if n else Zero)
Dec = lambda a: (a)(lambda b: b + 1)(0)
Add = lambda x: lambda y: lambda z: lambda w: (x)(z)((y)(z)(w))
Sub = lambda x: lambda y: (y)(Pred)(x)
Mul = lambda x: lambda y: lambda z: (y)((x)(z))
Null = lambda x: x(lambda y: Zero)(TRUE)
Eq = lambda x: lambda y: (And)((Null)((Sub)(y)(x)))((Null)((Sub)(x)(y)))
Gt = lambda x: lambda y: (And)((Null)((Sub)(y)(x)))((Not)((Eq)(y)(x)))
Lt = lambda x: lambda y: (And)((Not)((Gt)(x)(y)))((Not)((Eq)(x)(y)))
IfThenElse = lambda a: lambda b: lambda c: ((a)(b))(c)
Range = lambda a,b: map(Nth,range(a,b))

# Minimal
Fib = Y(lambda f: lambda n: Add(f(Pred(n)))(f((Sub)(n)(Two))) if (Boolean)((Gt)(n)(One)) else n)

# Exapnded
Fib = lambda x: (lambda a: (a)(lambda b: b + 1)(0))(
    (
        (
            lambda g: (lambda f: g(lambda arg: f(f)(arg)))(
                lambda f: g(lambda arg: f(f)(arg))
            )
        )(
            lambda f: lambda n: (
                (lambda x: lambda y: lambda z: lambda w: (x)(z)((y)(z)(w)))(
                    f(
                        (
                            lambda x: lambda y: lambda z: x(
                                lambda i: lambda j: j(i(y))
                            )(lambda k: z)(lambda k: k)
                        )(n)
                    )
                )(
                    f(
                        (
                            (
                                lambda x: lambda y: (y)(
                                    (
                                        lambda x: lambda y: lambda z: x(
                                            lambda i: lambda j: j(i(y))
                                        )(lambda k: z)(lambda k: k)
                                    )
                                )(x)
                            )
                        )(n)(
                            (
                                (lambda x: lambda y: lambda z: (y)((x)(y)(z)))(
                                    (
                                        (lambda x: lambda y: lambda z: (y)((x)(y)(z)))(
                                            (lambda x: lambda y: y)
                                        )
                                    )
                                )
                            )
                        )
                    )
                )
                if (
                    (
                        ((lambda x: lambda y: lambda z: (z)(x)(y)))(
                            lambda x: lambda y: x
                        )(False)
                    )
                )(
                    (
                        (
                            lambda x: lambda y: (lambda x: lambda y: (x)(y)(x))(
                                (
                                    (
                                        lambda x: x(lambda y: (lambda x: lambda y: y))(
                                            lambda x: lambda y: x
                                        )
                                    )
                                )(
                                    (
                                        (
                                            lambda x: lambda y: (y)(
                                                (
                                                    lambda x: lambda y: lambda z: x(
                                                        lambda i: lambda j: j(i(y))
                                                    )(lambda k: z)(lambda k: k)
                                                )
                                            )(x)
                                        )
                                    )(y)(x)
                                )
                            )(
                                ((lambda x: lambda y: lambda z: (x)(z)(y)))(
                                    (
                                        (
                                            lambda x: lambda y: (
                                                lambda x: lambda y: (x)(y)(x)
                                            )(
                                                (
                                                    (
                                                        lambda x: x(
                                                            lambda y: (
                                                                lambda x: lambda y: y
                                                            )
                                                        )(lambda x: lambda y: x)
                                                    )
                                                )(
                                                    (
                                                        (
                                                            lambda x: lambda y: (y)(
                                                                (
                                                                    lambda x: lambda y: lambda z: x(
                                                                        lambda i: lambda j: j(
                                                                            i(y)
                                                                        )
                                                                    )(
                                                                        lambda k: z
                                                                    )(
                                                                        lambda k: k
                                                                    )
                                                                )
                                                            )(x)
                                                        )
                                                    )(y)(x)
                                                )
                                            )(
                                                (
                                                    (
                                                        lambda x: x(
                                                            lambda y: (
                                                                lambda x: lambda y: y
                                                            )
                                                        )(lambda x: lambda y: x)
                                                    )
                                                )(
                                                    (
                                                        (
                                                            lambda x: lambda y: (y)(
                                                                (
                                                                    lambda x: lambda y: lambda z: x(
                                                                        lambda i: lambda j: j(
                                                                            i(y)
                                                                        )
                                                                    )(
                                                                        lambda k: z
                                                                    )(
                                                                        lambda k: k
                                                                    )
                                                                )
                                                            )(x)
                                                        )
                                                    )(x)(y)
                                                )
                                            )
                                        )
                                    )(y)(x)
                                )
                            )
                        )
                    )(n)(
                        (
                            (lambda x: lambda y: lambda z: (y)((x)(y)(z)))(
                                (lambda x: lambda y: y)
                            )
                        )
                    )
                )
                else n
            )
        )
    )(
        (
            (
                lambda g: (lambda f: g(lambda arg: f(f)(arg)))(
                    lambda f: g(lambda arg: f(f)(arg))
                )
            )(
                lambda f: lambda n: (
                    (lambda x: lambda y: lambda z: (y)((x)(y)(z)))(f(n - 1))
                    if n
                    else (lambda x: lambda y: y)
                )
            )
        )(x)
    )
)
```
