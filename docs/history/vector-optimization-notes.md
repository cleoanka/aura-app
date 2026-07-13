**1. Storage & Search (Scale: 1k–50k chunks)**
* **Choice:** Brute-force exact search via `sqlite-vec` (a modern Rust SQLite extension). 
* **Crossover Point:** ANN (like HNSW/usearch) is overkill here. 50k chunks × 384 dims × 4 bytes = ~76.8 MB. Brute-force exact matching over 76MB takes single-digit milliseconds in Rust. ANN only becomes strictly necessary for latency beyond 250k–500k chunks, at the cost of index bloat and recall drops.

**2. Normalization & Similarity**
* **Confirmation:** Yes, L2-normalizing vectors at index time and calculating dot-product at query time perfectly equals Cosine Similarity and is computationally cheaper.
* **Pitfalls:** Guard against division-by-zero (NaNs) on empty/malformed chunks. Ensure strict endianness handling (use native-endian `f32` slice-to-byte conversion) when casting vectors to SQLite `BLOB`s.

**3. Candle e5 on Apple Silicon**
* **Metal vs CPU:** For a small model like `e5-small` (~133MB), use **CPU + Apple Accelerate** feature in Rust. Metal (MPS) introduces kernel dispatch overhead making single-query latency *slower*. Only use MPS if batching >32 chunks during initial ingestion.
* **Download/UX:** Use the `hf-hub` crate. It natively handles caching to `~/.cache/huggingface/hub`. Load the model on a background thread, using an intercepting `std::io::Read` wrapper to stream download progress bytes over Tauri IPC (`app.emit()`) to avoid UI beachballing on first run.
* **Prefix Correctness:** You **must** prepend `"passage: "` to every codebase chunk before embedding, and `"query: "` to the user's search string. Omitting this severely degrades E5's semantic clustering.

**4. Incremental Embedding**
* **Hashing Strategy:** Compute a fast hash (e.g., BLAKE3) of the *chunk content*, not the file. 
* **Schema:** Use a deduplication schema: `chunks (chunk_hash PK, vector, text)` and `file_map (file_path, chunk_index, chunk_hash)`. This prevents re-embedding unchanged code and instantly deduplicates identical files/boilerplate across the workspace.

**5. Top 3 Mistakes**
* **Blind Text Splitting:** Chunking code arbitrarily by character count destroys context. Use `tree-sitter` to chunk logically by functions/structs, and inject the file path/signature into the text payload so the embedding captures local context.
* **Raw Score Merging:** FTS5 BM25 scores and Vector Cosine scores are mathematically incompatible. You must use true Reciprocal Rank Fusion (`score = 1 / (k + rank)`) rather than attempting to normalize and add their raw scores.
* **Main Thread Blocking:** Instantiating the model or running inference on the main Tauri thread stalls the macOS event loop. Push all Candle operations to a dedicated MPSC worker thread.
