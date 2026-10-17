---
date: "2016-03-01"
tags: ["fp"]
url: "https://www.stephendiehl.com/posts/exotic_01/"
---

# Exotic Programming Ideas: Part 1 (Module Systems)

All successful programming languages are alike; every unsuccessful programming language is unsuccessful in its own way.

The history of software is one that runs parallel to a simultaneous but separate conversation, and that is the story of how we think about computation and how we communicate those ideas with other humans. And the story of computation has been about the evolution of this very novel and peculiar form of human expression we call code. I suspect being a programmer in the 21st century must be like what being a royal scribe was like in Ancient Egypt in 3200 BCE. There’s this new modality of communication that most of the population is unaware of, yet it’s existence simultaneously enables commerce, culture and civilization to flourish.

And just like with natural languages we’ve seen a convergence of semantics and syntax into what is becoming like Latin was for European languages. These languages fall into the paradigm of C-inspired imperative paradigm with some object oriented features. This describes languages like C++, Go, Python, Ruby, Rust, Java, and Javascript that together make up the bulk of all new code that is written today. While most of programming activity has converged onto this local maxima of productivity, there are ideas at the fringes of our programming that contain more esoteric ideas with wildly different semantics and grammar than those found in the mainstream.

Throughout my life as professional programmer, I’ve realized there are two different groups of programmers. Those that see programming languages primarily as an instrument of human reason and those that see as a means of production for specific tasks. Just like there are few limited speakers of Pirahã, Navajo, Klingon or Berik; there are programming languages with similarly limited adoption. Nonetheless these languages encode some very interesting semantic structures that are often lost in translation when we try to encode in them in the mainstream.

For my Advent Blogging (because I’m bored and in lockdown), going to write about seven language semantics features at the fringes of software culture. The ones with the wacky wild ideas that will take you on a trip through what might had been if the Romans (i.e C) hadn’t conquered.

- [Week 1 - Module Systems](exotic_01.md)
- [Week 2 - Term Rewriting](exotic_02.md)
- [Week 3 - Effect Systems](exotic_03.md)
- [Week 4 - Datalog](exotic_04.md)

## Module Systems

Let’s start with module systems and modular programming. The big idea of modules is to break code up into reusable components called modules. Modules as a language feature were first developed in Modula-2 and Pascal, which were developed as a way to demarcate units of compilation. The notion found maturity in the Standard ML language in 1984 which developed the notion further and allowed abstraction and parameterisation of modules. notion. These days full module systems are found in languages within the ML family such as F#, OCaml, Standard ML although a few others such as Agda have them as well. We’ll look at OCaml which is a statically typed ML dialect with type inference.

``` ocaml
module MyModule = struct
  (* Set of Definitions *)
end
```

The information contained in a module can be both values and types. For example the type t and the function of a single argument square.

``` ocaml
module MyModule = struct
  type t = int
  let square x = x * x
end
```

``` ocaml
module MyModule = struct
  type t = int
  let square x = x * x
end
```

The components of a module can be bound to a description known as signature which constrains both the visibility of symbols within the module and enforces a consistent interface across the module.

``` ocaml
module MyModule : sig
 val square : int -> int
end =
struct
  let square x = x * x
end
```

Alternatively module signatures can be bound to specific name at the typelevel and defined independently across multiple modules using the module type syntax. This separates from the specification from the implementation. Here we define an abstract type t that can be referenced inside the signature abstractly to refer to its eventual instantiation with a concrete type. The term s is defined to be a “value of type t” in the specification.

``` ocaml
(* Specification of MySig *)
module type MySig = sig
  (* Set of Type Signatures *)
  type t
  val s : t
end

(* Implementation of MySig *)
module MyModule : MySig = struct
  (* Set of Definitions *)
  type t = int
  let s = 0
end
```

Modules can be opened using the open syntax. This will scope all exposed symbols within the module inside the given scope or globally if used at the toplevel.

``` ocaml
open MyModule;;                (* Toplevel *)
let open MyModule in expr;;    (* In let binding *)
```

Modules can also be projected into using dot syntax to retrieve specific symbol within a module’s scope. For example:\`

``` ocaml
module Foo = struct
  let a = 5
  let b = 6
end;;

print_int(Foo.a);;
print_int(Foo.(a*b));;
```

The signature of a module need not neccessarily constrain all symbols defined in the module. A signature may seal the implementation of specific parameters or types that implementation details internal the module and not exposed publically. For example in the following module the Hello signature hides the message used by the hello function and does not allow the downstream user to modify the internals of the message itself, only to the call the function.

``` ocaml
module type Hello = sig
 val hello : unit -> unit
end

module Impl : Hello = struct
  (* Private variable message *)
  let message = "Messaage is sealed Inside Impl"
  let hello () = print_endline message
end;;

Impl.hello();;
```

Modules themselves can also be nested within other modules, to an arbitrary depth. Projection into the Outer module can retrieve the Inner module and its contents in this example:

``` ocaml
module Outer = struct
  let a = 1
  module Inner = struct
    let b = 2
  end
end

let c = Outer.Inner.b;;
```

In addition to nested, modules can include the contents of other modules either by temporarily scoping their values in scope within the module’s definition or by using the include syntax which copy’s the contents of given module’s scope into the new definition.

``` ocaml
module A = struct
  let a = 10
end

module B = struct
  let b = 20
end

module Arith1 = struct
  include A
  include B
  let c = a+b
end

module Arith2 = struct
  open A
  open B
  let c = a+b
end;;

Arith1.a;;
Arith2.a;; (* Not in scope *)
```

A module itself can be parameterised by types, including other modules. These are known as functors and allow us to instantiate modules by specifying the interface that given module parameters must conform. In this example the parameter M is an abstract module that conforms to the signature S. There is an implementation of this signature by the given module A which can be passed as the parameter M in the instantiation of FA. Within the definition of F we project into M to retrieve the s and t parameters abstractly. Functors are effectively functions from modules to modules.

``` ocaml
module type S = sig
  type t
  val s : t
end

module A : S = struct
  type t = int
  let s = 0
end

(* Functor *)
module F (M : S) = struct
  type a = M.t
  let b = M.s
end

(* F applied to module A *)
module FA = F(A);;
```

Normally two invocations of the same functor will produce modules containing equal abstract types. However we can define a different class of functor known as a generative functor which is denoted by an extra parameter () which will produce an output module containing non-equal abstract types. In this example the output types of F and G are distinct. This is particularly important when dealing with references which require mutation and uniqueness.

``` ocaml
module G (M : S) () = struct
  type a = M.t
  let b = M.s
end

module GA = G(A)();;
```

This is the essence of module systems. While many languages have simple ways of namespacing and encapsulating units of code, the ML family enriches this by making the modules themselves both first order objects and parametrisable. This is a very powerful technique that is rarely seen elsewhere and the notion of programming with functors is one that gives rise to a rich set of abstractions for code reuse. Module systems are essential part of functional programming and future languages should learn from and explore new branches in this rich design space.

External References

- [Cornell CS3110](https://www.cs.cornell.edu/courses/cs3110/2019sp/textbook/modules/modular_programming.html)
- [Practical Foundations for Programming: Chapter 42](https://www.amazon.com/Practical-Foundations-Programming-Languages-Robert-ebook/dp/B00B4V6AB2)
