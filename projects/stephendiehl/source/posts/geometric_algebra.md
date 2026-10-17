---
date: "2013-04-21"
tags: ["mathematics", "python", "geometric algebra"]
url: "https://www.stephendiehl.com/posts/geometric_algebra/"
---

# Space-time Algebra in Python

Geometric Algebra is a very cool alternative formulation of linear algebra that provides a succinct framework for representing geometric transformations and physical phenomena. One of my favourite books is *Geometric Algebra for Computer Science* by Leo Dorst, Daniel Fontijne, and Stephen Mann. While it implements the code in a language called *GAViewer* which is a very cool environment for visualizing geometric algebra, I wanted to implement my own version in Python to really understand the concepts.

1.  **Blades**: These are the fundamental building blocks in geometric algebra representing oriented subspaces. For example:
    - A scalar (0-blade) is a point.
    - A vector (1-blade) represents a directed line.
    - A bivector (2-blade) represents a plane segment oriented in a specific direction.
2.  **Multivectors**: These are linear combinations of different blades and can represent a variety of geometric quantities.

We'll create a class structure that encapsulates different blades (scalars, vectors, bivectors) and multivectors, enabling operations such as addition, subtraction, and geometric products.

Let's start by defining the basic blade classes.

``` python
import numpy as np

class Blade:
    def __add__(self, other):
        return Multivector([self, other])

    def __sub__(self, other):
        return Multivector([self, -other])

    def __neg__(self):
        return -1 * self  # Negation

class Scalar(Blade):
    def __init__(self, value):
        self.value = value

    def __repr__(self):
        return f"{self.value}"

class Vector(Blade):
    def __init__(self, components):
        self.components = np.array(components)

    def __repr__(self):
        return f"Vector({self.components})"

    def magnitude(self):
        return np.linalg.norm(self.components)

class Bivector(Blade):
    def __init__(self, vector1, vector2):
        self.vector1 = vector1
        self.vector2 = vector2

    def __repr__(self):
        return f"Bivector({self.vector1}, {self.vector2})"
```

The `Multivector` class will represent any linear combination of blades.

``` python
class Multivector:
    def __init__(self, blades=[]):
        self.blades = blades

    def __repr__(self):
        return f"Multivector({self.blades})"

    def add(self, other):
        self.blades.extend(other.blades)

    def scale(self, scalar):
        for blade in self.blades:
            blade.value *= scalar
```

We can implement operations such as the geometric product for vectors and bivectors. Let’s define the operations inside the `Vector` and `Bivector` classes.

``` python
class Vector(Blade):
    def __init__(self, components):
        self.components = np.array(components)

    # Other methods as before...

    def __or__(self, other):
        """ Vector wedge product (bivector) """
        return Bivector(self, other)

    def __and__(self, other):
        """ Scalar product (dot product) """
        return Scalar(np.dot(self.components, other.components))


class Bivector(Blade):
    def __init__(self, vector1, vector2):
        self.vector1 = vector1
        self.vector2 = vector2

    def __repr__(self):
        return f"Bivector({self.vector1}, {self.vector2})"

    def dual(self):
        """ Returns the dual of a bivector as a vector """
        return Vector(np.cross(self.vector1.components, self.vector2.components))
```

Now let’s use our implementation to create some vectors, calculate their products, and demonstrate geometric algebra concepts.

``` python
# Create instances of Scalars, Vectors, and Bivectors
s1 = Scalar(5)
v1 = Vector([1, 0, 0])
v2 = Vector([0, 1, 0])
biv = v1 | v2  # Wedge product (Bivector)

# Output representations
print("Scalar:", s1)
print("Vector 1:", v1)
print("Vector 2:", v2)
print("Bivector (v1 wedge v2):", biv)

# Vector dot product
dot_product = v1 & v2
print("Dot Product (v1 · v2):", dot_product)

# Vector wedge product to get a bivector
biv_product = v1 | v2
print("Wedge Product Bivector (v1 ∧ v2):", biv_product)
```

Building upon our foundation we can extend the implementation to Space-Time Algebra (STA). STA integrates time as a dimension alongside space, allowing us to represent entities and transformations in a four-dimensional context (3 spatial dimensions and 1 temporal dimension).

In STA, we introduce the concept of a time vector, allowing the treatment of events and transformations in both space and time. We'll define a time unit (often represented as $c$, the speed of light, for consistency in relativistic contexts) to relate spatial and temporal components.

We'll modify our `Vector` class to include time as a component, creating a `SpaceTimeVector` class that encompasses three spatial dimensions and one temporal dimension.

``` python
class SpaceTimeVector(Blade):
    def __init__(self, components):
        # Expecting components in the form [x, y, z, t]
        assert len(components) == 4, "SpaceTimeVector must have four components: [x, y, z, t]."
        self.components = np.array(components)

    def __repr__(self):
        return f"SpaceTimeVector({self.components})"

    def magnitude(self):
        """ Calculate the Minkowski norm (using (-,+,+,+) signature) """
        return np.sqrt(-self.components[3]**2 + np.sum(self.components[:3]**2))

    def __add__(self, other):
        return SpaceTimeVector(self.components + other.components)

    def __sub__(self, other):
        return SpaceTimeVector(self.components - other.components)

    def __or__(self, other):
        """ Wedge product (resulting in a bivector in 3D + time) """
        return SpaceTimeBivector(self, other)

    def __and__(self, other):
        """ Scalar product (dot product, returns a scalar) """
        return Scalar(np.dot(self.components, other.components))
```

The `SpaceTimeBivector` class represents the geometric product of two `SpaceTimeVector` instances.

``` python
class SpaceTimeBivector(Blade):
    def __init__(self, vector1, vector2):
        self.vector1 = vector1
        self.vector2 = vector2

    def __repr__(self):
        return f"SpaceTimeBivector({self.vector1}, {self.vector2})"

    def dual(self):
        """ Returns the dual of a bivector as a SpaceTimeVector """
        return SpaceTimeVector(np.cross(self.vector1.components[:3], self.vector2.components[:3]))
```

Now let's create instances of `SpaceTimeVector` and demonstrate the functionality.

``` python
# Create space-time vectors
st_vector1 = SpaceTimeVector([1, 2, 3, 4])  # (x, y, z, t)
st_vector2 = SpaceTimeVector([4, 5, 6, 7])  # (x', y', z', t')

print("Space-Time Vector 1:", st_vector1)
print("Space-Time Vector 2:", st_vector2)

st_vector_sum = st_vector1 + st_vector2
print("Sum of Space-Time Vectors:", st_vector_sum)

st_vector_diff = st_vector1 - st_vector2
print("Difference of Space-Time Vectors:", st_vector_diff)

dot_product_st = st_vector1 & st_vector2
print("Dot Product of Space-Time Vectors:", dot_product_st)

# Wedge product to get a space-time bivector
st_bivector = st_vector1 | st_vector2
print("Wedge Product Space-Time Bivector:", st_bivector)

magnitude = st_vector1.magnitude()
print("Magnitude of Space-Time Vector 1:", magnitude)
```
