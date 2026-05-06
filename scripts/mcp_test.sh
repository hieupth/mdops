#!/usr/bin/env bash
# MCP test harness: starts mdtool-mcp server, sends JSON-RPC requests, captures responses.
set -euo pipefail

MCP_BIN="${1:-./target/release/mdtool-mcp}"
INPUT_FILE="/tmp/mcp_test_input.json"
OUTPUT_FILE="/tmp/mcp_test_output.json"
TIMING_FILE="/tmp/mcp_test_timing.txt"

# Build JSON-RPC request sequence
build_requests() {
    cat > "$INPUT_FILE" <<'REQUESTS'
{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test-client","version":"1.0"}}}
{"jsonrpc":"2.0","method":"notifications/initialized"}
{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}
{"jsonrpc":"2.0","id":10,"method":"tools/call","params":{"name":"markdown_read_outline","arguments":{"file_path":"specs/concept.md","max_depth":6}}}
{"jsonrpc":"2.0","id":11,"method":"tools/call","params":{"name":"markdown_read_block","arguments":{"file_path":"specs/concept.md","selector":{"id":1},"view":"data","include_text":true}}}
{"jsonrpc":"2.0","id":12,"method":"tools/call","params":{"name":"markdown_read_block","arguments":{"file_path":"specs/concept.md","view":"text","selector":{"id":5}}}}
{"jsonrpc":"2.0","id":13,"method":"tools/call","params":{"name":"markdown_search","arguments":{"file_path":"specs/concept.md","query":"BlockNode","case_sensitive":false}}}
{"jsonrpc":"2.0","id":14,"method":"tools/call","params":{"name":"markdown_search","arguments":{"file_path":"specs/concept.md","query":"fn ","case_sensitive":true}}}
{"jsonrpc":"2.0","id":15,"method":"tools/call","params":{"name":"markdown_validate","arguments":{"file_path":"specs/concept.md"}}}
{"jsonrpc":"2.0","id":16,"method":"tools/call","params":{"name":"markdown_normalize","arguments":{"file_path":"specs/concept.md","dry_run":true}}}
{"jsonrpc":"2.0","id":17,"method":"tools/call","params":{"name":"markdown_format_ascii","arguments":{"file_path":"specs/concept.md","mode":"FormatOnly","dry_run":true}}}
{"jsonrpc":"2.0","id":20,"method":"tools/call","params":{"name":"markdown_edit","arguments":{"file_path":"specs/concept.md","operations":[{"operation":"Replace","selector":{"id":5},"content":"**Replaced paragraph.**"}],"dry_run":true}}}
{"jsonrpc":"2.0","id":21,"method":"tools/call","params":{"name":"markdown_edit","arguments":{"file_path":"specs/concept.md","operations":[{"operation":"Insert","selector":{"id":10},"content":"\n> New blockquote\n","position":"After"}],"dry_run":true}}}
{"jsonrpc":"2.0","id":22,"method":"tools/call","params":{"name":"markdown_edit","arguments":{"file_path":"specs/concept.md","operations":[{"operation":"Delete","selector":{"id":5}}],"dry_run":true}}}
{"jsonrpc":"2.0","id":30,"method":"tools/call","params":{"name":"markdown_read_block","arguments":{"file_path":"specs/concept.md","view":"tree","depth":-1}}}
REQUESTS
}

# Run MCP server with input and capture output
run_mcp() {
    local start_ns end_ns elapsed_ms
    start_ns=$(date +%s%N)
    # Feed input to MCP server, capture output
    # The server reads from stdin, writes to stdout
    cat "$INPUT_FILE" | timeout 10 "$MCP_BIN" 2>/dev/null | head -30 > "$OUTPUT_FILE" || true
    end_ns=$(date +%s%N)
    elapsed_ms=$(( (end_ns - start_ns) / 1000000 ))
    echo "${elapsed_ms}ms" > "$TIMING_FILE"
}

# Extract a specific JSON-RPC response by id
get_response() {
    local id="$1"
    grep -a "\"id\":${id}" "$OUTPUT_FILE" || echo "{}"
}

echo "=== MCP Test Harness ==="
echo "Binary: $MCP_BIN"
echo ""

build_requests
echo "Built $(wc -l < "$INPUT_FILE") JSON-RPC requests"
echo ""

run_mcp
total_time=$(cat "$TIMING_FILE")
echo "Total MCP session time: ${total_time}"
echo ""

echo "=== Responses ==="
echo ""

# Parse each response
for id in 1 2 10 11 12 13 14 15 16 17 20 21 22 30; do
    resp=$(get_response "$id")
    if [ "$resp" != "{}" ]; then
        # Pretty print
        echo "--- Request id=$id ---"
        echo "$resp" | python3 -m json.tool 2>/dev/null | head -20
        echo ""
    fi
done
