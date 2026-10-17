---
date: "2016-03-04"
tags: ["fp", "logic-programming"]
url: "https://www.stephendiehl.com/posts/exotic_04/"
---

# Exotic Programming Ideas, Part 4 (Datalog)

Continuing on in our series on exotic programming ideas, we’re going to explore the topic of logic programming and a particular form known as datalog.

- [Week 1 - Module Systems](exotic_01.md)
- [Week 2 - Term Rewriting](exotic_02.md)
- [Week 3 - Effect Systems](exotic_03.md)
- [Week 4 - Datalog](exotic_04.md)

Datalog is a very simple formalism, it consists of only two things:

1.  A database of facts
2.  A set of rules for deriving new facts from existing facts

Datalog is executed by a query processor that given these two inputs, finds all instance of facts implied by both the databased and rules. For our examples we’re going to be coding our examples in the Souffle language. The namesake of the language is an acronym for the Systematic, Ontological, Undiscovered Fact Finding Logic Engine. It can be installed simply on many Linux systems with the command:

``` bash
$ apt-get install souffle
```

Souffle is a minimalist datalog system designed for complex queries over large data sets, such as those encountered in the context of doing static program analysis over large codebases. Souffle is implemented in C++ and can compile datalog programs into standalone executables via compilation to C. It is one of the simpler and most efficient implementations of these systems to date, and notably it has a simple type system on top of the logic engine.

Datalog encodes a decidable fragment of logic for which satisfaction is directly computable over a finite universe of terms. These terms consist of three classes of expressions. A fact represented syntactically by a expression suffixed by a period.

``` python
fact.
```

A rule represented by a head and body term with a turnstile (⊢) symbol between.

``` python
head :- body.
```

And query terms which query the truth value of fact within the database.

``` python
?- body.
```

An example of these terms would be “classical” categorical syllogism about the Greek philosopher Socrates. Socrates is a man, all men are mortal, therefore we can deduce that Socrates is mortal.

``` python
# All men are mortal / valar morghulis
mortal(X) :- human(X).

# Socrates is a man.
human("Socrates").

# Is Socrates mortal?
?- mortal("Socrates").
```

Execution of a Souffle program is in effect a set of transformations from input facts to deduced output facts by a prescription of rules.

Souffle can read input facts from a variety of sources including stdin, CSV files, and SQLite databases. he default input source is a tab-separated file for a relation where each row in the file represents a fact in the relation. This is specified with a IO input declaration of the form.

``` python
.decl A   (n: symbol)
.input A
```

This would read from a file A.facts of tab separated values for a symbolic value n. However we can modify our input by passing explicit IO parameters to it instead.

``` python
.input A(IO=file, filename="fact_database.csv",  delimiter=",")
.input A(IO=stdin, delimiter=",")
.input A(IO=sqlite, dbname="fact_database.db")
```

## Simple Example

A simple end to end example would be the program that associates a set of symbolic inputs with a set of numerical ones by a simple one-to-one mapping. We could write the simple A to B transformation as the following rules.

``` python
# Input from A.facts
.decl A  (n: symbol)
.input A

# Output to B.csv
.decl B  (n: number)
B(0) :- A("Hello").
B(1) :- A("World").
.output B
```

This reads from the input facts in A.facts and outputs B.csv which not surprisingly looks like the following when we invoke Souffle with the following command.

``` bash
$ souffle hello.dl
```

The output matches A to B according to the rules in hello.dl.

| A     | B   |
|-------|-----|
| Hello | 0   |
| World | 1   |

Relations Logic inside of datalog is specified by relations, which are ordered tuples of typed variables that relate quantities. For example if we had a set of named entities (represented as symbols) we can assign a relation that maps a property to each of them as facts.

``` python
.decl Human(x : symbol)
.decl Klingon(x : symbol)
.decl Android(x : symbol)

Human("Picard").
Human("Riker").
Klingon("Worf").
Android("Data").
```

Each variable is given a specific type indicated by var : type where type is one of the following builtins.

- symbol - Symbolic quantities represented as strings.
- number - Signed integers
- unsigned - Unsigned integers
- float - Floating point values

Relations can take multiple arguments that assign properties to a collection of symbols according to rules over that relation.

``` python
.decl Rank(x:symbol, y:symbol)
.decl Peers(x:symbol, y:symbol)

Rank("Captain", "Picard").
Rank("Officer", "Riker").
Rank("Officer", "Worf").
Rank("Officer", "Data").
```

Rules over these relations are defined by clauses. The following can be read as a pair (x,y) is in A relation, if it is in B relation.

``` python
A(x,y) :- B(x,y).
```

Clauses in Souffle are known in logic as Horn clauses. They are comprised of subterms joined together with a conjunction (∧) (logical AND in programming) that implies a statement u. Can read the following statement in math as

``` python
if a and b and … and z all hold, then also u holds
```

$$u \leftarrow a \land b \land \ldots \land z$$

The above is known as the implicative form. Logically this is equivalent to the statement written in terms of disjunction (∨) (logical OR in programming) and negation (¬) (logical NOT in programming).

$$\neg a \lor \neg b \lor \ldots \lor z$$

In Datalog notation we write this Horn clause in the following syntax.

``` python
u :- a, c, ..., z.
```

