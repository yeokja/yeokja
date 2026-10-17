---
date: "2016-03-08"
tags: ["fp"]
url: "https://www.stephendiehl.com/posts/exotic_02/"
---

# Exotic Programming Ideas: Part 2 (Term Rewriting)

Continuing on from our series last week, this time we’re going to discuss the topic of term rewriting. Term rewriting is a very general model of computation that consists of programmatic systems for describing transformations (called rules) which transform expressions called terms into other sets of terms.

- [Week 1 - Module Systems](exotic_01.md)
- [Week 2 - Term Rewriting](exotic_02.md)
- [Week 3 - Effect Systems](exotic_03.md)
- [Week 4 - Datalog](exotic_04.md)

The most widely used rewriting engines often sit at the heart of programs and languages used for logic programming, proof assistants and computer algebra systems. One of the most popular of these is Mathematica and the Wolfram language. This is a proprietary programming language that is offered by the company Wolfram Research as part of a framework for scientific computing. At the core of this system is a rewrite system that can manipulate composite symbolic expressions that represent mathematical expressisons. Other languages such as Maude, Pure, Aldor, and some Lisp variants implement similar systems but for the purpose of our examples we will use code written in Mathematica to demonstrate the more general concept.

Mathematica is normally run in a notebook environment, where the In and Out expressions are vectors indexed by a monotonically increasing index for each line entered at the REPL.

``` mathematica
Wolfram Language 12.0.1 Engine for Linux x86 (64-bit)
Copyright 1988-2019 Wolfram Research, Inc.

In[1]:= 1+2
Out[1]= 3

In[2]:= 1+2+x
Out[2]= 3 + x

In[3]:= List[x,y,z]
Out[3]= {x, y, z}
```

The language itself is spiritually a Lisp that operates on expressions, which can include symbolic terms (such as the x term above). These expressions are not variables but irreducible atoms that stand for variables in expressions. Composite expressions are similar to Lisp’s S-expressions and consist of a head term and a set of nested subexpressions which are arguments to the expressions.

``` mathematica
F[x,y]
```

In this case F is the head and x and y are arguments. This form of notation is known as an M-expression. The key notation is that all language constructs are themselves symbolic expressions and can be introspected and transformed by rules. For example the Head function extracts the head symbol from a given expression.

``` mathematica
In[4]:= Head[f[a,b,c]]
Out[4]= f

In[5]:= Symbol["x"]
Out[5]= x

In[6]:= Head[x]
Out[6]= Symbol
```

The arguments for a given M-expression can be extracted using the Level function which extracts the arguments of the given depth as a vector (denoted by braces). In this case the first level is the arguments a, b and c.

``` mathematica
In[7]:= Level[f[a,b,c],1]
Out[7]= {a, b, c}
```

Expressions can be written in multiple forms which reduce down to this base M-expression form. For example the addition operation is simply syntactic sugar for an expression with a Plus head. We can reduce this expression to its internal canonical form using the FullForm function. Multiplication is also denoted in infix by separating individual terms by spaces similar to convention used common algebra.

``` mathematica
In[8]:= FullForm[x+y]
Out[8]//FullForm= Plus[x, y]

In[9]:= FullForm[x y]
Out[9]//FullForm= Times[x, y]
```

In addition to infix syntactic sugar the two operators @ and // can be used to apply functions to arguments in either prefix or postfix form respectively.

``` mathematica
In[10]:= Sin[x] (* Full form *)
Out[10]= Sin[x]

In[11]:= Sin @ x (* Prefix sugar *)
Out[11]= Sin[x]

In[12]:= x // Sin (* Postfix sugar *)
Out[12]= Sin[x]
```

Complex mathematical expressions are simply nested combinations of terms. For example a simple monomial expression $x^3+(1+x)^2$

would be given by:

``` mathematica
In[13]:= FullForm[x^3 + (1 + x)^2]
Out[13]//FullForm= Plus[Power[x, 3], Power[Plus[1, x], 2]]

In[14]:= TeXForm[x^3 + (1 + x)^2]
Out[14]//TeXForm= x^3+(x+1)^2
```

This expression itself is a graph structure, and it’s implementation consists of a set of nodes with pointers to its subexpressions. The index notation using brackets can be used to directly manipulate a given subexpression which are enumerated left to right, starting at the head term.

