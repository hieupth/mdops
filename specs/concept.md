mdtool — Architecture v0.2

Version: v0.2
Status: Draft for Implementation

⸻

1. Purpose & Architecture Overview

mdtool is a Rust-native Markdown operations engine for AI systems.

Central rule: all meaningful Markdown operations must flow through a unified block tree — a single recursive data structure where the document itself is the root block, headings create container blocks, and every content element is a block node in the tree.

This document defines the domain model, block operations, processing pipeline, interfaces, and testing rules for an implementation built on this principle.

High-level shape:

              +----------------------------------+
              | Users / Agents / CI / MCP Tools  |
              +----------------+-----------------+
                               |
                               v
              +----------------------------------+
              |      API / CLI / Tool Layer      |
              | serde schemas + handlers          |
              +----------------+-----------------+
                               |
                               v
+-------------------------------------------------------------------+
|                    Application Services Layer                      |
| parse | read | write | normalize | validate                          |
+-------------------------------+-----------------------------------+
                                |
                                v
+-------------------------------------------------------------------+
|                         Core Domain Layer                          |
| DocumentBlock | Block tree | BlockSelector | Diagnostics           |
| DiffSummary | AsciiLayoutModel | Policies | Errors                |
+-------------------------------+-----------------------------------+
                                |
                                v
+-------------------------------------------------------------------+
|                         Infrastructure Layer                       |
| Parser adapter | File adapter | Diff adapter                    |
| Temp artifact management | Policy guard | Logging                |
+-------------------------------------------------------------------+

⸻

Technology Choices

- **Language:** Rust (latest stable edition). Compiled, zero-runtime-dependency single binary. Source code protection via native compilation.
- **Build:** Cargo workspace — `mdtool-core` (domain + ops), `mdtool-cli`, `mdtool-mcp`, `mdtool` (binary crate)
- **Data Modeling:** domain entities → Rust structs + enums; serialization → serde + serde_json; JSON Schema generation → schemars
- **Parser:** comrak — GFM-compliant AST with source position tracking (line/column/byte offsets)
  - mdtool will **not** expose comrak node types as public API. The parser is an infrastructure dependency; mdtool converts output into its own stable domain model.
- **CLI:** clap v4 with derive macros — type-safe argument parsing, auto-generated help
- **MCP:** rmcp crate — Model Context Protocol server implementation (stdio transport)
- **Error handling:** thiserror (library errors with typed diagnostics) + anyhow (application-level error chaining)
- **Logging:** tracing crate — structured, async-aware diagnostic logging. No output at default level; debug payloads available via `RUST_LOG=mdtool=debug`.
- **Testing:** built-in `#[test]` + proptest (property-based testing) + insta (snapshot/golden tests)

⸻

2. Domain Model: Unified Block Tree

2.1 Core Principle

A Markdown document is modeled as a single recursive tree of blocks. The document itself is the root block. Headings create container blocks (SectionBlock). Content elements (paragraphs, tables, lists, code fences) are child blocks within their nearest ancestor section. Every block has an ID, a parent reference, and a list of children — enabling uniform traversal, addressing, and mutation.

Example — this markdown:

```markdown
# Architecture

Overview text.

## Parser

Body text.

| Module | Role |
|--------|------|
| builder | constructs tree |

- item A
- item B

# Risks

Some risk text.
```

Produces this tree:

```
DocumentBlock (root)
├── SectionBlock (# Architecture, level=1)
│   ├── ParagraphBlock ("Overview text.")
│   └── SectionBlock (## Parser, level=2)
│       ├── ParagraphBlock ("Body text.")
│       ├── TableBlock (2x2, alignments=[none, none])
│       └── ListBlock (unordered, tight)
│           ├── ListItemBlock ("item A")
│           └── ListItemBlock ("item B")
└── SectionBlock (# Risks, level=1)
    └── ParagraphBlock ("Some risk text.")
```

2.2 Block Base Struct

```rust
/// Core data carried by every block node in the document tree.
#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    pub id: BlockId,                        // newtype around u32
    pub line_range: LineRange,
    pub parent_id: Option<BlockId>,
    pub children_ids: Vec<BlockId>,
    pub byte_range: Option<ByteRange>,
}

impl Block {
    pub fn has_children(&self) -> bool { !self.children_ids.is_empty() }
}
```

Common fields:
- `id` — unique identifier (newtype `BlockId(u32)`, assigned sequentially 0, 1, 2, …). Stable within a single parse.
- `line_range` — 1-based inclusive line range [start, end] in source text.
- `parent_id` — ID of the parent block, None for root.
- `children_ids` — ordered list of child block IDs.
- `byte_range` — half-open byte range [start, end) in source text.

Rust does not have inheritance. Instead, mdtool uses a **flat enum + struct composition** pattern: each concrete block type owns a `Block` for shared fields, and adds its own typed fields. The `BlockNode` enum provides type-safe dispatch over all variants.

2.3 DocumentBlock

```rust
/// Root block — represents the entire parsed document.
#[derive(Debug, Clone)]
pub struct DocumentBlock {
    pub block: Block,
    pub source_text: String,
    pub normalized_text: String,
    pub line_starts: Vec<usize>,
    pub block_by_id: HashMap<BlockId, BlockNode>,
    pub diagnostics: Vec<Diagnostic>,
    pub metadata: HashMap<String, String>,
}
```

`DocumentBlock` replaces both the old `Document` and `HeadingNode` concepts. It is the single entry point for all tree operations. `block_by_id` provides O(1) lookup of any block in the tree by its ID.

2.4 Block Type Catalog

2.4.1 BlockNode Enum — Type-Safe Dispatch

```rust
/// Sum type over all concrete block variants. No inheritance — composition + enum.
#[derive(Debug, Clone, PartialEq)]
pub enum BlockNode {
    Document(DocumentBlock),
    Section(SectionBlock),
    Blockquote(BlockquoteBlock),
    List(ListBlock),
    ListItem(ListItemBlock),
    Table(TableBlock),
    Paragraph(ParagraphBlock),
    Fence(FenceBlock),
    CodeBlock(CodeBlock),
    ThematicBreak(ThematicBreakBlock),
    Html(HtmlBlock),
    LinkRefDef(LinkRefDefBlock),
    Unknown(UnknownBlock),
}

impl BlockNode {
    pub fn block(&self) -> &Block { /* delegate to inner .block */ }
    pub fn block_mut(&mut self) -> &mut Block { /* delegate to inner .block */ }
}
```

