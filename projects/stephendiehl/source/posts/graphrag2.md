---
date: "2024-11-12"
tags: ["ai", "python"]
url: "https://www.stephendiehl.com/posts/graphrag2/"
---

# Tiny GraphRAG (Part 2)

In [Part 1](graphrag1.md), we built a minimal implementation of GraphRAG that demonstrated the core concepts. Now we'll extend our implementation with three significant improvements that make it more suitable for production use:

1.  Replacing serialized networkx graphs with Memgraph, a proper graph database.
2.  Adding hierarchical community detection to better capture the natural structure of documents.
3.  Implementing entity disambiguation to resolve different mentions of the same entity to a canonical form.

The complete implementation of these features is available in the repository in the `micro-graphrag` branch. The expanded implementation comes in at around 1500 lines of code with the new additions.

- <https://github.com/sdiehl/tiny-graphrag/tree/micro-graphrag>

## Memgraph Configuration

To use Memgraph as our graph database, we'll add the necessary services to our `docker-compose.yml`:

``` yaml
services:
  memgraph:
    image: memgraph/memgraph-mage:1.19-memgraph-2.19
    container_name: memgraph
    ports:
      - "7687:7687" # Bolt protocol port
      - "7444:7444" # HTTP API port
    command: ["--log-level=TRACE", "--query-execution-timeout-sec=0"]
    volumes:
      - mg_lib:/var/lib/memgraph

  lab:
    image: memgraph/lab:latest
    container_name: memgraph-ui
    ports:
      - "3000:3000"
    depends_on:
      - memgraph
    environment:
      - QUICK_CONNECT_MG_HOST=memgraph
      - QUICK_CONNECT_MG_PORT=7687
      - QUICK_CONNECT_MG_USER=admin
      - QUICK_CONNECT_MG_PASSWORD=admin

volumes:
  mg_lib:
```

This configuration:

1.  Sets up the Memgraph database with MAGE extensions for advanced graph algorithms
2.  Provides a web-based UI (Memgraph Lab) for visualizing and querying the graph
3.  Configures persistent storage through a Docker volume
4.  Exposes necessary ports for both the database (7687) and UI (3000)

The connection details are configured in our `GraphConfig` class:

``` python
@dataclass
class GraphConfig:
    """Memgraph connection configuration."""
    uri: str = "bolt://localhost:7687"
    username: str = "admin"
    password: str = "admin"
    database: str = "memgraph"
```

This setup complements our existing PostgreSQL/pgvector service, providing:

- PostgreSQL with pgvector for vector embeddings and search
- Memgraph for efficient graph storage and querying
- Web UI for graph visualization and exploration

The combination allows us to leverage both vector operations and graph operations effectively in our GraphRAG implementation.

## Build Pipeline Changes (`build.py`)

The build pipeline has been updated with new dataclasses and methods to support our enhanced features. The core data structures have been expanded to better represent our document processing pipeline:

``` python
@dataclass
class Entity:
    """Graph node with type information."""
    id: str
    type: str

@dataclass
class Relation:
    """Graph triple representing relationships."""
    head: str
    relation_type: str
    tail: str

@dataclass
class DocumentChunkData:
    """Document chunk with embedding."""
    text: str
    embedding: Any

@dataclass
class ProcessedDocument:
    """Complete processed document data."""
    chunks: List[DocumentChunkData]
    entities: List[Entity]
    relations: List[Relation]
```

The main document processing pipeline has been significantly enhanced to support entity disambiguation and improved graph construction. The process now happens in two passes:

1.  First pass collects all entity mentions with their context
2.  Second pass builds the graph using resolved canonical entities

Here's the core implementation:

``` python
def process_document(
    filepath: str,
    title: Optional[str] = None,
    max_chunks: int = -1,
    entity_types: List[str] = MIN_ENTITY_TYPES,
    relation_types: List[str] = DEFAULT_RELS_LIST,
) -> ProcessedDocument:
    """Process document with entity disambiguation."""
    # First pass: collect all entity mentions
    all_mentions = []
    for chunk_text, _ in page_chunks[:max_chunks]:
        extraction = extract_rels(chunk_text, entity_types, relation_types)
        for ent in extraction.entities:
            all_mentions.append((ent[0], ent[1], chunk_text))

    # Resolve entities to canonical forms
    disambiguator = EntityDisambiguator(model)
    resolved_entities = disambiguator.resolve_entities(all_mentions)

    # Map mentions to canonical forms
    entity_map = {
        m[0]: r[0]
        for m, r in zip(all_mentions, resolved_entities)
    }

    # Second pass: create graph with resolved entities
    g = nx.Graph()
    for chunk_text, _ in page_chunks[:max_chunks]:
        extraction = extract_rels(chunk_text, entity_types, relation_types)

        for rel in extraction.relations:
            if rel[0] in entity_map and rel[2] in entity_map:
                head = entity_map[rel[0]]
                tail = entity_map[rel[2]]
                g.add_edge(head, tail,
                    label=rel[1],
                    source_chunk=chunk_text
                )

    return g
```

The two-pass approach allows us to gather all possible entity mentions with their context before making disambiguation decisions, create a clean graph structure using only canonical entity forms, and maintain proper relationship mapping between disambiguated entities.

The process integrates with our new Memgraph storage system through the `GraphStore` class, which handles persistence and retrieval of the graph data. This updated pipeline enables better entity resolution through context-aware disambiguation, produces a cleaner graph structure with deduplicated entities, improves relationship accuracy through canonical entity forms, and provides efficient storage and retrieval through Memgraph integration.

The build process now produces a more robust knowledge graph that better captures the semantic relationships in the document while handling entity variations and ambiguity. This enhanced graph structure directly improves both local and global search capabilities by providing more accurate entity relationships and better-organized community structures.

## Memgraph Integration (`graph.py`)

Our original implementation used serialized networkx graphs stored as pickle files. While this worked for demonstration purposes, it's not suitable for production use. Let's replace it with Memgraph, a high-performance graph database optimized for machine learning workloads.

We'll replace our pickle-based storage with a new `GraphStore` class that handles all graph operations through Memgraph. This class provides a clean interface for storing and retrieving graph data, with methods for adding entities and relationships, querying the graph structure, and maintaining document-specific subgraphs. The Memgraph backend gives us significant performance improvements through its optimized graph traversal algorithms and built-in support for parallel queries. Here's the core implementation:

``` python
class GraphStore:
    """Handles graph storage and retrieval using Memgraph."""

    def __init__(self, config: GraphConfig | None = None):
        self.driver = GraphDatabase.driver(
            (config or GraphConfig()).uri,
            auth=((config or GraphConfig()).username,
                 (config or GraphConfig()).password)
        )

    def store_graph(
        self,
        doc_id: int,
        entities: List[Tuple[str, str]],
        relations: List[Tuple[str, str, str]],
        source_chunk: str,
    ) -> None:
        """Store entities and relations in Memgraph."""
        with self.driver.session() as session:
            # Store entities with vector embeddings
            for text, label in entities:
                query = """
                MERGE (n:Entity {
                    content: $content,
                    label: $label,
                    doc_id: $doc_id
                })
                """
                session.run(query, {
                    "content": text,
                    "label": label,
                    "doc_id": doc_id
                })

            # Store relations with source context
            for head, rel_type, tail in relations:
                query = """
                MATCH (h:Entity {content: $head}),
                      (t:Entity {content: $tail})
                CREATE (h)-[:RELATES {
                    type: $rel_type,
                    doc_id: $doc_id,
                    source_chunk: $source_chunk
                }]->(t)
                """
                session.run(query, {
                    "head": head,
                    "tail": tail,
                    "rel_type": rel_type,
                    "doc_id": doc_id,
                    "source_chunk": source_chunk
                })
```

## Hierarchical Communities (`communities.py`)

Our original community detection implementation used a flat structure where each entity belonged to exactly one community. This can be limiting for large documents where topics have natural hierarchical relationships. Unlike our previous flat implementation where each entity belonged to exactly one community, this approach creates multiple levels of communities, with larger communities containing smaller subcommunities.

For example, when analyzing the Obama Wikipedia article, we might get a hierarchical structure like:

