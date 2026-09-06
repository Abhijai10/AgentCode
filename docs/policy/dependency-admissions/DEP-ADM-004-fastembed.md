# DEP-ADM-004 — fastembed 6.0.1

Need:
  Batch 7 needs trained, local dense embeddings for durable semantic memory and hybrid retrieval.

Proposed dependency:
  fastembed 6.0.1 from crates.io, pinned in Cargo.lock and used only by ac-context.

Existing alternative check:
  Existing FTS and code-intelligence retrieval are lexical/structural. They cannot retrieve paraphrased project decisions without shared terms, and a hash or token-count vector is expressly insufficient.

License:
  Apache-2.0. Inspected `/Users/abhijairaghuvanshi/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/fastembed-6.0.1/LICENSE`; permissive under the OSS license matrix.

Security status:
  Cargo registry resolution is lockfile-pinned. The dependency performs local ONNX inference; model retrieval is invoked only by explicit local-model activation, never daemon startup. Model artifacts are cached locally.

Maintenance state:
  fastembed is a maintained Rust local-embedding library with supported MiniLM and BGE models.

Runtime/bundle cost:
  Adds ONNX Runtime and tokenizer dependencies. V1 selects `all-MiniLM-L6-v2` (384 dimensions) with a two-thread inference cap for the local-first macOS target.

Install/postinstall behavior:
  Cargo builds Rust crates only. The model download is explicit through `MemoryService::load_local_embeddings`; no global installer or source upload is used.

Why existing components are insufficient:
  FTS, symbol graph, and exact memory lookup remain useful retrieval signals but cannot produce real dense semantic similarity.

License gate result: PASS
Admission decision: ADMIT
Reviewer + date + evidence refs:
  Codex, 2026-08-25. Evidence: Cargo.lock; inspected license above; focused `cargo check -p ac-context`.