2.4.2 Section Blocks — heading-delimited containers

```rust
#[derive(Debug, Clone, PartialEq)]
pub struct SectionBlock {
    pub block: Block,
    pub level: u8,                                 // 1–6
    pub title: String,
    pub slug: String,                              // URL-safe title
    pub path: String,                              // canonical path e.g. "/Architecture/Parser"
    pub ordinal: u32,                              // disambiguates repeated sibling headings
    pub variant: HeadingVariant,                   // Atx | Setext
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum HeadingVariant { Atx, Setext }
```

SectionBlock is always a container. Its children are the blocks that appear between this heading and the next heading of same or shallower level. This includes paragraphs, tables, lists, sub-sections, and any other block.

2.4.3 Container Blocks — can hold child blocks

```rust
#[derive(Debug, Clone, PartialEq)]
pub struct BlockquoteBlock {
    pub block: Block,
    // Children are blocks inside the quote, stored in block.children_ids
}

#[derive(Debug, Clone, PartialEq)]
pub struct ListBlock {
    pub block: Block,
    pub ordered: bool,                             // true = ordered (1. 2. 3.)
    pub marker: char,                              // the actual marker character used
    pub tight: bool,                               // tight = no blank lines between items
}

#[derive(Debug, Clone, PartialEq)]
pub struct ListItemBlock {
    pub block: Block,
    pub checked: Option<bool>,                     // None = normal, Some(true) = [x], Some(false) = [ ]
    pub order: Option<u32>,                        // numeric value for ordered list items
}

#[derive(Debug, Clone, PartialEq)]
pub struct TableBlock {
    pub block: Block,
    pub alignments: Vec<Alignment>,
    pub header_row: Vec<String>,
    pub body_rows: Vec<Vec<String>>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Alignment { Left, Center, Right, None }
// Note: Alignment::None is a GFM convention for "no alignment specified".
// Pattern matching requires fully-qualified path to avoid confusion with Option::None.
```

2.4.4 Leaf Blocks — no children, terminal content

```rust
#[derive(Debug, Clone, PartialEq)]
pub struct ParagraphBlock { pub block: Block, pub raw_text: String }

#[derive(Debug, Clone, PartialEq)]
pub struct FenceBlock {
    pub block: Block,
    pub language: Option<String>,
    pub info_string: Option<String>,
    pub raw_text: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CodeBlock { pub block: Block, pub raw_text: String }

#[derive(Debug, Clone, PartialEq)]
pub struct ThematicBreakBlock { pub block: Block }

#[derive(Debug, Clone, PartialEq)]
pub struct HtmlBlock { pub block: Block, pub raw_text: String }

#[derive(Debug, Clone, PartialEq)]
pub struct LinkRefDefBlock {
    pub block: Block,
    pub label: String,
    pub destination: String,
    pub title: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct UnknownBlock { pub block: Block, pub raw_text: String }
```

2.5 Tree Construction Rules

Parsing pipeline:

Raw Markdown → normalize line endings → parse AST (comrak) → build flat block list → assign parent-child relationships → build heading tree → compute section ranges → assign paths/slugs → emit DocumentBlock

2.5.1 Heading Nesting

SectionBlocks form a tree based on heading levels:

```
# A → ## B → ### C → ## D → # E
```

builds:

```
SectionBlock(A, level=1)
├── SectionBlock(B, level=2)
│   └── SectionBlock(C, level=3)
└── SectionBlock(D, level=2)
SectionBlock(E, level=1)
```

Rule: each heading attaches beneath the nearest ancestor with a shallower level. All top-level headings (level=1) are children of DocumentBlock.

2.5.2 Edge Cases

| Case | Behavior |
|------|----------|
| Content before first heading | Children of DocumentBlock directly |
| Heading level jump (`# A` → `### C`) | Attach beneath nearest shallower ancestor; emit `HEADING_LEVEL_JUMP` diagnostic |
| Empty section (`# A` immediately followed by `# B`) | SectionBlock with no children |
| Setext heading (`text\n===`) | SectionBlock with `variant="setext"` |
| Duplicate heading titles | Same path; disambiguated by `ordinal` |
| Nested list (`- a\n  - b`) | ListItemBlock contains child ListBlock |
| Blank document (empty or whitespace only) | DocumentBlock with no children; no error |
| UTF-8 BOM prefix | Stripped during normalization before parsing |

2.5.3 Section Range Computation

For each SectionBlock:
- `line_range` spans from the heading line through all descendant content until the next sibling of same or shallower level.
- Content blocks within a section have their own `line_range` covering only their own lines.

Example:

```
1  # A          → A.line_range = 1..8
2  intro        → ParagraphBlock.line_range = 2..2
3  ## B         → B.line_range = 3..6
4  body         → ParagraphBlock.line_range = 4..4
5  ### C        → C.line_range = 5..6
6  nested       → ParagraphBlock.line_range = 6..6
7  ## D         → D.line_range = 7..8
8  tail         → ParagraphBlock.line_range = 8..8
```

2.6 Primitive Types

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[serde(transparent)]
pub struct BlockId(pub u32);  // serializes as plain number: 42, not {"BlockId": 42}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ByteRange {
    pub start: usize,  // inclusive
    pub end: usize,    // exclusive
}