``` mathematica
In[15]:= f[a,b,c][[0]]
Out[15]= f

In[16]:= f[a,b,c][[1]]
Out[16]= a

In[17]:= f[a,b,c][[2]]
Out[17]= b
```

Individual arguments to expressions can be reordered and substituted into other expressions be referencing their slots by index.

``` mathematica
In[18]:= F[#3, #1, #2] & [x, y, z]
Out[18]= F[z, x, y]
```

Complex mathematical expressions can be expressed in terms of graphs of expressions and can be rendered in a graphical tree form. Effectively all rules and functions in this system are transformations of directed graphs to other directed graph structures. For example a simple trigonometric series can be expressed symbolically as:

``` mathematica
In[19]:= Series[Cos[x]/x, {x, 0, 10}]
```

This has the internal representation as this graph structure of M-expressions.

``` mathematica
In[20]:= FullForm[Series[Cos[x]/x, {x, 0, 10}]]
Out[20]//FullForm= SeriesData[x, 0, {1, 0, Rational[-1, 2], 0, Rational[1, 24], 0, Rational[-1, 720], 0, Rational[1, 40320], 0, Rational[-1, 3628800], 0, Rational[1, 39916800]}, 0, 11, 1]
```

This symbolic expression can be expanded into individual terms by evaluating the Series term into individual terms.

$$\sum_{x=0}^{10} \frac{\cos x}{x} = \frac{1}{x} - \frac{x}{2} + \frac{x^3}{24} - \frac{x^5}{720} + \frac{x^7}{40320} - \frac{x^9}{3628800} + O(x^{11})$$

## Evaluation Order

Evaluation in the language proceeds by reduction of terms until there are no more rules that apply to the reduced expression. Such an expression is said to be in normal form. This mode of evaluation differs from sequential, statement-based languages where sequential updates to memory and the sequencing of effectful statements is the mode of execution.

For example the Range function produces a series of values as a list. This list can itself be passed to another function which applies a function over each element, such as the querying the primality of a given integer.

``` mathematica
In[20]:= Range[1,10]
Out[20]= {1, 2, 3, 4, 5, 6, 7, 8, 9, 10}

In[21]:= Map[PrimeQ, Range[1,10]]
Out[21]= {False, True, True, False, True, False, True, False, False, False}
```

This sequence of booleans is the final form, no subsequent rules in scope can be applied and the final expression is returned.

## Rules

The primary mechanism of introducing new user-defined logic to the environment is the introduction of rewrite rules. These are defined by either a Rule definition or it’s infix sugar -\>. The left-hand side is given in pattern syntax and may bind variable definitions over the right-hand side.

For example the rule that writes all occurrences of the integer 1 to 2.

``` mathematica
In[22]:= Rule[1,2]
Out[22]= 1 -> 2
```

Or the rule that transforms each explicit occurance of the symbol x into the symbol y.

``` mathematica
In[23]:= Rule[x,y]
Out[23]= x -> y
```

Pattern variables can also be introduced which match on subexpression and name them in the scope of the rule. The resulting match on this “hole” is bbound as a variable name over the right-hand side of the rule. For example the rule that matches any quantity x\_ and binds this to the definition x.

``` mathematica
In[24]:= Rule[x_,x]
Out[24]= x_ -> x
```

More usefully these pattern variables can occur within expressions. So for instance if we wanted to rewrite any argument given to the sine function as a cosine function, we could write the following rule:

``` mathematica
In[25]:= Sin[x_] -> Cos[x]
Out[25]= Sin[x_] -> Cos[x]
```

Rules can be applied to a given expression using either ReplaceAll function and it’s infix form /. applies a given rule to the expression on the left.

``` mathematica
In[26]:= ReplaceAll[x, Rule[x,y]]
Out[26]= y

In[27]:= x /. Rule[x,y]
Out[27]= y
```

In our trigonometric example from above, we can rewrite nested trigonometric functions from within their expression body.

``` mathematica
In[28]:= Sin[a+b] /. Sin[x_] -> Cos[x]
Out[28]= Cos[a + b]
```

Rules can be combined by passing a list of rules as an argument to ReplaceAll. For example the following does a single pass over the given list and rewrites all x to y and all y to z.

