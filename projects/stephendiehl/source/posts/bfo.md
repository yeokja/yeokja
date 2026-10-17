---
date: "2024-07-21"
tags: ["logic-programming", "ontology"]
url: "https://www.stephendiehl.com/posts/bfo/"
---

# Basic Formal Ontology

The *Basic Formal Ontology* is an upper ontology that can describe a broad swatch of human knowledge. It's a cornerstone for logic programmers and those creating comprehensive knowledge graphs which cover multiple domains.

graph TD;\
  Q --\> Q;\
  Q\[Variable Order-Class\] --\> P;\
  P\[Metaclass\] --\> Z;\
  Z\[Class\] --\> A;\
  A\[Entity\] --\> B\[Continuant\];\
  A --\> C\[Occurrent\];\
  B --\> D\[Independent Continuant\];\
  B --\> E\[Dependent Continuant\];\
  D --\> F\[Object\];\
  D --\> G\[Object Aggregate\];\
  D --\> H\[Fiat Object\];\
  E --\> I\[Specifically Dependent Continuant\];\
  E --\> J\[Generically Dependent Continuant\];\
  I --\> K\[Quality\];\
  I --\> L\[Disposition\];\
  C --\> M\[Process\];\
  C --\> N\[Process Boundary\];\
  C --\> O\[Temporal Region\];\

With BFO providing an underlying structure, this ontology encompasses fundamental categories and principles, enabling better communication and synergistic understanding among professionals.

The hierarchical composition of BFO distinguishes major ontological categories, reflective of our comprehension of reality:

### Entity

As the overarching category in BFO, entities encompass every conceivable existence, which is bifurcated into:

- **Continuants**: Entities retaining their identity across time, including:
  - **Material Entities**: Tangible objects like cells and tissues.
  - **Immaterial Entities**: Non-physical elements like policies or ideas.
- **Occurrents**: Time-bound occurrences, for example, biological processes like cell division.

### Dependent Continuants

These entities necessitate the existence of others for their survival, incapable of standalone presence. They are differentiated into:

- **Specifically Dependent Continuant (SC)**: Qualities tethered to a singular entity, such as a car's color.
- **Generically Dependent Continuant (GC)**: Traits applicable across several instances, like the shared feature of "redness" in all red objects.

### Independent Continuants

Entities within this category can exist autonomously, which includes:

- **Objects**: Individual material items, like a particular book.
- **Object Aggregates**: Assemblages of objects, such as a book collection.
- **Fiat Objects**: Definitions based more on relational contexts than on innate physical characteristics, such as a designated parking space.

### Qualities

Observable or quantifiable characteristics of entities fall into this category, with illustrations like:

- **Disposition**: A latent tendency to manifest a condition under specific circumstances, such as fragility.
- **Relational Quality**: Attributes elucidating relationships among entities, like a parental connection.

### Process

Processes embody temporal developments or changes, which are definable through their involved entities and chronology. Key sub-categories include:

- **Process Boundary**: Markers denoting the commencement or termination of processes.
- **History**: Documented sequences of events across a temporal expanse.

### Regions

Regions cater to the demarcation of spatial or temporal planes where entities reside or events transpire:

- **Spatial Regions**: Three-dimensional spaces occupied by material entities.
- **Temporal Regions**: Linear time segments encompassing occurrences.

### Extensions to BFO

To augment BFO's core ontology, **metaclasses**, which classify other classes, can be incorporated. Additionally, the introduction of **variable-order classes**—abstract constructs designating classes whose exemplars may span various class orders or even include non-classes—is noteworthy. Unique to variable-order classes is their classification as a subclass and superclass of themselves.

For example, the hierarchy of a "Barack Obama" is:

**Lower Ontology**

- "Barack Obama" instance of "human"
- "human" subclass of "person"
- "person" subclass of "individual"
- "individual" subclass of "agent"

**Upper Ontology**

- "agent" subclass of "independent continuant"
- "independent continuant" subclass of "continuant"
- "continuant" subclass of "entity"
- "entity" instance of "class"

With that you can categorize everything. Well, almost everything—as Hamlet might, "There are more things in Heaven and Earth than are dreamt of in your ontology." But BFO comes impressively close.
