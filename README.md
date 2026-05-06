# mdtool

Rust-native Markdown operations engine for AI agents.

Parses Markdown into a semantic block tree, then exposes structured read,
search, edit, validate, normalize, and ASCII-art-repair operations through
three interfaces: **CLI**, **MCP server**, and a Rust library crate.

## Quick Start

```bash
# Build
cargo build --release

# CLI — heading outline
mdtool outline doc.md --max-depth 3

# CLI — search
mdtool search doc.md "pattern"

# CLI — edit (dry run)
mdtool edit replace doc.md --path /section --content "new text" --dry-run

# MCP server (for AI agents)
mdtool-mcp
```

## Tools

| Tool | Description |
|------|-------------|
| `outline` | Heading outline with levels and canonical paths |
| `read-block` | Block data, text, tree, children, or by-type views |
| `search` | Block-scoped text search |
| `edit` | Replace, insert, delete, rename, toggle (dry-run support) |
| `validate` | Structural validation with diagnostics |
| `normalize` | Formatting normalization |
| `format-ascii` | ASCII art formatting and repair |

## Benchmark: MCP vs CLI vs Direct

Fair token-budget comparison on a 168 KB, 3 140-line PRD document.
Measures **input + output characters** per operation — what the agent actually
sends and receives over the wire.

```
Total Token Budget (chars)
                          MCP   30.1K  |##                                  |
                          CLI   65.8K  |####                                |
                        Direct  1.3M   |########################################|
                                       0        320K      640K      960K    1.3M

MCP  saves 97.6%  vs Direct
CLI  saves 94.7%  vs Direct
```

Why the gap? When an agent manipulates files directly, it must **Read the
entire document** (168 KB) into context before it can locate or edit anything.
mdtool's block-tree engine handles the file internally and returns only the
structured result the agent asked for.

```
Input vs Output Breakdown

  MCP    in   4.5K   |                                    |
         out  25.6K  |##                                  |
  ----
  CLI    in   2.5K   |                                    |
         out  63.3K  |####                                |
  ----
  Direct in   1.8K   |                                    |
         out   1.2M  |########################################|
                       0       320K      640K      960K     1.3M
```

```
Per-Group Total Chars

  Read (outline, section text, fences, children)
    MCP     9.8K  |#                                   |
    CLI    30.5K  |##                                  |
  Direct  188.8K  |##############                      |

  Search (Invoice, scoped API, case-sensitive)
    MCP     6.1K  |#                                   |
    CLI     7.9K  |#                                   |
  Direct   59.6K  |#####                               |

  Edit (replace, rename, delete, insert — dry run)
    MCP     1.7K  |                                    |
    CLI     5.0K  |                                    |
  Direct  498.7K  |####################################|

  Process (validate, normalize)
    MCP     1.9K  |                                    |
    CLI     2.0K  |                                    |
  Direct  166.4K  |############                        |

  ASCII (find, format)
    MCP     0.9K  |                                    |
    CLI     1.0K  |                                    |
  Direct  166.8K  |############                        |

  E2E (outline → search → edit → validate)
    MCP     9.7K  |#                                   |
    CLI    19.3K  |#                                   |
  Direct  174.5K  |#############                       |
```

```
Round Trips & Latency

  Round trips     MCP   19    CLI   19    Direct   32
  Total time ms   MCP   29    CLI   71    Direct   34
```

> Thinking tokens are NOT measured. Direct approach requires more agent
> reasoning to parse raw text, so the real-world advantage is even larger.

## Architecture

See [`specs/concept.md`](specs/concept.md) for the full architecture document.

## License

Copyright &copy; 2025 [Hieu Pham](https://github.com/hieupth). All rights reserved.