- Level 0: Major Life Periods
  - Political Career Community
  - Level 1: Presidential Terms
    - First Term Community
    - Level 2: Key Events
      - Financial Crisis Response
      - Healthcare Reform
      - Bin Laden Operation
    - Second Term Community
    - Level 2: Key Events
      - Climate Change Actions
      - Iran Nuclear Deal
      - Cuba Relations
  - Level 1: Pre-Presidential Career
    - Senate Career Community
    - State Senate Community
  - Personal Life Community
  - Level 1: Education
    - Columbia University
    - Harvard Law School
  - Level 1: Early Life
    - Hawaii Background
    - Chicago Community Work

The `HierarchicalCommunity` class represents each community node in this tree. It stores:

- The triples (entity relationships) contained in that community
- A list of subcommunities at the next level down
- An optional summary of the community's contents
- The level/depth in the hierarchy

The `build_hierarchical_communities` function recursively builds this tree structure by:

1.  Running community detection on each subgraph
2.  Extracting the relationship triples for that community
3.  Recursively detecting subcommunities within each community
4.  Stopping when it hits the maximum depth or minimum community size

This hierarchical approach allows for more nuanced analysis of document structure and relationships between entities at different scales. In the Obama example above, we can see how it naturally captures both broad life periods and specific events/accomplishments within his presidency.

``` python
@dataclass
class HierarchicalCommunity:
    """Represents a hierarchical community structure."""
    triples: List[Tuple[str, str, str, str]]
    subcommunities: List["HierarchicalCommunity"]
    summary: Optional[str] = None
    level: int = 0

def build_hierarchical_communities(
    g: nx.Graph,
    max_levels: int = 3,
    min_size: int = 5
) -> CommunityResult:
    """Build hierarchical communities from a graph."""

    def build_level(subgraph: nx.Graph, level: int) -> List[HierarchicalCommunity]:
        if level >= max_levels or len(subgraph) < min_size:
            return []

        communities = []
        coms = algorithms.leiden(subgraph)

        for com in coms.communities:
            community_subgraph = subgraph.subgraph(com)

            # Extract triples for this community
            triples = [
                (s, data["label"], t, data["source_chunk"])
                for s, t, data in community_subgraph.edges(data=True)
            ]

            # Recursively build subcommunities
            subcommunities = build_level(community_subgraph, level + 1)

            communities.append(HierarchicalCommunity(
                triples=triples,
                subcommunities=subcommunities,
                level=level
            ))

        return communities

    return CommunityResult(
        communities=build_level(g, 0),
        node_community_map=build_community_map(g)
    )
```

This hierarchical approach provides several key benefits by enabling multi-scale analysis of document structure at different levels of granularity. Topics naturally organize themselves into meaningful hierarchies, such as "Politics" flowing down to "Presidential Campaign" and further to "Primary Elections". The approach allows for flexible querying by matching queries to the most appropriate level of detail. Additionally, it enables improved summary generation that can capture both broad themes and specific details within the document structure.

## Entity Disambiguation (`dis.py`)

Our original implementation treated entities as exact string matches, which led to missed connections between different references to the same entity (e.g., "Barack Obama" vs. "President Obama"). Let's add entity disambiguation using vector similarity and contextual cues.

Here's the new entity disambiguation system, which uses vector embeddings and contextual information to resolve different mentions of the same entity to a canonical form:

1.  Each entity mention is embedded along with its surrounding context using sentence-transformers.
2.  A similarity matrix is computed between all mention embeddings
3.  Mentions are clustered based on similarity scores above a threshold (0.85)
4.  For each cluster, a canonical form is selected based on frequency and completeness
5.  Entity types are resolved by taking the most specific type that appears in the cluster

This system handles cases like:

- Name variations ("Barack Obama" vs "President Obama")
- Titles and honorifics ("Dr. Smith" vs "John Smith")
- Abbreviations and acronyms ("United Nations" vs "UN")
- Contextual references ("the president" when referring to a specific person)

The implementation below demonstrates the core disambiguation logic:

``` python
class EntityDisambiguator:
    def __init__(self, embedding_model: SentenceTransformer):
        self.model = embedding_model
        self.mention_cache: Dict[str, np.ndarray] = {}

    def get_mention_embedding(self, mention: str, context: str) -> np.ndarray:
        """Get embedding for entity mention with context."""
        key = f"{mention}::{context}"
        if key not in self.mention_cache:
            # Combine mention with surrounding context
            text = f"{mention} | {context}"
            self.mention_cache[key] = self.model.encode(text)
        return self.mention_cache[key]

    def resolve_entities(
        self,
        mentions: List[Tuple[str, str, str]]  # (text, type, context)
    ) -> List[Tuple[str, str]]:  # (canonical_form, type)
        """Resolve entity mentions to canonical forms."""
        resolved = []
        clusters: Dict[str, List[int]] = {}

        # Compute similarity matrix
        embeddings = [
            self.get_mention_embedding(m[0], m[2])
            for m in mentions
        ]

        similarities = cosine_similarity(embeddings)

        # Cluster similar mentions
        for i in range(len(mentions)):
            assigned = False
            for canonical, cluster in clusters.items():
                # Check similarity with existing cluster
                mean_sim = np.mean([similarities[i][j] for j in cluster])
                if mean_sim > 0.85:  # Similarity threshold
                    cluster.append(i)
                    assigned = True
                    break

            if not assigned:
                # Create new cluster
                clusters[mentions[i][0]] = [i]

        # Select canonical forms and resolve types
        for canonical, cluster in clusters.items():
            # Use most frequent entity type in cluster
            types = [mentions[i][1] for i in cluster]
            resolved.append((canonical, mode(types)))

        return resolved
```

The disambiguator is integrated into the document processing pipeline:

``` python
def process_document(
    filepath: str,
    title: Optional[str] = None,
    max_chunks: int = -1,
    entity_types: List[str] = MIN_ENTITY_TYPES,
    relation_types: List[str] = DEFAULT_RELS_LIST,
) -> ProcessedDocument:
    """Process document with entity disambiguation."""
    # First pass: collect all entity mentions
    all_mentions = []
    for chunk_text, _ in page_chunks[:max_chunks]:
        extraction = extract_rels(chunk_text, entity_types, relation_types)
        for ent in extraction.entities:
            all_mentions.append((ent[0], ent[1], chunk_text))

    # Resolve entities to canonical forms
    disambiguator = EntityDisambiguator(model)
    resolved_entities = disambiguator.resolve_entities(all_mentions)

    # Map mentions to canonical forms
    entity_map = {
        m[0]: r[0]
        for m, r in zip(all_mentions, resolved_entities)
    }

    # Second pass: create graph with resolved entities
    g = nx.Graph()
    for chunk_text, _ in page_chunks[:max_chunks]:
        extraction = extract_rels(chunk_text, entity_types, relation_types)

        for rel in extraction.relations:
            if rel[0] in entity_map and rel[2] in entity_map:
                head = entity_map[rel[0]]
                tail = entity_map[rel[2]]
                g.add_edge(head, tail,
                    label=rel[1],
                    source_chunk=chunk_text
                )

    return g
```

## Conclusion

These improvements significantly enhance the system's capabilities, but they do come with some notable computational costs. The use of Memgraph requires more memory compared to serialized graphs, though this tradeoff enables better scalability. The hierarchical community detection process is computationally more intensive than simple flat clustering approaches. Additionally, the vector similarity comparisons needed for entity resolution introduce extra processing overhead during document ingestion.

The benefits provided by these improvements generally justify the additional resource usage. Query performance sees dramatic improvements, with Memgraph enabling graph queries that are 10 to 100 times faster than the previous approach. The quality of answers improves substantially through the combination of hierarchical community structure and more accurate entity disambiguation. Most importantly, the enhanced architecture allows the system to scale effectively to handle much larger documents and more complex knowledge graphs than was previously possible.

While these improvements make our implementation more production-ready, it's worth remembering that like the original GraphRAG paper, there's still a fair bit of "graph goes in, magic comes out" happening under the hood. We've added better persistence with Memgraph, smarter community detection, and more robust entity handling, but at its core we're still essentially throwing a bunch of language models at a graph and hoping they play nice together. Sometimes they do, sometimes they don't, and sometimes they produce surprisingly good results through what can only be described as "accidental competence."