``` mathematica
In[29]:= {x,y,z} /. {x->y, y->z}
Out[29]= {y, z, z}
```

Since the rules are applied in order, to the given intermediate expression of the last rule, this does not transform the entire list into the same element. However the function ReplaceRepeated will apply a given set of rules repeatedly until a fix point is reached and the term is identical the last set reduction.

``` mathematica
In[30]:= ReplaceRepeated[x, Rule[x,y]]
Out[30]= y

In[31]:= ReplaceRepeated[x, {Rule[x,y], Rule[y,z]}]
Out[31]= z

In[32]:= {x,y,z} /. {x->y, y->z}  (* Non-repeated *)
Out[32]= {y, z, z}

In[33]:= {x,y,z} //. {x->y, y->z}  (* Repeated *)
Out[33]= {z, z, z}
```

It is quite possible to define a set of rules for which this process simply repeats ad infinitum and does not converge. This has no normal form either blows up in time (or space) or until the runtime decides to give up.

``` mathematica
In[34]:= {1,2} //. {1 -> 2, 2 -> 1}
ReplaceRepeated::rrlim: Exiting after {1, 2} scanned 65536 times.
```

## Algebraic Transformations

Many sets of rules admit multiple rewriting paths that do not necessarily result in the same final term. A system of rules in which the ordering of the rewrites are chosen does not make a difference to the eventual result is called confluent. The prescription on evaluation order used inside the rewrite rules inside of the runtime. This is arbitrary choice, however Mathematica evaluates the head of an expression and then evaluates each element of the expression. These elements are in themselves expressions and the same evaluation procedure is recursively applied.

Within the standard library there are many sets of these transformations for working with algebraic simplifications and term reordering. The function FullSimplify is the most general and will collect and simplify real-valued algebraic expressions according to a enormous corpus of idenities and relations it has defined internally.

``` mathematica
In[35]:= FullSimplify[x^2 x + y x + z + x y]
Out[35]= Plus[Power[x, 3], Times[2, x, y], z]
```

However we can introduce and extend these rule sets ourselves. For example if we wanted to encode a new simple set of rules for simplifying trigonometric functions according to given identity we could write a rule for the following math identity.

$$\sin(\alpha + \beta) = \sin \alpha \cos \beta + \cos \alpha \sin \beta$$

$$\cos(\alpha + \beta) = \cos \alpha \cos \beta - \sin \alpha \sin \beta$$

This would be encoded as the following two pattern and rewrites:

``` mathematica
In[36]:= ElemTrigExpand = {
  Sin[a_ + b_] -> Sin[a] Cos[b] + Cos[a] Sin[b],
  Cos[a_ + b_] -> Cos[a] Cos[b] - Sin[a] Sin[b]
}
```

This could then we applied to trigonometric functions of linear arguments repeatedly to reduce the nested terms to a single variable.

``` mathematica
In[37]:= Sin[a+b+c] //. ElemTrigExpand
Out[37]= Cos[a] (Cos[c] Sin[b] + Cos[b] Sin[c]) Sin[a] (Cos[b] Cos[c] + Sin[b] Sin[c])
```

## Attributes & Properties

Terms that occur within rules can also have additional structure which rules can dispatch on. Symbols can have defined Attributes which can inform reductions and reorderings of symbols into canonical forms according. Often those common algebraic properties like associativity, commutativity, identities and annihilation.

``` mathematica
In[38]:= Attributes[f] = {Flat, OneIdentity}
Out[38]= {Flat, OneIdentity}

In[39]:= f[1, f[2, f[3, 4]]]  (* Associative *)
Out[39]= f[1, 2, 3, 4]

In[40]:= f[f[1]]  (* Idempotent *)
Out[40]= f[1]
```

In addition to structural properties of functions, symbols can be enriched with assumptions about their origin and the domain or set from which they are defined. For exmaple whether a given number is nonzero, integral, prime, or a member of a given set. These properties are themselves symbolic expressions, so for example if we to encode the assumption that a given variable z is in the complex plane:

$$z \in \mathbb{C}$$

We could write the following expression, where Complexes is a given builtin standing for $\mathbb{C}$.

