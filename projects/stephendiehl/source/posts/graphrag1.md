---
date: "2024-11-04"
tags: ["ai", "python"]
url: "https://www.stephendiehl.com/posts/graphrag1/"
---

# Tiny GraphRAG (Part 1)

We're going to build a tiny 1000 line implementation of a GraphRAG algorithm originally invented by Microsoft. I consistently hear people talk about this algorithm at meetups, but it appears there are several orders of magnitude of people talking about it than actually using it or implementing it. Likely because the reference implementation is enormous and rather complex. So let's break it down and see if there's any merit to the hype around this approach.

For some background, naive RAG is a basic approach to augmenting LLM outputs with external information that primarily relies on vector similarity search. The process involves converting documents to text, splitting that text into chunks, and embedding these chunks into a vector space where similar semantic meanings are represented by similar vector positions. When a user submits a query, it is embedded into the same vector space, and the text chunks with the nearest vector positions are retrieved and added to the LLM's context window along with the original query.

GraphRAG is an alternative structured approach to goes beyond simple semantic search and uses a graph-based index of a document. The key idea is that it uses LLMs to build a knowledge graph from source documents, then detects communities of closely-related entities within that graph, and generates summaries for these communities. This creates a alternative structured index that can be leveraged for both local entity-specific queries and global document-wide analysis. For some tasks this can perform better than naive RAG.

We're going to build a tiny version that runs entirely locally without depending on commercial LLM providers. Our system uses:

- [Postgres](https://www.postgresql.org/) + [pgvector](https://github.com/pgvector/pgvector) for vector storage
- [sentence-transformers](https://www.sbert.net/) for embeddings
- [Meta's Llama 3.2-3B](https://huggingface.co/meta-llama/Llama-3.2-3B-Instruct) for core langauge model
- [GLiNER](https://github.com/urchade/GLiNER) for entity extraction
- [GLiREL](https://github.com/jackboyla/GLiREL) for relation extraction
- [networkx](https://networkx.org/) for graph operations

We could also trivially use a graph database (like [Memgraph](https://memgraph.com/) or [Neo4j](https://neo4j.com/)) but for the sake of simplicity we'll just store the networkx graph data serialized to disk.

The source code is available on Github under a MIT license.

- <https://github.com/sdiehl/tiny-graphrag>

## Core Ideas

When doing naive RAG, we typically rely on vector similarity to find relevant context. This works well for simple cases but can miss important contextual relationships. For example, let's look at a basic vector similarity comparison using the `sentence-transformers` library.

``` python
from sentence_transformers import SentenceTransformer

model = SentenceTransformer('all-MiniLM-L6-v2')

sentence1 = "I love programming"
sentence2 = "I enjoy writing code"

embedding1 = model.encode(sentence1)
embedding2 = model.encode(sentence2)

similarity = model.similarity(embedding1, embedding2)
print(f"Similarity score: {similarity:.4f}")
```

While this approach can identify semantically similar content, it has several key limitations. First, it doesn't capture the rich relationships between entities and concepts that exist in our documents. Second, it struggles with what is commonly called the "local context problem" - where relevant information may be scattered across different parts of a document or multiple documents, without having direct semantic similarity to the query. For example, understanding the side effects of a medication might require connecting information about its chemical composition in one section with patient outcomes described in another, even though these passages may not be semantically similar to a query about side effects. GraphRAG addresses these limitations by building a structured knowledge graph that explicitly represents relationships between entities and concepts, allowing it to trace these connections and assemble relevant context even when the individual pieces aren't semantically similar to the query. GraphRAG has two different modes, *local search* and *global search*.

Local search is optimized for entity-specific queries and operates by focusing on specific entities and their relationships within the knowledge graph. It combines structured data from the knowledge graph with unstructured data from input documents, making it particularly effective for questions about specific entities, such as "What are the healing properties of chamomile?" The process begins by taking a user query and optional conversation history, then identifies semantically-related entities in the knowledge graph. It extracts relevant information including connected entities, relationships, entity covariates, community reports, and associated text chunks from source documents. This information is then prioritized and filtered to fit within a context window before generating a response using the filtered context.

Global search, on the other hand, is designed for dataset-wide analysis and performs better at handling queries that require information aggregation across the entire dataset. It's better at identifying broader themes and patterns, making it optimal for answering high-level questions like "What are the top 5 themes in the data?" The process employs a map-reduce approach where the map phase segments community reports into text chunks and generates intermediate responses with rated importance points. The reduce phase then filters and aggregates the most important points to produce a final comprehensive response.

The key difference between these approaches is that while local search focuses on specific entity-based reasoning, global search provides broader dataset-level insights by leveraging the knowledge graph's community structure and pre-summarized semantic clusters.

## Building the Graph

GraphRAG takes a new approach to RAG by building a graph-based index of document content. There are four main steps.

1.  **Graph Construction**: We first process source documents to extract entities and relationships, building a knowledge graph representation of the content. This is done using:

    - Text chunking and preprocessing
    - Entity and relationship extraction using LLMs
    - Graph construction with extracted elements

2.  **Community Detection**: The system then uses the Leiden algorithm to detect communities of closely-related entities in the graph.

3.  **Community Summary**: For each detected community, we generate a natural language summary describing the key entities and relationships within that community. This provides a high-level overview of the topics and concepts represented in each cluster.

4.  **Map-Reduce**: For global search queries, we use a map-reduce approach where we first generate responses for each community summary (map phase) and then combine and synthesize these responses (reduce phase) to create a comprehensive answer that draws from information across the entire graph.

## Example Dataset

In our example we'll use a set of Wikipedia articles about Barack Obama to demonstrate how the system automatically detects topical clusters. The Leiden community detection algorithm will analyze the entity-relationship graph and identify densely connected subgraphs that represent coherent topics. For example, when processing Obama's biography, it will automatically detect clusters like:

- Early Life & Education: Entities about his childhood in Hawaii, education at Columbia and Harvard Law
- Political Career: His time as state senator, U.S. senator, and presidential campaigns
- Presidency: Key policies, executive actions, and administration officials
- Post-Presidency: Book deals, speaking engagements, Obama Foundation work

Each cluster is then summarized by analyzing the entities and relationships within it. For instance, the Early Life cluster summary might read: "This section covers Barack Obama's upbringing in Hawaii and Indonesia, his undergraduate studies in political science at Columbia University, and his law degree from Harvard where he was president of the Harvard Law Review."

The system determines cluster boundaries automatically based on the density of connections between entities, without requiring manual topic specification. This organic clustering helps capture the natural thematic structure of the documents across non-local chunks.

## Data Models (`db.py`)

We'll use the SQLAlchemy data models to define persistence to Postgres. The **Document** model stores the original document content and metadata. The content is stored as raw text, with optional title and timestamps.

``` python
class Document(Base):
    """Represents a document in the database."""
    __tablename__ = "documents"

    id = Column(Integer, primary_key=True)
    content = Column(Text, nullable=False)
    title = Column(String(256))
    created_at = Column(DateTime(timezone=True), server_default=func.now())
    updated_at = Column(DateTime(timezone=True), onupdate=func.now())

    # Relationships
    chunks = relationship("DocumentChunk", back_populates="document")
    communities = relationship("Community", back_populates="document")
```

The **DocumentChunk** model stores segments of documents with their vector embeddings for efficient semantic search. Each chunk contains the raw text content, its position in the original document (chunk_index), and a dense vector embedding. The model uses HNSW (Hierarchical Navigable Small World) indexing for fast approximate nearest neighbor search on embeddings, and a GIN index for full-text search on content. This enables both semantic similarity search via embeddings and keyword-based search on the text.

``` python
class DocumentChunk(Base):
    """Represents a chunk of a document with vector embedding."""
    __tablename__ = "document_chunks"

    id = Column(Integer, primary_key=True)
    document_id = Column(Integer, ForeignKey("documents.id"), nullable=False)
    content = Column(Text, nullable=False)
    chunk_index = Column(Integer, nullable=False)
    embedding = mapped_column(Vector(384), nullable=False)  # Vector embedding

    # Indexes for efficient search
    __table_args__ = (
        Index(
            "ix_document_chunks_embedding",
            embedding,
            postgresql_using="hnsw",
            postgresql_with={"m": 16, "ef_construction": 64},
            postgresql_ops={"embedding": "vector_cosine_ops"},
        ),
        Index(
            "ix_document_chunks_content_fts",  # Full-text search index
            text("to_tsvector('english', content)"),
            postgresql_using="gin",
        ),
    )
```

The **Community** model represents clusters of related entities detected in the document. Each community has a summary description (content), a vector embedding for similarity search, and stores its member node IDs as a JSON string. Like DocumentChunk, it uses HNSW indexing on the embedding for efficient similarity search. Communities are linked back to their source document and enable exploration of thematically related entity groups.

``` python
class Community(Base):
    """Represents a community of related entities."""
    __tablename__ = "communities"

    id = Column(Integer, primary_key=True)
    document_id = Column(Integer, ForeignKey("documents.id"), nullable=False)
    content = Column(Text, nullable=False)  # Community summary
    embedding = mapped_column(Vector(384), nullable=False)
    nodes = Column(Text, nullable=False)  # JSON string of node IDs

    # Indexes
    __table_args__ = (
        Index(
            "ix_communities_embedding",
            embedding,
            postgresql_using="hnsw",
            postgresql_ops={"embedding": "vector_cosine_ops"},
        ),
    )
```

The **Summary** model stores generated summaries of a community. Each summary has a text content field and a vector embedding for similarity search. Like the other models, it uses HNSW indexing on the embedding for efficient similarity search, as well as a full-text search index on the content.

``` python
class Summary(Base):
    """Represents a document summary."""
    __tablename__ = "summaries"

    id = Column(Integer, primary_key=True)
    document_id = Column(Integer, ForeignKey("documents.id"), nullable=False)
    content = Column(Text, nullable=False)
    embedding = mapped_column(Vector(384), nullable=False)

    # Indexes
    __table_args__ = (
        Index(
            "ix_summaries_embedding",
            embedding,
            postgresql_using="hnsw",
            postgresql_ops={"embedding": "vector_cosine_ops"},
        ),
        Index(
            "ix_summaries_content_fts",
            text("to_tsvector('english', content)"),
            postgresql_using="gin",
        ),
    )
```

## Document Processing Pipeline (`build.py`)

The top of of graph processing pipeline beings in `build.py` which handles the construction of the vector index and knowledge graph from our textual source. This calls out to several core functions.

- `process_document` - Takes a document filepath and processes it by chunking the text, extracting entities and relations, and building a knowledge graph. Returns both the chunked document with embeddings and the constructed graph.
- `chunk_document` - Splits a document into overlapping text segments of configurable size with some overlap between chunks to maintain context across boundaries. Returns a list of text chunks.
- `build_communities` - Analyzes the knowledge graph to detect clusters of closely related entities using community detection algorithms. Generates summaries for each community to capture the key information. Returns community assignments and summaries.
- `extract_rels` - Processes text chunks to identify relationships between entities using a combination of rule-based patterns and LLM extraction. Returns a list of extracted relationships with their entity pairs and relationship types.

``` python
def store_document(
    filepath: str,
    engine: Engine,
    title: Optional[str] = None,
    max_chunks: int = -1,
) -> Tuple[int, str]:
    """Store document in database and save graph."""
    session_local = sessionmaker(bind=engine)

    with session_local() as session:
        # Process document
        chunks, graph = process_document(filepath, title, max_chunks)

        # Store document
        doc = Document(content=open(filepath).read(), title=title)
        session.add(doc)
        session.flush()

        # Store chunks with embeddings
        for chunk_text, embedding in chunks:
            chunk = DocumentChunk(
                document_id=doc.id,
                content=chunk_text,
                embedding=embedding,
                chunk_index=0
            )
            session.add(chunk)

        # Build and store communities
        community_result = build_communities(graph)

        # Save graph to disk
        graph_path = f"graphs/{doc.id}_graph.pkl"
        with open(graph_path, "wb") as f:
            pickle.dump(graph, f)

        session.commit()
        return doc.id, graph_path
```

The `process_document` function handles the construction of hte knowledge graph entities (nodes and relations) from the chunk. These are appended into a global graph which is a networkx undirected Graph data structure. We store the source text provenance as a proeprty of each triple that is generated which is used in the local search method at the end.

``` python
def process_document(
    filepath: str,
    title: Optional[str] = None,
    max_chunks: int = -1,
    entity_types: List[str] = MIN_ENTITY_TYPES,
    relation_types: List[str] = DEFAULT_RELS_LIST,
) -> Tuple[List[Tuple[str, Any]], nx.Graph]:
    """Process a document and return chunks and graph."""
    # Read and chunk document
    page_text = open(filepath).read()
    page_chunks = chunk_document(page_text)

    # Build graph
    g = nx.Graph()

    for chunk_text, _embedding in tqdm(page_chunks[:max_chunks]):
        extraction = extract_rels(chunk_text, entity_types, relation_types)

        for ent in extraction.entities:
            g.add_node(ent[0], label=ent[1])

        for rel in extraction.relations:
            g.add_edge(rel[0], rel[2], label=rel[1], source_chunk=chunk_text)

    return page_chunks, g
```

## Chunking (`chunking.py`)

The chunking algorithm splits input documents into overlapping segments while preserving sentence boundaries and semantic coherence. It processes the text sentence by sentence, accumulating them into chunks until reaching a specified size limit (default 200 characters), then generates dense vector embeddings for each chunk using a sentence transformer model. To maintain context continuity between chunks, it implements an overlap mechanism (default 50 characters) where a portion of the previous chunk's sentences are carried forward into the next chunk. This overlap helps capture cross-chunk relationships and ensures no context is lost at chunk boundaries. The algorithm returns a list of tuples containing both the text chunks and their corresponding embeddings, which are used later for similarity search and knowledge graph construction.

``` python
def chunk_document(
    text: str,
    chunk_size: int = 200,
    overlap: int = 50
) -> List[Tuple[str, npt.NDArray[np.float32]]]:
    """Split document into overlapping chunks with embeddings."""
    sentences = [s.strip() for s in text.split(".") if s.strip()]
    chunks = []
    current_chunk: List[str] = []
    current_length = 0

    for sentence in sentences:
        sentence_length = len(sentence)

        if current_length + sentence_length > chunk_size and current_chunk:
            # Create chunk and get embedding
            chunk_text = ". ".join(current_chunk) + "."
            embedding = model.encode(chunk_text, convert_to_numpy=True)
            chunks.append((chunk_text, embedding))

            # Handle overlap
            overlap_chunk = []
            overlap_size = 0
            for s in reversed(current_chunk):
                if overlap_size + len(s) <= overlap:
                    overlap_chunk.insert(0, s)
                    overlap_size += len(s)

            current_chunk = overlap_chunk
            current_length = overlap_size

        current_chunk.append(sentence)
        current_length += sentence_length

    return chunks
```

## Entity and Relation Extraction (`extract.py`)

The extraction pipeline integrates GLiNER and GLiREL models into a spaCy workflow for entity and relation extraction. GLiNER handles named entity recognition while GLiREL extracts relationships between those entities. We configure the pipeline to use GPU acceleration for better performance and allow customization of both entity types (e.g. `Person`, `Organization`) and relation types (e.g. `occupation`, `place of birth` ) through parameters. The core extraction logic is implemented in two key functions:

``` python
def nlp_model(threshold: float, entity_types: tuple[str], device: str = DEVICE):
    """Instantiate a spacy model with GLiNER and GLiREL components."""
    custom_spacy_config = {
        "gliner_model": "urchade/gliner_mediumv2.1",
        "chunk_size": 250,
        "labels": entity_types,
        "style": "ent",
        "threshold": threshold,
        "map_location": device,
    }
    spacy.require_gpu()  # type: ignore

    nlp = spacy.blank("en")
    nlp.add_pipe("gliner_spacy", config=custom_spacy_config)
    nlp.add_pipe("glirel", after="gliner_spacy")
    return nlp
```

And the `extract_rels` method which takes text input along with entity and relation type configurations, processes it through the NLP pipeline, and returns structured extraction results containing both the identified entities and their relationships. The method applies a confidence threshold to filter out low-quality relation matches, ensuring only high-confidence relationships are included in the final output.

``` python
def extract_rels(
    text: str,
    entity_types: List[str],
    relation_types: List[str],
    threshold: float = 0.75,
) -> ExtractionResult:
    """Extract entities and relations from text."""
    nlp = nlp_model(threshold, tuple(entity_types))
    docs = list(nlp.pipe([(text, {"glirel_labels": relation_types})]))
    relations = docs[0][0]._.relations

    # Extract entities with their types
    ents = [(ent.text, ent.label_) for ent in docs[0][0].ents]

    # Extract relations above threshold
    rels = [
        (item["head_text"], item["label"], item["tail_text"])
        for item in relations
        if item["score"] >= threshold
    ]

    return ExtractionResult(entities=ents, relations=rels)
```

GLiNER is an transformer-based Named Entity Recognition model that can identify enitites in text. Unlike traditional NER (like in spaCy) models that are limited to predefined entity types, or LLM-based models that require significant computing resources, GLiNER offers a more resource-efficient solution while maintaining high performance. It uses a bidirectional transformer encoder to process text and entity types in parallel, treating NER as a matching problem between entity type embeddings and textual span representations in latent space. Despite being much smaller (only 0.3B parameters), it outperforms both ChatGPT and fine-tuned LLMs in zero-shot evaluations on various NER benchmarks.

A set of minimal default entity annotations are provided but can be optionally overloaded.

``` python
MIN_ENTITY_TYPES = [
    "Person",
    "Organization",
    "Location",
    "Product",
    "Concept",
]
```

## Named Relation Recognition (`rel_types.py`)

GLiREL (Generalist and Lightweight model for Relation Extraction) is a zero-shot relation extraction model that can classify relationships between entities in text without requiring pre-training on specific relation types. It works alongside GLiNER in the spaCy pipeline to identify semantic relationships between detected entities, providing confidence scores that allow for threshold-based filtering. The model is particularly efficient as it can handle new relation types on the fly while maintaining a lightweight architecture.

The relation recognition system defines semantic relationships between entities using a either free form relations with no constraints. Like the following

``` python
DEFAULT_RELS = [ "created", "author", "location", "place of birth", "occupation" ]
```

Or we can specify type constraints which use the NER labels to constrain the types of relations that can be created.

``` python
DEFAULT_RELS = {
    "created": {"allowed_head": ["Person"], "allowed_tail": ["Organization"]},
    "author": {"allowed_head": ["Person"], "allowed_tail": ["Document"]},
    "location": {"allowed_head": ["Location"], "allowed_tail": ["Location"]},
    "place of birth": {"allowed_head": ["Person"], "allowed_tail": ["Location"]},
    "occupation": {"allowed_head": ["Person"], "allowed_tail": ["Concept"]},
    ...
}
```

## Community Segmentation (`communities.py`)

The [Leiden algorithm](https://en.wikipedia.org/wiki/Leiden_algorithm) is a graph community detection method that improves upon the earlier Louvain algorithm for finding communities in large-scale networks. It's particularly notable for its ability to guarantee well-connected communities, which makes it ideal for the GraphRAG approach to document analysis. The algorithm works by partitioning a graph into modular communities where nodes within each community have stronger connections to each other than to nodes in other communities.

We use the Leiden algorithm in our system to detect communities within the knowledge graph constructed from document entities and relationships. The implementation uses the `networkx` and `cdlib` library for graph operations and stores the community structure alongside the original text chunks which generated the triple from the graph.

``` python
def build_communities(g: nx.Graph) -> CommunityResult:
    """Build communities from a graph."""
    communities = []
    node_community_map = {}

    # Convert to integers for algorithm
    gi = nx.convert_node_labels_to_integers(g, label_attribute="original_label")

    # Create reverse mapping
    reverse_mapping = {
        node: data["original_label"]
        for node, data in gi.nodes(data=True)
    }

    for component in nx.connected_components(gi):
        subgraph = gi.subgraph(component)
        if len(subgraph) > 1:
            # Detect communities using Leiden
            coms = algorithms.leiden(subgraph)

            for com_id, com in enumerate(coms.communities):
                # Extract community relationships
                community_subgraph = coms.graph.subgraph(com)
                community_triples = []

                for s, t, data in community_subgraph.edges(data=True):
                    triple = (
                        reverse_mapping[s],
                        data.get("label"),
                        reverse_mapping[t],
                        data.get("source_chunk")
                    )
                    community_triples.append(triple)

                communities.append(community_triples)

                # Map nodes to communities
                for node in com:
                    node_community_map[reverse_mapping[node]] = com_id

    return CommunityResult(
        communities=communities,
        node_community_map=node_community_map
    )
```

Once we have extracted communities from our knowledge graph, we need to generate natural language summaries that capture the key information and relationships contained in each community's triples. To do this, we pass the triples to a language model which synthesizes them into coherent text that describes the main concepts and connections present in that portion of the graph. This summary text provides a semantic representation of the community that we can later use for retrieval.

Since communities can contain many triples and language models have limited context windows, we need to handle large communities carefully. For communities with more triples than can fit in the context window (default 30 triples), we first split them into smaller chunks and generate intermediate summaries. These chunk summaries are then concatenated and passed through the model again to produce a final unified summary that captures the key information across all chunks. This two-pass approach allows us to summarize communities of any size while working within model constraints.

``` python
def generate_community_summary(
    llm: Llama,
    community: List[Tuple[str, str, str, str]],
    max_triples: int = 30,
    temperature: float = 0.2,
) -> str:
    # Chunk the community into smaller pieces if too large
    chunks = [
        community[i : i + max_triples] for i in range(0, len(community), max_triples)
    ]

    summaries: List[str] = []
    for chunk in chunks:
        response = llm.create_chat_completion(
            messages=[
                {
                    "role": "user",
                    "content": COMMUNITY_SUMMARY.format(community=chunk),
                }
            ],
            temperature=temperature,
        )
        summaries.append(response["choices"][0]["message"]["content"])

    # If we had multiple chunks, combine them
    if len(summaries) > 1:
        combined = " ".join(summaries)
        # Generate a final summary of the combined text
        response = llm.create_chat_completion(
            messages=[
                {
                    "role": "user",
                    "content": COMMUNITY_COMBINE.format(combined=combined),
                }
            ],
            temperature=temperature,
        )
        return response["choices"][0]["message"]["content"]

    return summaries[0]
```

When we run this on a subset of our Obama dataset we get something like the following graph with five major communities. Related to topics like the Supreme Court appointeess, Iraq War and the Democratic party that are discovered automatically from the graph structure.

<div style="text-align: center; max-width: 800px; margin: 0 auto;">

<img src="https://www.stephendiehl.com/images/23_communities.png" width="800" alt="Graph showing communities detected in Obama dataset" />

</div>

And now we're done with the construction phase of the pipeline. Now onto the search methods.

## Query (`query.py`)

The `QueryEngine` class is the core component responsible for handling different types of searches. During initialization, it sets up the Llama language model for generating responses and creates a database session factory using SQLAlchemy. The engine parameter connects to the PostgreSQL database with pgvector for our vector searches.

``` python
class QueryEngine:
    def __init__(self, engine: Engine) -> None:
        self.llm = Llama.from_pretrained(
            repo_id=MODEL_REPO,
            filename=MODEL_ID,
            local_dir=".",
            verbose=False,
            n_ctx=DEFAULT_CTX_LENGTH,
        )
        self.SessionLocal = sessionmaker(bind=engine)
        self.engine = engine
```

The query engine uses `llama-cpp-python` to interface with Meta's Llama 3.2-3B model for generating responses. The model is loaded from a local GGUF file that can be downloaded from Hugging Face. During initialization, the model is loaded with a specific context length and configuration. The core interaction happens through the `_generate_llm_response` method which provides a local interface for chat completion requests. It takes a system prompt, user content, and temperature parameter, then returns the generated response as a string.

``` python
def _generate_llm_response(
    self,
    system_prompt: str,
    user_content: str,
    temp: float = DEFAULT_TEMPERATURE
) -> str:
    """Common method for generating LLM responses."""
    response = self.llm.create_chat_completion(
        messages=[
            {"role": "system", "content": system_prompt},
            {"role": "user", "content": user_content},
        ],
        temperature=temp,
    )

    if not response or "choices" not in response:
        raise RuntimeError("Failed to get valid response from LLM")

    return str(response["choices"][0]["message"]["content"])
```

## Local Search

The local search method implements entity-centric search using the knowledge graph. It first loads the serialized graph, extracts entities from the query, gathers relevant data (entities, relationships, and text chunks) from the graph, builds a context string, and generates a response using the LLM. This approach is particularly effective for queries about specific entities and their relationships.

``` python
def local_search(self, query: str, graph_path: str) -> str:
    g = self.load_graph(graph_path)
    query_entities = self._extract_query_entities(query)
    relevant_data = self._gather_relevant_data(g, query_entities)
    context = self._build_context(relevant_data)
    return self._generate_response(query, context)
```

This helper method gathers relevant information from the knowledge graph for local search. It takes query entities and the graph as input, then collects matching nodes, their relationships, and associated text chunks. While our implementation uses simple case-insensitive substring matching to find relevant entities, a production system would likely employ more sophisticated entity matching techniques to improve accuracy and coverage. The method builds a comprehensive context for the query based on the matched entities.

``` python
def _gather_relevant_data(
    self, graph: nx.Graph, query_entities: List[str]
) -> RelevantData:
    relevant_data = RelevantData()

    for query_entity in query_entities:
        for node in graph.nodes():
            if query_entity.lower() in str(node).lower():
                self._process_matching_node(node, graph, relevant_data)

    return relevant_data
```

The local search context is structured to provide the LLM with a comprehensive view of the relevant information extracted from the knowledge graph. It takes the entities matched from the query, their relationships found in the graph, and supporting text chunks, and formats them into a clear template. The entities section lists all matched nodes from the graph that were relevant to the query. The relationships section details how these entities are connected to each other through the graph edges. Finally, the supporting text section includes the actual document text chunks associated with these entities and relationships. This structured context helps the LLM understand both the semantic relationships between entities as well as their textual descriptions, allowing it to generate more accurate and contextually appropriate responses.

``` python
LOCAL_SEARCH_CONTEXT = """
Relevant Entities: {entities}

Relationships:
{relationships}

Supporting Text:
{text_chunks}
"""
```

## Global Search

Global search uses a map-reduce approach to analyze document communities. It first retrieves all community summaries for a document, then in the map phase, it generates intermediate answers for each community. In the reduce phase, it combines these answers into a coherent response.

``` python
def global_search(self, query: str, doc_id: int, limit: int = 5) -> str:
    session = self.SessionLocal()
    try:
        communities = (
            session.query(Community)
            .filter(Community.document_id == doc_id)
            .all()
        )

        intermediate_answers = []
        print("Performing map phase over communities.")

        for community in tqdm(communities):
            answer = self._generate_llm_response(
                GLOBAL_SEARCH_COMMUNITY,
                f"Community Summary:\n{community.content}\n\nQuery: {query}",
                temp=0.7,
            )
            if answer and "No relevant information found" not in answer:
                intermediate_answers.append(answer)

        print("Performing reduce phase over community answers.")
        return self._generate_llm_response(
            GLOBAL_SEARCH_COMBINE,
            f"Query: {query}\n\nAnswers to combine:\n{' '.join(intermediate_answers)}",
            temp=0.7,
        )
    finally:
        session.close()
```

## Naive RAG

The naive search method implements a traditional RAG approach using hybrid search (combining vector similarity and keyword matching). It retrieves relevant text chunks using the hybrid search function, combines them into a context string, and generates a response using the LLM. This method is simpler than the graph-based approaches but can be effective for straightforward queries.

``` python
def naive_search(self, query: str, limit: int = 5) -> str:
    from tiny_graphrag.search import hybrid_search
    search_results = hybrid_search(query, limit=limit, engine=self.engine)
    context = "\n\n".join([result.content for result in search_results])
    return self._generate_llm_response(
        NAIVE_SEARCH_RESPONSE,
        f"Context:\n{context}\n\nQuery: {query}"
    )
```

The hybrid search implementation combines two different search strategies: semantic search using vector embeddings and traditional keyword-based search. This is implemented in a single SQL query that leverages PostgreSQL's pgvector extension for vector similarity and full-text search capabilities. Let's look at the core query from the `search.py` file:

``` sql
WITH semantic_search AS (
    SELECT id, content, document_id,
           RANK () OVER (ORDER BY embedding <=> (:embedding)::vector) AS rank
    FROM document_chunks
    ORDER BY embedding <=> (:embedding)::vector
    LIMIT 20
),
keyword_search AS (
    SELECT id, content, document_id,
           RANK () OVER (ORDER BY ts_rank_cd(to_tsvector('english', content), query) DESC)
    FROM document_chunks, plainto_tsquery('english', :query) query
    WHERE to_tsvector('english', content) @@ query
    ORDER BY ts_rank_cd(to_tsvector('english', content), query) DESC
    LIMIT 20
)
SELECT
    COALESCE(semantic_search.document_id, keyword_search.document_id) AS document_id,
    COALESCE(semantic_search.content, keyword_search.content) AS content,
    COALESCE(1.0 / (:k + semantic_search.rank), 0.0) +
    COALESCE(1.0 / (:k + keyword_search.rank), 0.0) AS score
FROM semantic_search
FULL OUTER JOIN keyword_search ON semantic_search.id = keyword_search.id
ORDER BY score DESC
LIMIT :limit
```

The query uses Common Table Expressions (CTEs) to perform two parallel searches. The first CTE, semantic_search, uses pgvector's cosine similarity operator \<=\> to find documents whose embeddings are closest to the query embedding. It ranks results based on vector similarity and selects the top 20 matches. The second CTE, keyword_search, uses PostgreSQL's full-text search capabilities with ts_rank_cd to find and rank documents based on keyword matches, also selecting the top 20 results.

The main SELECT statement then combines these results using a FULL OUTER JOIN, which ensures we capture matches from both search methods even if a document only appears in one of them. This is where the Reciprocal Rank Fusion comes into play. The formula is implemented in the scoring formula:

``` sql
COALESCE(1.0 / (:k + semantic_search.rank), 0.0) +
COALESCE(1.0 / (:k + keyword_search.rank), 0.0) AS score
```

The RRF algorithm combines multiple ranked lists by giving each item a score based on its position in each list. The constant k (default 60) acts as a smoothing factor that mitigates the impact of high rankings in individual result lists. For each document, its final score is the sum of the reciprocal of (k + rank) from each source. The COALESCE function handles cases where a document appears in only one of the result sets, defaulting to 0 for missing rankings.

This fusion approach is an improvement because it compoensates for the strengths and weaknesses of both search methods. Semantic search can capture documents with similar meaning even when they use different words, while keyword search ensures exact matches aren't missed. The RRF algorithm provides a principled way to combine these signals, giving higher weight to documents that rank well in both searches while still preserving strong single-source matches. The results are finally ordered by the combined RRF score in descending order and limited to the requested number of results. This hybrid approach provides more robust search results than either semantic or keyword search alone, particularly for queries where the relevant information might be expressed in various ways throughout the document.

## Putting It All Together

Ok we have everything implemented. A basic CLI interface is provided in the reference implementation to run the pipeline the terminal. We also provide a Docker Compose setup for running the pgvector database in an ephemeral Docker container.

To setup the database and ingest a text document, run the following:

```
graphrag init
graphrag build data/Barack_Obama.txt
```

This will ingest into Postgres and local graph file in the `./graphs` folder. To search use the `query` command.

```
graphrag query local --graph graphs/1_graph.pkl "What did Barack Obama study at Columbia University?"
graphrag query global --graph graphs/1_graph.pkl --doc-id 1 "What are the main themes of this document?"
graphrag query naive "What efforts did Barack Obama due to combat climate change?"
```

Under the hood this is using the toplevel Python API which we built above. To call it from Python import the `tiny_graphgrag` namespace and invoke the toplevel functions.

``` python
from tiny_graphrag import QueryEngine, store_document, init_db

# Initialize the database
engine = init_db()

# Process and store a document
doc_id, graph_path = store_document(
    filepath="data/Barack_Obama.txt",
    title="Barack Obama Wikipedia",
    engine=engine
)

# Create query engine with the database connection
query_engine = QueryEngine(engine)

result = query_engine.local_search(
    query="What did Barack Obama study at Columbia University?",
    graph_path=graph_path
)
print("Local Search Result:", result)

result = query_engine.global_search(
    query="What are the main themes of this document?",
    doc_id=doc_id
)
print("Global Search Result:", result)

result = query_engine.naive_search(
    query="What efforts did Barack Obama due to combat climate change?"
)
print("Naive Search Result:", result)
```

## Custom Entity Types

For more specialized use cases, we can overload the entity and relation types.

``` python
from tiny_graphrag import QueryEngine, store_document, init_db

engine = init_db("postgresql://admin:admin@localhost:5432/tiny-graphrag")

# Define custom geographic entity types
geography_entity_types = [
    "City",
    "Country",
    "Landmark",
    "River",
    "Mountain",
    "Ocean"
]

# Define custom relation types with type constraints
geography_relation_types = {
    "capital_of": {
        "allowed_head": ["City"],
        "allowed_tail": ["Country"]
    },
    "flows_through": {
        "allowed_head": ["River"],
        "allowed_tail": ["City", "Country"]
    },
    "located_in": {
        "allowed_head": ["City", "Landmark"],
        "allowed_tail": ["Country"]
    },
    "borders": {
        "allowed_head": ["Country"],
        "allowed_tail": ["Country", "Ocean"]
    }
}

# Process document with custom types
doc_id, graph_path = store_document(
    filepath="data/geography.txt",
    title="World Geography",
    engine=engine,
    entity_types=geography_entity_types,
    relation_types=geography_relation_types
)

query_engine = QueryEngine(engine)

result = query_engine.local_search(
    query="What rivers flow through Paris?",
    graph_path=graph_path
)
print("Geographic Local Search:", result)

result = query_engine.global_search(
    query="What are the major geographical features of Western Europe?",
    doc_id=doc_id
)
print("Geographic Global Search:", result)
```

## Part Two

In [Part 2](graphrag2.md), we'll go beyond the minimal 1000 line toy impelmentation, and add several significant improvements to our toy implementation that would make it more suitable for production use. The first major change would be replacing our serialized networkx graphs with a proper graph database (Memgraph). Graph databases are specifically optimized for relationship-based queries and can handle large-scale graphs much more efficiently than our current pickle file-based approach. They also provide built-in support for graph embeddings (node2vec), which allow us to combine structural and semantic similarity in our queries. This is particularly useful for finding related entities that might not be directly connected but share similar patterns of relationships.

The second addition will address the computational expense of our current global search implementation. Instead of using a map-reduce approach that requires n+1 LLM calls, we will use top-k clustering on the community summaries. This would group similar communities together based on their vector embeddings. We will also implement a hierarchical clustering approach (like the original GraphRAG reference implementation) that creates a multi-level summary structure, enabling us to start with high-level summaries and drill down only when necessary.

Then we will focus on improving entity resolution and deduplication. Our current implementation treats entities as exact string matches, which can miss connections between different references to the same entity (e.g., "Barack Obama" vs. "President Obama" vs. "Obama"). We can add vector similarity comparison for entity matching, using the embeddings of entity mentions along with their surrounding context to identify when different strings refer to the same entity.

## How Does It Perform

GraphRAG can theoretically outperform naive RAG in certain scenarios, however both approaches suffer from fundamental limitations that make them quite fragile to use in practice. Naive RAG's effectiveness heavily depends on careful tuning of chunking strategies and embedding parameters - optimizations that often need to be discovered empirically and rarely generalize well across different domains or document types. GraphRAG attempts to address these limitations by introducing structured knowledge representation, but it essentially trades one set of hard problems for another. While it can capture relationships that naive RAG might miss, it critically depends on accurate entity and relation extraction, which is itself an unsolved problem. The quality of the final results is highly sensitive to the accuracy of these extraction steps, and errors tend to compound throughout the pipeline.

Both approaches require what amounts to a sequence of statistical "miracles" to align perfectly in order to produce accurate and contextually appropriate answers. The chunking must be just right, the embeddings must capture the relevant semantic relationships, the LLM must interpret the context correctly, and in GraphRAG's case, the entity and relation extraction must also be accurate. The failure of any single component can lead to irrelevant or misleading responses. GraphRAG is not uniformly "better" but it trades an embedding simliarity problem for a entity/relation extration problem, which may be easier to solve in some domains. The global search for GraphRAG is also significantly more computationally expensive and requires n+1 (where n is the communities in the graph) calls to LLMs to perform it's map-reduce operation, which for large documents could be considerable.

For anyone considering implementing either approach, it's crucial to understand that end users of this system would need to accept a high degree of potential inaccuracy and out-of-context answers when relying on these chains of black box models. The current state of RAG, whether graph-based or naive, is not sufficiently reliable for production systems where accuracy and consistency are important. While these approaches can be interesting for experimental or research purposes, they are not yet mature enough to serve as dependable solutions for production information retrieval systems.

GraphRAG is not some algorithmic panacea it seems to often be marketed as. It does not "solve hallucinations" or fix the "pre-training overflow problem" that all RAG approachs that use a language model as final synthesizer suffer from. However it might prove more useful in some specific low-risk domains and may show improvement over a naive RAG implementation. In conclusion, if you're looking for a way to turn perfectly good documents into unreliable answers while burning through GPU credits, both naive RAG and GraphRAG are excellent choices.