impl ByteRange {
    pub fn length(&self) -> usize { self.end - self.start }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LineRange {
    pub start: usize,  // 1-based inclusive
    pub end: usize,    // 1-based inclusive
}

impl LineRange {
    pub fn line_count(&self) -> usize { self.end - self.start + 1 }
    pub fn contains(&self, line: usize) -> bool { self.start <= line && line <= self.end }
}
```

⸻

3. Block Operations

3.1 Block Addressing

Every block can be targeted by multiple addressing strategies, unified in a single selector model.

```rust
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
pub struct BlockSelector {
    pub id: Option<BlockId>,              // direct block ID
    pub path: Option<String>,             // canonical section path (e.g., "/Architecture/Parser")
    pub block_index: Option<usize>,       // Nth child of resolved parent (0-based)
    pub block_type: Option<String>,       // filter by type (e.g., "fence", "table")
    pub title: Option<String>,            // match SectionBlock title
    pub level: Option<u8>,                // match SectionBlock level
    pub allow_first_match: bool,          // resolve ambiguity by taking first match
}
```

Resolution order: by id → by path → by query fields (title, level, block_type).

Resolution rules:
- Empty selector (all fields None) → resolves to DocumentBlock (root)
- 0 matches → `BlockNotFoundError`
- 1 match → return Block
- >1 matches → `AmbiguousBlockError` unless `allow_first_match == true`

3.2 Tree Traversal

```rust
pub fn walk(block: &BlockNode, doc: &DocumentBlock) -> impl Iterator<Item = &BlockNode>;
/// Depth-first traversal of block and all descendants.

pub fn find_by_id(doc: &DocumentBlock, id: BlockId) -> Option<&BlockNode>;
/// O(1) lookup via doc.block_by_id.

pub fn find_children(doc: &DocumentBlock, id: BlockId) -> Vec<&BlockNode>;
/// Return ordered list of child blocks.

pub fn find_ancestors(doc: &DocumentBlock, id: BlockId) -> Vec<&BlockNode>;
/// Walk up parent chain to root.

pub fn find_by_type(doc: &DocumentBlock, block_type: &str) -> Vec<&BlockNode>;
/// Return all blocks of a given type (e.g., all FenceBlock).

pub fn find_by_path(doc: &DocumentBlock, path: &str) -> Option<&SectionBlock>;
/// Resolve canonical path to SectionBlock.
```

3.3 Read Operations

```rust
pub fn read_outline(doc: &DocumentBlock, max_depth: u8) -> Vec<OutlineEntry>;
/// Traverse all SectionBlock nodes up to max_depth.

pub fn read_block(doc: &DocumentBlock, selector: &BlockSelector) -> Result<&BlockNode>;
/// Resolve selector → return single block.

pub fn read_block_text(doc: &DocumentBlock, selector: &BlockSelector) -> Result<String>;
/// Resolve selector → return block's source text (extracted via line_range).

pub fn read_block_tree(doc: &DocumentBlock, selector: &BlockSelector, depth: i32) -> Result<BlockTree>;
/// Resolve selector → return block and its subtree as nested structure. depth=-1 means unlimited.

pub fn read_block_children(doc: &DocumentBlock, selector: &BlockSelector) -> Result<Vec<&BlockNode>>;
/// Resolve selector → return ordered children of the matched block.

pub fn read_blocks_by_type(doc: &DocumentBlock, block_type: &str) -> Vec<&BlockNode>;
/// Return all blocks matching the given type across the entire tree.
```

3.3.1 AI Context Format

For AI consumption via MCP tools, block data must be serialized in a token-efficient format. The key principle: return structure, not full text.

read_outline response (compact):
```json
{
  "sections": [
    {"id": "b_1", "level": 1, "title": "Architecture", "path": "/Architecture"},
    {"id": "b_3", "level": 2, "title": "Parser", "path": "/Architecture/Parser"}
  ]
}
```

read_block_tree response (compact — include_text=False):
```json
{
  "id": "b_1", "type": "section", "title": "Architecture", "level": 1,
  "children": [
    {"id": "b_2", "type": "paragraph"},
    {"id": "b_3", "type": "section", "title": "Parser", "level": 2, "children": [
      {"id": "b_4", "type": "paragraph"},
      {"id": "b_5", "type": "table", "rows": 3, "cols": 2},
      {"id": "b_6", "type": "list", "items": 2}
    ]}
  ]
}
```

read_block response (full — include_text=True):
```json
{
  "id": "b_5", "type": "table",
  "line_range": [7, 10],
  "alignments": ["none", "none"],
  "header_row": ["Module", "Role"],
  "body_rows": [["builder", "constructs tree"]]
}
```

include_text parameter controls whether raw_text / body_rows are included. Default depends on operation: outline and tree → False (structure only), single block → True (full data).

3.4 Search Operations

```rust
pub fn search_blocks(
    doc: &DocumentBlock,
    query: &str,
    selector: Option<&BlockSelector>,
    case_sensitive: bool,
) -> Vec<SearchMatch>;

pub struct SearchMatch {
    pub block_id: BlockId,
    pub matched_text: String,
}
```

Search for text across blocks. `query` is a plain text search term. Optionally scope to a subtree via selector. Returns matching block IDs and matched line text. Case-insensitive by default.

This is critical for AI workflows where agents need to locate content before editing. Without search, an AI would need to read the entire document tree and scan manually — expensive in tokens and unreliable.

3.5 Write Operations

All write operations follow the **patch-reparse-validate** cycle:
1. Resolve selector → find target block and its line_range.
2. Compute text patch against normalized_text.
3. Reparse the full text to produce a new DocumentBlock.
4. Validate the new tree and collect diagnostics.
5. Return (new_doc, diff_summary, diagnostics).

This guarantees structural consistency after every mutation.

3.5.1 Generic Block Mutations

```rust
/// Replace the matched block's source text. If container, replaces entire subtree.
pub fn replace_block(doc: &DocumentBlock, selector: &BlockSelector, new_text: &str) -> Result<(DocumentBlock, DiffSummary, Vec<Diagnostic>)>;

/// Insert a new block immediately before the matched block.
pub fn insert_block_before(doc: &DocumentBlock, selector: &BlockSelector, new_text: &str) -> Result<(DocumentBlock, DiffSummary, Vec<Diagnostic>)>;

/// Insert a new block immediately after the matched block.
pub fn insert_block_after(doc: &DocumentBlock, selector: &BlockSelector, new_text: &str) -> Result<(DocumentBlock, DiffSummary, Vec<Diagnostic>)>;

/// Append a new block as the last child of the matched container block.
pub fn append_child(doc: &DocumentBlock, selector: &BlockSelector, new_text: &str) -> Result<(DocumentBlock, DiffSummary, Vec<Diagnostic>)>;

/// Delete the matched block and its entire subtree.
pub fn delete_block(doc: &DocumentBlock, selector: &BlockSelector) -> Result<(DocumentBlock, DiffSummary, Vec<Diagnostic>)>;

/// Move the matched block (and subtree) to a new parent. If index is None, append at end.
pub fn move_block(doc: &DocumentBlock, selector: &BlockSelector, target_parent_id: BlockId, index: Option<usize>) -> Result<(DocumentBlock, DiffSummary, Vec<Diagnostic>)>;

/// Ensure a SectionBlock exists at the given path. Create with specified heading level if missing.
pub fn ensure_section(doc: &DocumentBlock, path: &str, heading_level: u8) -> Result<(DocumentBlock, DiffSummary, Vec<Diagnostic>)>;
```

3.5.2 Semantic Helpers

These are higher-level operations that understand block internals, so AI tools don't need to construct raw markdown. Each semantic helper translates into a generic mutation internally but provides a type-safe, error-resistant interface.

```rust
/// Change the heading title of a SectionBlock. Preserves heading level and child content.
pub fn rename_section(doc: &DocumentBlock, selector: &BlockSelector, new_title: &str) -> Result<(DocumentBlock, DiffSummary, Vec<Diagnostic>)>;

/// Change a SectionBlock's heading level (e.g., ## → ###). Emits diagnostic if invalid nesting.
pub fn change_heading_level(doc: &DocumentBlock, selector: &BlockSelector, new_level: u8) -> Result<(DocumentBlock, DiffSummary, Vec<Diagnostic>)>;

/// Add a row to a TableBlock. position: append | prepend | index.
pub fn add_table_row(doc: &DocumentBlock, selector: &BlockSelector, row: &[String], position: InsertPosition) -> Result<(DocumentBlock, DiffSummary, Vec<Diagnostic>)>;

/// Update a single cell in a TableBlock. row and col are 0-based.
pub fn update_table_cell(doc: &DocumentBlock, selector: &BlockSelector, row: usize, col: usize, value: &str) -> Result<(DocumentBlock, DiffSummary, Vec<Diagnostic>)>;

/// Remove a row from a TableBlock. row is 0-based index.
pub fn remove_table_row(doc: &DocumentBlock, selector: &BlockSelector, row: usize) -> Result<(DocumentBlock, DiffSummary, Vec<Diagnostic>)>;

/// Toggle a ListItemBlock's checked state: [ ] → [x] or [x] → [ ].
pub fn toggle_task(doc: &DocumentBlock, selector: &BlockSelector) -> Result<(DocumentBlock, DiffSummary, Vec<Diagnostic>)>;

/// Add an item to a ListBlock. If index is None, append at end.
pub fn add_list_item(doc: &DocumentBlock, selector: &BlockSelector, text: &str, index: Option<usize>) -> Result<(DocumentBlock, DiffSummary, Vec<Diagnostic>)>;

/// Remove the item at index from a ListBlock.
pub fn remove_list_item(doc: &DocumentBlock, selector: &BlockSelector, index: usize) -> Result<(DocumentBlock, DiffSummary, Vec<Diagnostic>)>;
```

Design principle: semantic helpers exist because they prevent the #1 AI error pattern — constructing malformed markdown. A typo in table formatting (misaligned `|`, missing `|`) corrupts the document. Semantic helpers guarantee structurally correct output.

3.5.3 Batch Operations

```rust
/// Apply one or more mutations in a single pass. Single-element = single edit.
/// Multi-element = batch with single patch-reparse cycle.
/// Rejects all if any operation fails resolution.
pub fn edit(
    doc: &DocumentBlock,
    operations: &[EditOp],
) -> Result<(DocumentBlock, DiffSummary, Vec<Diagnostic>)>;
```

`EditOp` is a tagged union (same as `EditOperation` enum in §6.6.2):
- Replace(selector, new_text)
- Insert(selector, new_text, position)
- Delete(selector)
- Move(selector, target_parent_id, index)
- EnsureSection(path, heading_level)
- Semantic operations (RenameSection, AddTableRow, ToggleTask, etc.)

Operations are applied in order. If any operation fails resolution, the entire batch is rejected.

3.5 Diff & Patch

```rust
#[derive(Debug, Clone, PartialEq)]
pub struct DiffSummary {
    pub changed: bool,
    pub changed_line_ranges: Vec<LineRange>,
    pub added_lines: usize,
    pub removed_lines: usize,
}

/// Replace lines [start, end] (1-based inclusive) with new_text.
pub fn patch_text(text: &str, line_range: LineRange, new_text: &str) -> String;

/// Patch text, reparse, return (new_full_text, new_doc, diff_summary).
pub fn patch_and_reparse(
    doc: &DocumentBlock,
    line_range: LineRange,
    new_text: &str,
) -> Result<(String, DocumentBlock, DiffSummary)>;
```

Patch workflow:
resolve selector → compute line_range of target → patch raw text slice → reparse → validate → return new DocumentBlock + DiffSummary + diagnostics

⸻

4. Processing Pipeline

4.1 Normalization

Operates on formatting concerns, not semantic content.

v0.2 operations: heading spacing, blank line after heading, collapse excessive blank lines, trim trailing whitespace outside protected fences, normalize final newline, preserve fenced code contents by default.

```rust
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
pub struct NormalizeOptions {
    pub ensure_single_trailing_newline: bool,  // default true
    pub trim_trailing_whitespace: bool,        // default true
    pub max_consecutive_blank_lines: usize,    // default 1
    pub preserve_fenced_code: bool,            // default true
    pub normalize_heading_spacing: bool,       // default true
}

impl Default for NormalizeOptions {
    fn default() -> Self { /* all true, max_consecutive_blank_lines = 1 */ }
}
```

4.2 Validation

Surfaces structural/operational issues without mutating the document.

v0.2 checks: heading level jumps, ambiguous duplicate paths, unclosed/malformed fences, empty sections, ASCII formatting issues, inconsistent replacement heading level.

Diagnostic codes:

`HEADING_LEVEL_JUMP` | `DUPLICATE_SECTION_PATH` | `BLOCK_NOT_FOUND` | `BLOCK_SELECTOR_AMBIGUOUS` | `UNCLOSED_FENCE` | `EMPTY_SECTION` | `ASCII_BORDER_MISMATCH` | `ASCII_MIXED_TABS_SPACES` | `INVALID_REPLACEMENT_HEADING`

Severity = `info` | `warning` | `error`

```rust
#[derive(Debug, Clone, PartialEq)]
pub struct Diagnostic {
    pub code: String,
    pub severity: Severity,
    pub message: String,
    pub line: Option<usize>,
    pub column: Option<usize>,
    pub line_range: Option<LineRange>,
    pub suggested_fix: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Severity { Info, Warning, Error }
```

4.3 Diagnostics Collection

Diagnostics are collected during parsing and mutation operations. Each DocumentBlock carries a `diagnostics` list. Operations may append new diagnostics (e.g., validation warnings after a replace). Diagnostics are never silently dropped.

⸻

5. ASCII Subsystem

5.1 Scope

Fenced blocks with info strings `ascii`, `box`, `diagram`. These are FenceBlock instances where `info_string in {"ascii", "box", "diagram"}`.

5.2 Detection

Block is ASCII candidate if `matches!(block, BlockNode::Fence(ref f)) && ["ascii", "box", "diagram"].contains(&f.info_string.as_deref().unwrap_or(""))`.

5.3 Internal Model

```rust
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AsciiMode { FormatOnly, RepairSafe, RepairAggressive }

#[derive(Debug, Clone)]
pub struct AsciiBlock {
    pub block_id: BlockId,
    pub info_string: String,
    pub indent: usize,
    pub lines: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct AsciiEdit {
    pub block_id: BlockId,
    pub changed: bool,
    pub before: Vec<String>,
    pub after: Vec<String>,
    pub diagnostics: Vec<Diagnostic>,
}
```

5.4 Formatting Rules

| Mode | Behavior |
|------|----------|
| format_only | Normalize tabs→spaces, trim trailing spaces, left-normalize indentation |
| repair_safe | + Align box right edges, normalize interior padding, harmonize borders |
| repair_aggressive | Reserved in schema; may remain partially implemented in v0.2 |

Critical constraint: if structural confidence is low, leave block unchanged and emit warning diagnostic. Never "hallucinate" a diagram repair.

5.5 Integration with Block Tree

ASCII operations target FenceBlock nodes within the tree. The operation reads the block's `raw_text`, processes it, and uses `replace_block()` to write back the result — going through the standard patch-reparse-validate cycle.

⸻

6. Interfaces

6.1 Application Services

Services compose domain logic and adapters. They are the main entrypoint for SDK, CLI, and tools.

Service structs:
- **ParseService** — parse markdown text or file into DocumentBlock
- **BlockService** — unified read/write operations on block tree
- **NormalizeService** — formatting normalization
- **ValidateService** — structural validation
- **AsciiService** — ASCII art processing

6.2 BlockService API (representative)

```rust
pub struct BlockService {
    policy: FilesystemPolicy,
}

impl BlockService {
    // Read
    pub fn read_outline(&self, file_path: &str, max_depth: u8) -> Result<ReadResponse>;
    pub fn read_block(&self, file_path: &str, selector: &BlockSelector, include_text: bool) -> Result<ReadResponse>;
    pub fn read_block_text(&self, file_path: &str, selector: &BlockSelector) -> Result<ReadResponse>;
    pub fn read_block_tree(&self, file_path: &str, selector: &BlockSelector, depth: i32, include_text: bool) -> Result<ReadResponse>;
    pub fn read_block_children(&self, file_path: &str, selector: &BlockSelector) -> Result<ReadResponse>;
    pub fn read_blocks_by_type(&self, file_path: &str, block_type: &str, limit: Option<usize>, offset: Option<usize>) -> Result<ReadResponse>;
    pub fn search_blocks(&self, file_path: &str, query: &str, selector: Option<&BlockSelector>, case_sensitive: bool) -> Result<ReadResponse>;