These can be added to either the global assumption set which is defined by the variable `Assumptions`.

``` mathematica
In[42]:= $Assumptions = {a > 0, b > 1}
Out[42]= a > 0
```

Rewrite such algebraic simplifications will dispatch on the assumptions of terms encountered in redexes and take different reduction paths based on chains of deductions about the properties of symbols. For example the identity that the square root of a square reduces to identity dependencies on the sign of the given symbol. Using the Refine function we can incorporate the global assumptions to tell the simplifier which branch to choose.

``` mathematica
In[43]:= Refine[Sqrt[a^2 b^2]]
Out[43]= a b
```

The truth value of a predicate over a symbol can be queried using this function as well. This either results in a True, False or indeterminate answer when the symbol has no local assumptions over it.

``` mathematica
In[44]:= Refine[Positive[b]]
Out[44]= True

In[45]:= Refine[Positive[x]]
Out[45]= Positive[x] (* Has no truth value *)
```

Many functions also take a named Assumptions parameter which can be used to introduce local assumptions over the arguments given a to a function.

``` mathematica
In[46]:= Refine[Sqrt[x^2], Assumptions -> {x < 0}]
Out[46]= -x
```

The assumption is capable of doing limited first order resolution in order to deduce properties of symbols within certain domains. For example the assumption x \> 1 will imply the fact that x is non-zero and not negative. Or the assumption than an integer is y \> 2 and prime implies that y is not even.

Patterns on the left hand side of rules can also dispatch both on the type of an expression and on assumptions and attributes of pattern variables. For example the LHS x_Symbol matches any symbolic quantity (but not numbers) can rewrites it to a given constant. Note that this matches on the head of the List itself and rewrite the head to a constant.

``` mathematica
In[47]:= {1, x, Pi} /. x_Symbol -> 0 (* Matches List, x, Pi *)
Out[47]= 0[1, 0, 0]
```

Predicate functions can be wrapped around pattern variables by passing a \_?s suffix followed by the function. For example to check whether a given term is a numerical quantity we can use the NumericQ function. This will match on the Pi and 1 terms but not the List or 1 terms.

``` mathematica
In[48]:= {1, x, Pi} /. x_?NumericQ -> 0 (* Matches 1, Pi *)
Out[48]= {0, x, 0}
```

## Special Functions

Within the standard library there is a vast corpus of mathematical knowledge encoded in a library of assumptions and rules over symbolic quantities. For example the Riemann zeta function has hundreds of known idenities and representations in terms of other functions. For even integer quantities is a simple closed form expressed in terms of π and Bernoulli numbers.

$$\zeta(2) = \frac{\pi^2}{6}$$

Mathematica is aware of these identities and will chose to transform the Zeta function to this reduced form automatically. For non-even values there is no simple closed form (in general) but there is a series approximation that can be computed to an arbitrary number of digits using the N function.

``` mathematica
In[49]:= Zeta[2]
Out[49]= Times[Rational[1, 6], Power[Pi, 2]]

In[50]:= Zeta[3]
Out[50]= Zeta[3]

In[51]:= N[Zeta[3], 30]
Out[51]= 1.20205690315959428539973816151
```

## Calculus

The famous XKCD cartoon illustrates the asymmetry of certain calculus problems. This cartoon finds direct representation when trying to encode simplifications over derivatives and integrals within these rewrite systems.

