---
date: "2024-02-01"
tags: ["datalog", "logic-programming", "fp"]
url: "https://www.stephendiehl.com/posts/souffle/"
---

# Cooking Logics with Soufflé

Soufflé is an efficient Datalog engine designed for program analysis and knowledge representation. In this tutorial, we'll demonstrate basic and advanced features of Soufflé, including component parameterization and algebraic data types.

Ensure you have Soufflé installed. You can download it from the [Soufflé GitHub repository](https://github.com/souffle-lang/souffle) and follow the installation instructions.

Datalog is a declarative language for defining data queries and transformations. It's particularly useful for pattern matching and recursive data processing. If you're new to Datalog, you might find it helpful to start with a simple example.

``` prolog
// Simple example: Relationships
.decl parent(x: symbol, y: symbol)
.decl ancestor(x: symbol, y: symbol)

parent("Alice", "Bob").
parent("Bob", "Charlie").
parent("Charlie", "David").

ancestor(X, Y) :- parent(X, Y).
ancestor(X, Y) :- parent(X, Z), ancestor(Z, Y).
```

In this example:

- We define a `parent` relationship.
- We derive an `ancestor` relation using recursion.

To run the program, save your code in a file called `family.dl`, then run the following command:

``` bash
souffle family.dl
```

Souffle supports the standard functional programming features, such as component parameterization and algebraic data types.

Component parameters enable you to create reusable modules in Soufflé. Consider a scenario where we want to analyze two graphs using a similar structure but with different parameters.

Create a Datalog file called `graph.dl`:

``` prolog
.decl edge(X: symbol, Y: symbol)
.decl reach(X: symbol, Y: symbol)

edge("A", "B").
edge("B", "C").
edge("C", "D").

reach(X, Y) :- edge(X, Y).
reach(X, Y) :- edge(X, Z), reach(Z, Y).
```

Now, we can easily create different graphs by saving them as separate Datalog files while referencing the same component.

Soufflé also supports algebraic data types like sum types and product types. This feature allows you to model complex data structures.

**Sum Types**

Sum types represent a value that can take on multiple forms. Here’s an example of a sum type for shapes:

``` prolog
.type Shape = Circle {radius: float} | Square {side: float}
.decl shape_instance(name: symbol, shape: Shape)
.output shape_instance

shape_instance("Circle1", $Circle(3.0)).
shape_instance("Square1", $Square(2.5)).
```

**Product Types**

Product types combine multiple values. Here’s an example of a product type representing a 2D point:

``` prolog
.type Point = [x: float, y: float]
.decl point_instance(name: symbol, p: Point)
.output point_instance

point_instance("Point1", [1.0, 2.0]).
point_instance("Point2", [3.5, 4.5]).
```

**Modules**

You can create more complex structures by combining component parameterization with algebraic data types. You can combine the shape and point definitions into a reusable component:

``` prolog
.type Shape = Circle {radius: float} | Square {side: float}
.type ShapeParam = [name: symbol, shape: Shape]

.decl shape_instance(param: ShapeParam)
.output shape_instance
shape_instance(["Circle1", $Circle(3.0)]).
shape_instance(["Square1", $Square(2.5)]).
```

In addition to the basic features, Soufflé supports advanced features such as component parameterization and algebraic data types. Components behave much like generative modules in Standard ML and can be parameterized by types.

``` prolog
.comp Catalog<T> {
    .type ItemID <: T
    .decl item(id: ItemID, title: symbol, author: symbol)
    .decl borrower(id: symbol, name: symbol)
    .decl borrowed(item_id: ItemID, borrower_id: symbol, due_date: symbol)

    .decl overdue(item_id: ItemID, borrower_id: symbol, days_overdue: number)
    overdue(ItemID, BorrowerID, DaysOverdue) :-
        borrowed(ItemID, BorrowerID, DueDate),
        to_number(DueDate) < to_number("2024-03-01"),  // Assuming current date is 2024-03-01
        DaysOverdue = to_number("2024-03-01") - to_number(DueDate).

    .decl popular_item(id: ItemID, borrow_count: number)
    popular_item(ItemID, count(BorrowerID)) :-
        borrowed(ItemID, BorrowerID, _),
        count(BorrowerID) > 1.
}

.init BookCatalog = Catalog<symbol>
.init MovieCatalog = Catalog<number>

BookCatalog.item("B001", "1984", "George Orwell").
BookCatalog.item("B002", "To Kill a Mockingbird", "Harper Lee").
BookCatalog.borrower("U001", "Alice").
BookCatalog.borrower("U002", "Bob").
BookCatalog.borrowed("B001", "U001", "2024-02-15").
BookCatalog.borrowed("B002", "U002", "2024-02-20").
BookCatalog.borrowed("B001", "U002", "2024-03-05").

MovieCatalog.item(1, "The Godfather", "Francis Ford Coppola").
MovieCatalog.item(2, "Pulp Fiction", "Quentin Tarantino").
MovieCatalog.borrower("U003", "Charlie").
MovieCatalog.borrower("U004", "David").
MovieCatalog.borrowed(1, "U003", "2024-02-25").
MovieCatalog.borrowed(2, "U004", "2024-02-28").
MovieCatalog.borrowed(1, "U004", "2024-03-10").

.decl overdue_books(id: symbol, borrower: symbol, days_overdue: number)
.output overdue_books
overdue_books(ID, Borrower, Days) :- BookCatalog.overdue(ID, Borrower, Days).

.decl popular_movies(id: number, borrow_count: number)
.output popular_movies
popular_movies(ID, Count) :- MovieCatalog.popular_item(ID, Count).
```