In clauses, free variables that occur inside of the conjoined expressions are implicitly universally quantified, i.e. they are required to hold forall (denoted ∀) substitutions of the variable in a domain. The domain in this case is the set of inhabitants of a given type (i.e. symbol, unsigned, etc).

So the datalog expression above

``` python
A(x,y) :- B(x,y).
```

Is equivalent to the Horn clause.

$$\forall x. \forall y. \neg B(x,y) \lor A(x,y)$$

So for example we know that Klingons and Humans both eat food, but obviously androids do not. We write the following rules over a variable term X of type symbol.

``` python
.decl Eats(x:symbol)

Eats(X) :- Human(X).
Eats(X) :- Klingon(X).
.output Eats
```

Which produces the following output from the given set of input facts about the symbols in scope given the constraint on their species.

| Eats   |
|--------|
| Riker  |
| Picard |
| Worf   |

Terms can also be negated, for instance if we wanted to enumerate all officers that eat could write the following rule which excludes androids.

``` python
InvitedToDinner(X) :- Rank("Officer", X), !Android(X).
```

| InvitedToDinner |
|-----------------|
| Riker           |
| Worf            |

A more complex rule can consist of multiple terms separated by command, all of which have to evaluate true. For example two officers are peers if they have same rank and are not the same person. We use an intermediate variable Z to test the equality of rank in this relation.

``` python
.decl Peers(x:symbol, y:symbol)
Peers(X, Y) :- Rank(Z, X), Rank(Z, Y),  X != Y.
```

| Peers |
|-------|
| Riker |
| Worf  |
| Riker |
| Data  |
| Worf  |
| Data  |
| Riker |
| Worf  |

Rules can also be recursive. For example we we wanted to write a query which found all first degree friend relations of a given set of facts we could write the following recursive definition.

``` python
.decl Friend(x:symbol, y:symbol)
.decl FriendOfFriend(x:symbol, y:symbol)

FriendOfFriend(X, Y) :- Friend(X, Z), FriendOfFriend(Z, Y).
```

| FriendOfFriend |
|----------------|
| Riker          |
| Worf           |

An important class of relations are that of equivalence relations. These are relations between two terms that satisfy an additional set of constraints: reflexivity, symmetry, and transitivity. Relations of this form behave similar to how the equality symbol is treated in algebra.

$$a = a$$

$$a = b \Rightarrow b = a$$

$$a = c, b = c \Rightarrow a = c$$

``` python
.decl equivalence(x:number, y:number)
equivalence(a, b) :- R(a), R(b).

// reflexivity
equivalence(a, a) :- equivalence(a, _).

// symmetry
equivalence(a, b) :- equivalence(b, a).

// transitivity
equivalence(a, c) :- equivalence(a, b), equivalence(b, c).
```

In the implementation of the relation, the data and intermediary deductions are stored in a common B-tree data structure. However this can be modified by passing an explicit argument to force the use of a disjoint-set or trie data structure for specific uses cases.

``` python
.decl A(x : number, y : number) eqrel // Disjoint-set
.decl B(x : number, y : symbol) btree // B-tree
.decl C(x : number, y : symbol) brie  // Dense trie
```

When working with integral types (number, unsigned and float) arithmetic expressions behave as expected and can be pattern matched on. For example we can write the Fibonacci function as a relation in terms of itself.

``` python
fib(0, 0).
fib(1, 1).
fib(n+1, x+y) :- fib(n, x), fib(n-1, y), n < 15.
```

## Type System

Souffle has a limited type system that can be used to track the types of variables quantified in clause expressions. For example one cannot have a variable which simultaneously holds a symbol and unsigned integer. We can however define custom type synonyms on top of the ground types of the languages that will let us semantically distinguish different user-defined types of symbols and numeric quantities. For example:

``` python
.type Person <: symbol
.type Food <: symbol

.decl Eats(x : Person, y : Food)
.decl Dinner(x : Person, y : Food)

Dinner(x, y) :- Eats(x, y). // Well-typed
Dinner(x, y) :- Eats(y, x). // Not well-typed, Person != Food
```

Unions of type synonyms are also permitted so long as the underlying types of the synonyms are equivalent.

``` python
.type A <: number
.type B <: number
.type C = A | B    // Either in A or B
The language also has the ability to define both sum and product types which let us both options and records as terms in our clauses.

.type Sum = A { x: number } | B { x: symbol }
.type Prod = C { x: number, y : symbol }
For example we could build a linked list (or cons list) out of a product type which has both a head element and a tail element.

// Linked list
.type List = [ head : number, tail : List ]
.decl MyList(x : List)

MyList( [1, [2, nil]] ).
```

Logic programming is a very natural way of expressing abstract program semantics and with tools like Souffle we have excellent logic engine that can be embedded in other static analysis applications to collect facts about code and deduce implications about code composition and usage. This a powerful technique and future static analysis tools are going to yield whole new ways of reasoning about and synthesising code in the future.

## External References

- [Souffle Language Reference](https://souffle-lang.github.io/souffle/language-reference.html)
- [SWI-Prolog Online](https://www.swi-prolog.org/)
- [μZ in Z3](https://github.com/Z3Prover/z3/blob/master/examples/python/muz.py)
- [Tool Talk: Souffle](https://www.youtube.com/watch?v=Z_Z_Z_Z_Z)
- [Domain Modeling with Datalog](https://www.youtube.com/watch?v=Z_Z_Z_Z_Z)