![](https://imgs.xkcd.com/comics/differentiation_and_integration.png)

Indeed, differentiation is almost trivial to encode using the techniques from above. The linearity of the differential operator, product rule, power rule and constant rule can be written in four lines of rules. The standard library has a larger corpus of these definitions but they are usually about this succinct.

``` mathematica
{
  Diff[f_ g_, x_]             ->  f Diff[g, x] + g Diff[f, x],
  Diff[f_ + g_, x_]           ->  Diff[f, x] + Diff[g, x],
  Diff[Pow[x_,n_Integer], x_] ->  n * Pow[x,n-1],
  Diff[x_, x_]                ->  1
  Diff[n_?NumericQ, x_]       ->  0
}
```

However many computer algebra systems are capable of doing calculations of indefinite integrals through almost magical means. For example:

``` mathematica
In[52]:= Integrate[Log[1/x], x]
Out[52]= Plus[x, Times[x, Log[Power[x, -1]]]]
```

While Mathematica’s symbolic integration capacities may seem like magic, they are indeed just the canonical example of rewriting machinery combined a corpus of transformations and some analysis from the 1930s. While there are certain integrals that are computable directly by pattern matching, there is a non-trivial result in the study of the Meijer G-function which admits a transformation of many types of transcendental functions into a convenient closed form to compute integrals.

The Meijer G-function is an indexed function of 4 vector parameters expressed the following complex integral where L runs from $−i\infty$ to $i\infty$.

$$G_{m,n}^{p,q}(z \mid \mid a_1, \ldots, a_n, a_{n+1}, \ldots, a_p \mid \mid b_1, \ldots, b_m, b_{m+1}, \ldots, b_q) = \frac{1}{2\pi i} \int_L \prod_{j=1}^m \Gamma(b_j - s) \prod_{j=1}^n \Gamma(1 - a_j + s) \prod_{j=m+1}^q \Gamma(1 - b_j + s) \prod_{j=n+1}^p \Gamma(a_j - s) z^s ds$$

This is well-studied function, and supririsngly a great many classes of special functions (trigonometric functions, Bessel functions, hypergeoemtric, etc) can be expresed in terms in terms of the G functions and thus we can reduce complex nestings of these functions down in terms of simply manipulating G-functino terms. Apply integration theorems over this reduced expression produces one or more other G-function terms. We can then expand that result back into the special functions to get the integration result. This practice does not work universally, but practically in enough cases to be incredibly useful.

One of the very general indefinte integration theorems rexpressses the integrand in terms of permutations of the G-function indexes. On the left hand side we have an integral expression and on the right hande side we have the computed integral both in terms of G-functions.

$$\int G_{m,n}^{p,q}(z \mid \mid a_1, \ldots, a_n, a_{n+1}, \ldots, a_p \mid \mid b_1, \ldots, b_m, b_{m+1}, \ldots, b_q) dz = G_{m,n}^{p+1,q+1}(z \mid \mid 1, a_1+1, \ldots, a_n+1, a_{n+1}+1, \ldots, a_p+1 \mid \mid b_1+1, \ldots, b_m+1, 0, b_{m+1}+1, \ldots, b_q+1)$$

A entire book of integrals can be reduced down to a simple set of multiplication and transform rules of G-functions in a few lines of term rewrite logic.

At the core of Mathematica’s integration engine is transformations involving this approach along with a variety of other heuristics that work quite well. For example the MeijerGReduce function can be used to transform many clases of special functions into the G-function equiveleants and then FullSimplify and other identities can reduce these expressions further.

``` mathematica
In[53]:= MeijerGReduce[Sqrt[x] Sqrt[1+x],x]
```

$$\sqrt{x} \sqrt{1+x} \quad \Rightarrow \quad
\frac{- \sqrt{x} \quad
{ G_{0,0}^{1,1}\left(x\left| \begin{array}{c} \frac{3}{2}, 0 \ - \ \end{array} \right.\right) }
}{2 \sqrt{\pi }}$$

The confluence of all of these features gives rise to a programming environment that is tailored for working with and abstract expressions in a way that differs drastically from the semantics of most mainstream languages.

It is true that this system can be used to program arbitrary logic, it can however become a bit clunky when trying to code imperative logic that works with inplace references and mutable data structures. Nevertheless it is a very powerful domain tool for working in scientific computing and the number of functions related to technical domains that are surfaced in the language represents decades of meticulous expert domain expertise.

Many universities have site licenses for this software so it is worth playing around with the ideas found in this system if you are keen on programming langauges with heterodox evaluation models and semantics.

## External References

- [Term Rewriting and All That](https://www.amazon.com/Term-Rewriting-All-Computer-Science/dp/0521301136)
- [The Essense of Strategic Rewriting](https://www.amazon.com/Essence-Strategic-Rewriting-Computer-Science/dp/0521301136)
- [Maude](https://maude.cs.illinois.edu/)
- [Pure](https://www.cs.ox.ac.uk/projects/pured/index.html)
- [Wolfram Engine](https://www.wolfram.com/engine/)
- [Mathematica Expressions](https://reference.wolfram.com/language/guide/Expressions.html)
