---
date: "2024-11-02"
tags: ["ai", "humor"]
url: "https://www.stephendiehl.com/posts/rag/"
---

# The Limitations of RAG

Retrieval-Augmented generation or RAG is the future of enterprise search. In short it's the art of grinding your documents into a fine, high-dimensional [vector slurry](https://youtu.be/J-QeTbmchvQ?t=95), sifting through the high-dimensional slurry for chunks, and then piping the chunks through a non-deterministic large language model, and praying that something resembling useful truth comes out the other end. It's like making a smoothie with your corporate knowledge base, except instead of getting a refreshing drink, you often end up with... well, let's explore that.

Picture this: you take your perfectly organized documents, lovingly crafted by humans who understood context, domain terminology, and nuance, and throw them into the BERT Vector Blender Transformer™. What comes out? A chunky soup of floating point numbers that allegedly capture the "essence" of your content. It's somewhat like trying to describe the Mona Lisa's smile using the frequency coefficients of the Fourier transform of a compressed JPEG of the painting. Technically possible, but practically hard.

Then you throw those vectors into a vector database and use HNSW to search through the high-dimensional space, hoping your nearest neighbor search actually finds semantically similar content rather than just unrelatedly close vectors. Then for RAG to work correctly, we need a sequence of miracles to occur:

1.  Your chunking algorithm needs to perfectly slice your documents
2.  Your embedding model needs to capture the true semantic meaning
3.  The embedding of the question and the retrieved chunks need to be similar
4.  Your vector search needs to find the actually relevant bits
5.  Your LLM needs to weave it all together coherently with only the in-context information

It's like playing a game of telephone, except every player is a bullshit artist who has been fine-tuned to lie instead of saying they don't know. Each component introduces its own fun flavor of randomness. The whole pipeline is like trying to hit a bullseye while riding a unicycle on a boat during a storm. Sure it can randomly work, you might even get lucky sometimes, but good luck reproducing that result!

And then there's the shallow matching problem. Your RAG system might think it's being helpful by retrieving content about "apple pie" when you ask about "Apple's corporate structure" because hey, they both contain "apple," right? It's like having a particularly enthusiastic but somewhat deaf assistant who, when you ask about "quantum computing," eagerly provides you with information about "quantum healing crystals" because the word "quantum" is similar. The coherence of these responses often resembles a college freshman's essay written ten minutes before deadline – technically all the words are there, but they're arranged with all the grace of a drunk giraffe attempting ballet.

Even with all your carefully retrieved context, LLMs still love to dance with fiction. The retrieved context is less like a strict set of facts and more like loose suggestions that the model may or may not choose to acknowledge, and there's no guarantee that the model will sample from the given context or from its general crawl of the dark corners of the internet. You might feed it your entire company handbook, only to have it confidently declare that your CEO is a time-traveling penguin who invented post-it notes. Or it might regurgitate the policies and procedures that don't exist, or mix them with the policies of a competitor, or tell your [employees to break the law](https://apnews.com/article/new-york-city-chatbot-misinformation-6ebc71db5b770b9969c906a7ee4fae21).

And then there's the structured data problem. Ask your RAG system to find the average salary across departments while excluding temporary contractors, and it'll cheerfully write you a novel about "the rich tapestry of compensation across our organizational landscape." It's like asking a literature professor to do your taxes - they'll spend six pages waxing lyrical about the metaphorical significance of your W-2 and conclude that your 401k represents humanity's eternal struggle against desire in a post-modern world. These systems can eloquently write a haiku about every number in your database but ask them to perform basic aggregations and suddenly they're a philosophy major who took "Intro to Statistics" as an elective: "But what does 'average' really mean in a post-truth world?" they'll muse, while completely ignoring your desperate pleas for a simple mean calculation.

Now basically none of this matters, because at least at this point in time, most AI projects are about producing slick demos for C-level executives who were drunk on the paradigm shifting power of NFTs twelve months ago, and not about solving real problems. And RAG pipeline are amazing for demos because they give the *appearance of working* but without actually *having* to work. Which is the best kind of engineering.