    // Write — generic (each maps to an EditOperation variant)
    pub fn replace_block(&self, file_path: &str, selector: &BlockSelector, content: &str, dry_run: bool) -> Result<WriteResponse>;
    pub fn insert_block(&self, file_path: &str, selector: &BlockSelector, content: &str, position: InsertPosition, dry_run: bool) -> Result<WriteResponse>;
    pub fn delete_block(&self, file_path: &str, selector: &BlockSelector, dry_run: bool) -> Result<WriteResponse>;
    pub fn move_block(&self, file_path: &str, selector: &BlockSelector, target_parent_id: BlockId, index: Option<usize>, dry_run: bool) -> Result<WriteResponse>;
    pub fn ensure_section(&self, file_path: &str, path: &str, heading_level: u8, dry_run: bool) -> Result<WriteResponse>;

    // Write — semantic helpers (each maps to an EditOperation variant)
    pub fn rename_section(&self, file_path: &str, selector: &BlockSelector, new_title: &str, dry_run: bool) -> Result<WriteResponse>;
    pub fn change_heading_level(&self, file_path: &str, selector: &BlockSelector, new_level: u8, dry_run: bool) -> Result<WriteResponse>;
    pub fn add_table_row(&self, file_path: &str, selector: &BlockSelector, row: &[String], position: InsertPosition, dry_run: bool) -> Result<WriteResponse>;
    pub fn update_table_cell(&self, file_path: &str, selector: &BlockSelector, row: usize, col: usize, value: &str, dry_run: bool) -> Result<WriteResponse>;
    pub fn remove_table_row(&self, file_path: &str, selector: &BlockSelector, row: usize, dry_run: bool) -> Result<WriteResponse>;
    pub fn toggle_task(&self, file_path: &str, selector: &BlockSelector, dry_run: bool) -> Result<WriteResponse>;
    pub fn add_list_item(&self, file_path: &str, selector: &BlockSelector, text: &str, index: Option<usize>, dry_run: bool) -> Result<WriteResponse>;
    pub fn remove_list_item(&self, file_path: &str, selector: &BlockSelector, index: usize, dry_run: bool) -> Result<WriteResponse>;

    // Write — unified entry point for MCP (single or batch, single patch-reparse)
    pub fn edit(&self, file_path: &str, operations: &[EditOp], dry_run: bool) -> Result<WriteResponse>;

    // Processing
    pub fn normalize(&self, file_path: &str, options: Option<NormalizeOptions>, dry_run: bool) -> Result<WriteResponse>;
    pub fn validate(&self, file_path: &str) -> Result<ReadResponse>;
}

pub enum InsertPosition { Before, After, Append, Index(usize) }
```

6.3 Result Models

```rust
/// Compact response for read operations. No changed/diff overhead.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ReadResponse {
    pub success: bool,
    pub data: serde_json::Value,  // operation-specific: outline, block, tree, search results
    pub diagnostics: Option<Vec<Diagnostic>>,  // None when empty — saves tokens
}

/// Full response for write operations. Includes change tracking.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct WriteResponse {
    pub success: bool,
    pub changed: bool,
    pub diagnostics: Vec<Diagnostic>,
    pub diff_summary: Option<DiffSummary>,
    pub content: Option<String>,  // resulting document content after edit
}

/// Error response — returned when success=false.
/// Mapped to MCP `isError: true` + content array.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ErrorResponse {
    pub error_code: String,          // e.g. "BLOCK_NOT_FOUND", "POLICY_VIOLATION"
    pub message: String,
    pub diagnostic: Option<Diagnostic>,
    pub suggested_action: Option<String>,
}
```

Token savings: `ReadResponse` is ~50% smaller than the previous unified `OperationResult` — no `changed`, no `diff_summary`, no `output_path`, no `metadata`.

6.4 Public API Surface

```rust
use mdtool_core::services::{ParseService, BlockService, ValidateService, NormalizeService, AsciiService};
use mdtool_core::domain::selectors::BlockSelector;
```

Usage:

```rust
let svc = BlockService::new(FilesystemPolicy::default());
let result = svc.replace_block(
    "concept.md",
    &BlockSelector::from_path("/Architecture/Risks"),
    "## Risks\n\nUpdated content.",
    true, // dry_run
)?;
```

6.5 CLI Design (clap derive)

```rust
#[derive(Parser)]
#[command(name = "mdtool", version, about = "Markdown operations engine")]
enum Cli {
    Outline { file: String, #[arg(long, default_value = "3")] max_depth: u8 },

    ReadBlock {
        file: String,
        #[arg(long)] path: Option<String>,
        #[arg(long)] id: Option<u32>,
        #[arg(long, default_value = "data")] view: String,  // data | text | tree | children
    },

    ReadBlocks { file: String, #[arg(long)] r#type: String },

    Search { file: String, query: String, #[arg(long)] case_sensitive: bool },

    #[command(subcommand)]
    Edit(EditCommand),

    Validate { file: String },
    Normalize { file: String, #[arg(long)] dry_run },
    FormatAscii { file: String, #[arg(long, default_value = "format_only")] mode: String },

    #[command(subcommand)]
    Mcp(McpSubcommand),
}

#[derive(Subcommand)]
enum EditCommand {
    Replace { file: String, #[arg(long)] path: Option<String>, #[arg(long)] id: Option<u32>, #[arg(long)] from_file: String },
    Insert { file: String, #[arg(long)] path: Option<String>, #[arg(long)] id: Option<u32>, #[arg(long)] after, #[arg(long)] before, #[arg(long)] from_file: String },
    Delete { file: String, #[arg(long)] id: u32 },
    Move { file: String, #[arg(long)] id: u32, #[arg(long)] target: u32, #[arg(long)] index: Option<usize> },
    RenameSection { file: String, #[arg(long)] path: String, #[arg(long)] new_title: String },
    AddTableRow { file: String, #[arg(long)] path: String, #[arg(long)] values: Vec<String> },
    ToggleTask { file: String, #[arg(long)] id: u32 },
    // ... all semantic operations as subcommands
}
```

CLI preserves fine-grained commands for human ergonomics. MCP collapses to 8 tools for token efficiency. Both use the same service logic.

6.6 MCP Tool Exposure — 7 Tools

Same service logic powers Rust SDK, CLI, and MCP server — no duplicated business logic per interface. Tools are designed to minimize count while preserving full domain coverage. Each tool is parameter-driven: a `view` or `operation` enum selects the specific behavior.

6.6.1 Tool Registry

| # | Name | readOnly | Description |
|---|------|----------|-------------|
| 1 | `markdown_read_outline` | yes | Return heading outline of a markdown document as a flat list of sections with id, level, title, and canonical path. Use this first to understand document structure before targeting specific blocks. |
| 2 | `markdown_read_block` | yes | Read block data from a markdown document. The `view` parameter controls what is returned: "data" (full block details), "text" (raw text content only), "tree" (block subtree as nested structure), "children" (ordered child blocks), or "type" (all blocks of a given type). Defaults to "data". |
| 3 | `markdown_search` | yes | Search for text across blocks in a markdown document. Returns matching block IDs and matched line text. Optionally scope to a subtree via selector. Case-insensitive by default. |
| 4 | `markdown_edit` | no | Edit one or more blocks in a markdown document. Pass a single operation for single edits, or an array for batch (single patch-reparse cycle). The `operation` parameter selects the mutation type: generic mutations (replace, insert, delete, move, ensure_section) and semantic helpers (rename_section, change_heading_level, add_table_row, update_table_cell, remove_table_row, toggle_task, add_list_item, remove_list_item). Semantic helpers construct structurally correct markdown internally. Defaults to dry_run=true. |
| 5 | `markdown_validate` | yes | Validate a markdown document for structural issues. Returns diagnostics with severity, code, message, and optional suggested fix. Checks: heading level jumps, duplicate section paths, unclosed fences, empty sections, ASCII formatting issues. |
| 6 | `markdown_normalize` | no | Normalize formatting of a markdown document without changing semantic content. Operations: heading spacing, trailing whitespace, excessive blank lines, final newline. Preserves fenced code contents by default. Defaults to dry_run=true. |
| 7 | `markdown_format_ascii` | no | Format or repair ASCII art diagrams inside fenced code blocks (info strings: ascii, box, diagram). Modes: format_only (safe), repair_safe (conservative border alignment). Leaves block unchanged if structural confidence is low. Defaults to dry_run=true. |

6.6.2 Tool Schemas

```rust
// === Tool 1: markdown_read_outline ===
#[derive(Deserialize, Serialize, JsonSchema)]
#[schemars(description = "Return heading outline of a markdown document")]
pub struct ReadOutlineRequest {
    pub file_path: String,
    #[serde(default = "default_max_depth")]
    pub max_depth: u8,  // default: 3
}

// === Tool 2: markdown_read_block ===
#[derive(Deserialize, Serialize, JsonSchema)]
#[schemars(description = "Read block data. Use 'view' to select: data, text, tree, children, type")]
pub struct ReadBlockRequest {
    pub file_path: String,
    #[serde(default)]
    pub selector: Option<BlockSelector>,  // required for views: data, text, tree, children
    #[serde(default = "default_view")]
    pub view: BlockView,                  // default: "data"
    pub block_type: Option<String>,       // required when view="type"
    pub depth: Option<i32>,               // for view="tree", default: -1 (unlimited)
    pub include_text: Option<bool>,       // for view="data" or "tree", default: true for data, false for tree
    pub limit: Option<usize>,             // for view="type", pagination limit
    pub offset: Option<usize>,            // for view="type", pagination offset
}

#[derive(Deserialize, Serialize, JsonSchema)]
pub enum BlockView { Data, Text, Tree, Children, Type }

// === Tool 3: markdown_search ===
#[derive(Deserialize, Serialize, JsonSchema)]
#[schemars(description = "Search for text across blocks. Returns matching block IDs and matched text.")]
pub struct SearchRequest {
    pub file_path: String,
    pub query: String,
    pub selector: Option<BlockSelector>,  // scope to subtree
    #[serde(default)]
    pub case_sensitive: bool,
}

// === Tool 4: markdown_edit ===
// Single operation or batch — always a single patch-reparse cycle.
#[derive(Deserialize, Serialize, JsonSchema)]
#[schemars(description = "Edit blocks. Single operation or batch array. Single patch-reparse cycle. Rejects all if any fails.")]
pub struct EditRequest {
    pub file_path: String,
    pub operations: Vec<EditOp>,          // 1+ operations; single-element = single edit
    #[serde(default = "default_true")]
    pub dry_run: bool,                    // default: true
}

#[derive(Deserialize, Serialize, JsonSchema)]
pub struct EditOp {
    pub operation: EditOperation,
    pub selector: Option<BlockSelector>,  // required for most operations
    pub content: Option<String>,          // for replace, insert
    pub position: Option<InsertPosition>, // for insert
    pub target_parent_id: Option<BlockId>,// for move
    pub index: Option<usize>,             // for move, add_list_item, remove_list_item
    pub path: Option<String>,             // for ensure_section
    pub heading_level: Option<u8>,        // for ensure_section, change_heading_level
    pub new_title: Option<String>,        // for rename_section
    pub row: Option<Vec<String>>,         // for add_table_row
    pub row_index: Option<usize>,         // for update_table_cell, remove_table_row
    pub col: Option<usize>,               // for update_table_cell
    pub value: Option<String>,            // for update_table_cell
}

#[derive(Deserialize, Serialize, JsonSchema)]
pub enum EditOperation {
    Replace,
    Insert,
    Delete,
    Move,
    EnsureSection,
    RenameSection,
    ChangeHeadingLevel,
    AddTableRow,
    UpdateTableCell,
    RemoveTableRow,
    ToggleTask,
    AddListItem,
    RemoveListItem,
}

// === Tool 5: markdown_validate ===
#[derive(Deserialize, Serialize, JsonSchema)]
#[schemars(description = "Validate document structure. Returns diagnostics with severity, code, message, suggested fix.")]
pub struct ValidateRequest {
    pub file_path: String,
}

// === Tool 6: markdown_normalize ===
#[derive(Deserialize, Serialize, JsonSchema)]
#[schemars(description = "Normalize formatting without changing semantic content. Preserves fenced code by default.")]
pub struct NormalizeRequest {
    pub file_path: String,
    pub options: Option<NormalizeOptions>,
    #[serde(default = "default_true")]
    pub dry_run: bool,
}

// === Tool 7: markdown_format_ascii ===
#[derive(Deserialize, Serialize, JsonSchema)]
#[schemars(description = "Format/repair ASCII art in fenced blocks (ascii, box, diagram). Conservative by default.")]
pub struct FormatAsciiRequest {
    pub file_path: String,
    #[serde(default)]
    pub mode: AsciiMode,  // default: FormatOnly
    #[serde(default = "default_true")]
    pub dry_run: bool,
}

```

6.6.3 Design Principles

**Token efficiency**: 7 tool schemas ≈ 700 tokens overhead per request (vs ~1900 for 19 tools). Parameter-driven design means each tool's schema is reused across multiple use cases.

**Unified edit tool**: `markdown_edit` handles both single and batch edits via `operations` array. Single-element array = single edit. Multi-element = batch with single patch-reparse cycle. No separate batch tool needed.

**Safe defaults**: All write tools default `dry_run=true`. AI previews changes before applying — prevents accidental document corruption.

**Semantic helpers as operations, not tools**: `markdown_edit` exposes 13 operation variants via a single `operation` enum. Server validates operation-specific parameters. This prevents AI from constructing malformed markdown (misaligned table pipes, missing list markers) while keeping the tool surface minimal.

**Compact responses**: Read tools return `ReadResponse` (no `changed`/`diff_summary`). Write tools return `WriteResponse` (includes `changed`/`diff_summary`).

**Pagination**: `view="type"` supports `limit`/`offset` to avoid returning hundreds of blocks at once.

**Response size guard**: `view="tree"` defaults `depth=3` for documents with >100 blocks. Prevents multi-KB responses that waste context window tokens. AI can increase depth if needed.

**Error handling**: All errors are returned as `ErrorResponse` with `error_code`, `message`, and optional `suggested_action`. Mapped to MCP `isError: true`. AI can read the error and self-correct (e.g., try a different selector, or set `allow_first_match=true`).

**JSON serialization**: All MCP responses use compact JSON (`serde_json::to_string`) — no pretty-printing. Saves ~30% tokens on whitespace.

**MCP Resources**: mdtool does NOT expose files as MCP Resources. Tools are sufficient — AI calls `read_block(view="text")` with an empty selector to read the full document. Adding Resources would duplicate read tool functionality.

**Rate limiting**: MCP server enforces configurable per-session rate limit (default: 100 calls/minute). Prevents runaway agents from overwhelming the server.

MCP handler rule: handlers must never contain transformation logic. They only: parse request → call service → serialize result.

6.7 Filesystem & Policy Controls

mdtool may be exposed to agents, so filesystem access must be constrained.

```rust
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
pub struct FilesystemPolicy {
    pub allowed_roots: Vec<String>,
    pub read_only: bool,
    pub allow_overwrite: bool,
    pub create_backups: bool,
}
```

Enforcement: all service methods operating on file paths pass through PolicyGuard: normalize absolute path → verify within allowed roots → verify operation allowed → continue or raise.

⸻

7. Error Model & Testing

7.1 Error Hierarchy

```rust
#[derive(Debug, thiserror::Error)]
pub enum MdtoolError {
    #[error("parse error: {0}")]
    Parse(String),

    #[error("validation error: {0}")]
    Validation(String),

    #[error("block not found: {selector:?}")]
    BlockNotFound { selector: BlockSelector },

    #[error("ambiguous block selector: {n} matches for {selector:?}")]
    AmbiguousBlock { selector: BlockSelector, n: usize },

    #[error("transformation error: {0}")]
    Transformation(String),

    #[error("ascii layout error: {0}")]
    AsciiLayout(String),

    #[error("policy violation: {0}")]
    Policy(String),

    #[error(transparent)]
    Io(#[from] std::io::Error),
}
```

Tool/CLI boundaries translate errors into structured outputs with: error type, message, optional diagnostic code, optional suggested action.

7.2 Logging & Observability

- No noisy logs by default; structured debug logging optional
- Debug payloads: resolved selector details, block tree snapshot, line range chosen for patch

7.3 Testing Architecture

Test pyramid: End-to-End → Integration → Golden → Unit

Unit tests (`#[test]`):
- Block tree construction, selector resolver, line range computation, patch span computation, ASCII box formatter, diagnostics emission.

Golden tests (insta snapshots — mandatory for document operations):
- Each fixture: input markdown + operation request + expected output markdown + expected diagnostics.
- `insta::assert_snapshot!` for deterministic output comparison.

Property tests (proptest):
- Idempotency of normalization, idempotency of format_only, selector consistency under duplicate headings, newline preservation invariants, block tree round-trip (parse → serialize → reparse yields same tree).

Integration tests (`tests/` directory):
- Filesystem policy enforcement, CLI JSON output, MCP server invocation, error response format verification.

⸻

8. Implementation Plan

Cargo workspace structure:

```
mdtool/
├── Cargo.toml              # workspace root
├── crates/
│   ├── mdtool-core/         # domain model, block ops, pipeline (shared library)
│   ├── mdtool-cli/          # CLI binary (depends on mdtool-core)
│   └── mdtool-mcp/          # MCP server binary (depends on mdtool-core)
├── tests/                  # integration tests
├── fixtures/               # golden test fixtures
└── specs/                  # architecture docs
```

Phase 1 — Foundations
cargo workspace setup, BlockId/LineRange/ByteRange primitives, Block struct, BlockNode enum, MdtoolError hierarchy, comrak parser adapter, block tree builder

Phase 2 — Tree Construction
flat block extraction from comrak AST, parent-child assignment, heading tree construction, section range computation, path/slug assignment, block_by_id HashMap index

Phase 3 — Read Path
BlockSelector, resolve_selector, read_block, read_block_children, read_blocks_by_type, read_outline, tree traversal iterators

Phase 4 — Write Path
replace_block, insert_block_before/after, append_child, delete_block, move_block, ensure_section, patch-reparse-validate cycle, diff summary

Phase 5 — Processing
Normalization pipeline, validation checks, diagnostics collection

Phase 6 — ASCII Engine
FenceBlock detection, format_only, repair_safe, diagnostics

Phase 7 — Interfaces
CLI commands (clap derive), serde+schemars request/response models, rmcp MCP server wrapper, MCP integration tests (tool discovery via tools/list, schema validation, error responses, dry_run=true vs false, rate limiting)

Definition of Done:

| Subsystem | Done when |
|-----------|-----------|
| Parser/Builder | Representative doc parses into stable DocumentBlock; block tree correct; ranges correct; duplicates explicit |
| Block Ops | Selectors resolve predictably; mutations hit correct spans only; reparsing mandatory & stable |
| ASCII Formatting | Fenced blocks detected; format_only idempotent; repair_safe conservatively corrects borders |
| Tooling | Core requests validate with serde+schemars; CLI and MCP use same service logic; 7 MCP tools with descriptions and annotations; outputs stable |

⸻

Non-Negotiables

1. No regex-only architecture for block parsing
2. No direct mutation logic inside CLI or MCP handlers
3. No public API leakage of parser-specific token internals
4. No silent ambiguity resolution unless explicitly requested
5. No write operation without reparsing and revalidation
6. No aggressive ASCII reconstruction by default
7. No unrestricted filesystem mutation in tool mode

