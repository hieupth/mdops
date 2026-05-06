#!/usr/bin/env python3
"""
Comprehensive MCP test harness for mdtool-mcp.
Tests all 7 MCP tools across 8 groups on full_prd.md (3140 lines, 168KB).
Outputs structured JSON metrics for comparison with direct file ops.
"""
import subprocess, json, time, sys, os

MCP_BIN = os.environ.get("MCP_BIN", "./target/release/mdtool-mcp")
TEST_FILE = os.environ.get("TEST_FILE", "mcp_test_prd.md")
ASCII_FILE = "broken_ascii.md"
RESULTS = []

# ── MCP Protocol helpers ──────────────────────────────────────────────
proc = subprocess.Popen(
    [MCP_BIN],
    stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
    bufsize=0,
)

def send(msg):
    proc.stdin.write((json.dumps(msg) + "\n").encode())
    proc.stdin.flush()

def recv():
    line = b""
    while True:
        ch = proc.stdout.read(1)
        if not ch:
            return None
        if ch == b"\n":
            break
        line += ch
    return json.loads(line.decode()) if line else None

def parse_response(text):
    """Parse MCP tool response text. Returns (data, error_msg).
    The API wraps responses in {"success": true/false, "data": ..., "error": ...}."""
    if not text:
        return None, "empty response"
    try:
        wrapper = json.loads(text)
        if isinstance(wrapper, dict):
            if wrapper.get("success"):
                return wrapper.get("data"), None
            else:
                return None, wrapper.get("error", wrapper.get("message", text[:200]))
        # Some responses may be raw data (not wrapped)
        return wrapper, None
    except json.JSONDecodeError:
        return None, f"JSON parse error: {text[:200]}"

def tool_call(name, args, label):
    send({"jsonrpc": "2.0", "id": label, "method": "tools/call",
          "params": {"name": name, "arguments": args}})
    t0 = time.perf_counter()
    resp = recv()
    t1 = time.perf_counter()
    ms = (t1 - t0) * 1000

    is_error = resp.get("result", {}).get("isError", False) if resp else True
    content = resp.get("result", {}).get("content", []) if resp else []
    size = sum(len(c.get("text", "")) for c in content)
    text = content[0].get("text", "") if content else ""

    result = {
        "label": label,
        "tool": name,
        "args": args,
        "ms": round(ms, 1),
        "status": "ERR" if is_error else "OK",
        "output_chars": size,
        "is_error": is_error,
        "response_preview": text[:500] if not is_error else text[:200],
    }
    RESULTS.append(result)
    return result, text

# ── Initialize ────────────────────────────────────────────────────────
send({"jsonrpc": "2.0", "id": 0, "method": "initialize",
      "params": {"protocolVersion": "2024-11-05", "capabilities": {},
                 "clientInfo": {"name": "agent-test", "version": "1.0"}}})
resp = recv()
print(f"Initialize: {resp['result']['serverInfo']['name']} v{resp['result']['serverInfo']['version']}")
send({"jsonrpc": "2.0", "method": "notifications/initialized"})
time.sleep(0.05)

# List tools
send({"jsonrpc": "2.0", "id": "list", "method": "tools/list", "params": {}})
resp = recv()
tools = [t["name"] for t in resp.get("result", {}).get("tools", [])]
print(f"Tools ({len(tools)}): {', '.join(tools)}\n")

# ── Test Execution ────────────────────────────────────────────────────
print(f"{'='*90}")
print(f"{'Group':<6} {'Label':<25} {'Tool':<30} {'Time':>8} {'Status':>7} {'Chars':>7}")
print(f"{'='*90}")

def run(group, label, tool, args):
    r, text = tool_call(tool, args, label)
    r["group"] = group
    status = "ERR" if r["is_error"] else "OK"
    print(f"  G{group:<4} {label:<25} {tool:<30} {r['ms']:>6.1f}ms {status:>7} {r['output_chars']:>6}ch")
    return r, text

# ─────────────────────────────────────────────────────────────────────
# GROUP 1: Read Structure
# ─────────────────────────────────────────────────────────────────────
run("1", "1.1-outline-d3", "markdown_read_outline",
    {"file_path": TEST_FILE, "max_depth": 3})

run("1", "1.2-block-id1-data", "markdown_read_block",
    {"file_path": TEST_FILE, "selector": {"id": 1}, "view": "data", "include_text": True})

# Find path for section "01. Tong quan" by reading outline first
r1, outline_text = tool_call("markdown_read_outline",
    {"file_path": TEST_FILE, "max_depth": 6}, "1.3-outline-full")
r1["group"] = "1"

# API returns {"success": true, "data": [...sections...]}
sections, outline_err = parse_response(outline_text)
if outline_err:
    print(f"  WARNING: Failed to parse outline: {outline_err}")
    sections = []

print(f"  G1    {'1.3-outline-full':<25} {'markdown_read_outline':<30} {r1['ms']:>6.1f}ms {'OK' if not outline_err else 'ERR':>7} {r1['output_chars']:>6}ch")

# Find section 01 path and section 02 id
sec01_path = None
sec02_id = None
if sections:
    for s in sections:
        title = s.get("title", "")
        if "Tong quan" in title or "tong quan" in title.lower():
            sec01_path = s.get("path", "")
        if "Kien truc" in title or "kien truc" in title.lower():
            sec02_id = s.get("id")

if sec01_path:
    run("1", "1.3-tree-sec01", "markdown_read_block",
        {"file_path": TEST_FILE, "view": "tree", "selector": {"path": sec01_path}, "depth": 2})

if sec02_id:
    run("1", "1.4-children-sec02", "markdown_read_block",
        {"file_path": TEST_FILE, "view": "children", "selector": {"id": sec02_id}})

run("1", "1.5-fences-by-type", "markdown_read_block",
    {"file_path": TEST_FILE, "view": "by_type", "block_type": "fence"})

# ─────────────────────────────────────────────────────────────────────
# GROUP 2: Search
# ─────────────────────────────────────────────────────────────────────
run("2", "2.1-search-invoice", "markdown_search",
    {"file_path": TEST_FILE, "query": "Invoice", "case_sensitive": False})

run("2", "2.2-search-confidence", "markdown_search",
    {"file_path": TEST_FILE, "query": "confidence", "case_sensitive": True})

run("2", "2.3-search-api", "markdown_search",
    {"file_path": TEST_FILE, "query": "API", "case_sensitive": False})

# Find a section path for scoped search
sec06_path = None
if sections:
    for s in sections:
        if "Functional" in s.get("title", "") or "Chuc nang" in s.get("title", ""):
            sec06_path = s.get("path", "")
            break

if sec06_path:
    run("2", "2.4-search-api-scoped", "markdown_search",
        {"file_path": TEST_FILE, "query": "API", "selector": {"path": sec06_path}})

# ─────────────────────────────────────────────────────────────────────
# GROUP 3: Write - Replace
# ─────────────────────────────────────────────────────────────────────
# Find paragraph block IDs via by_type. API returns {"success":true,"data":[id1,id2,...]}
r_para, para_text = tool_call("markdown_read_block",
    {"file_path": TEST_FILE, "view": "by_type", "block_type": "paragraph"},
    "find-para")
para_ids, para_err = parse_response(para_text)
if para_err:
    print(f"  WARNING: Failed to parse by_type response: {para_err}")
    para_ids = []

# para_ids is a flat list of BlockId integers, e.g. [16, 42, ...]
if para_ids and len(para_ids) > 0:
    para_id = para_ids[0]
    if para_id is not None:
        run("3", "3.1-replace-para", "markdown_edit",
            {"file_path": TEST_FILE,
             "operations": [{"operation": "Replace", "selector": {"id": para_id},
                            "content": "**Replaced paragraph for testing.**"}],
             "dry_run": True})

# RenameSection - find first level-2 section from outline
first_section = None
if sections:
    for s in sections:
        if s.get("level", 0) == 2 and s.get("id", 0) > 1:
            first_section = s
            break
if first_section:
    run("3", "3.2-rename-section", "markdown_edit",
        {"file_path": TEST_FILE,
         "operations": [{"operation": "RenameSection",
                        "selector": {"id": first_section["id"]},
                        "new_title": "Renamed Section"}],
         "dry_run": True})

# Batch replace - need at least 2 paragraph IDs
if para_ids and len(para_ids) >= 2:
    p1 = para_ids[0]
    p2 = para_ids[1]
    if p1 is not None and p2 is not None:
        run("3", "3.3-batch-replace", "markdown_edit",
            {"file_path": TEST_FILE,
             "operations": [
                {"operation": "Replace", "selector": {"id": p1}, "content": "Batch A"},
                {"operation": "Replace", "selector": {"id": p2}, "content": "Batch B"},
             ],
             "dry_run": True})

# ─────────────────────────────────────────────────────────────────────
# GROUP 4: Write - Insert/Delete
# ─────────────────────────────────────────────────────────────────────
if para_ids:
    pid = para_ids[0]
    if pid is not None:
        run("4", "4.1-insert-after", "markdown_edit",
            {"file_path": TEST_FILE,
             "operations": [{"operation": "Insert", "selector": {"id": pid},
                            "content": "\n> Inserted blockquote after paragraph.\n",
                            "position": "After"}],
             "dry_run": True})

    pid2 = para_ids[1] if len(para_ids) > 1 else None
    if pid2 is not None:
        run("4", "4.2-insert-before", "markdown_edit",
            {"file_path": TEST_FILE,
             "operations": [{"operation": "Insert", "selector": {"id": pid2},
                            "content": "Inserted paragraph before.",
                            "position": "Before"}],
             "dry_run": True})

        run("4", "4.3-delete-block", "markdown_edit",
            {"file_path": TEST_FILE,
             "operations": [{"operation": "Delete", "selector": {"id": pid2}}],
             "dry_run": True})

# EnsureSection
run("4", "4.4-ensure-section", "markdown_edit",
    {"file_path": TEST_FILE,
     "operations": [{"operation": "EnsureSection", "path": "/test-section",
                    "heading_level": 2}],
     "dry_run": True})

# ─────────────────────────────────────────────────────────────────────
# GROUP 5: Validate & Normalize
# ─────────────────────────────────────────────────────────────────────
run("5", "5.1-validate", "markdown_validate",
    {"file_path": TEST_FILE})

r_norm, norm_text = tool_call("markdown_normalize",
    {"file_path": TEST_FILE, "dry_run": True}, "5.2-normalize")
r_norm["group"] = "5"
norm_data, norm_err = parse_response(norm_text)
norm_status = "OK" if not norm_err else "ERR"
print(f"  G5    {'5.2-normalize':<25} {'markdown_normalize':<30} {r_norm['ms']:>6.1f}ms {norm_status:>7} {r_norm['output_chars']:>6}ch")

# ─────────────────────────────────────────────────────────────────────
# GROUP 6: ASCII Art
# ─────────────────────────────────────────────────────────────────────
run("6", "6.1-ascii-fences", "markdown_read_block",
    {"file_path": ASCII_FILE, "view": "by_type", "block_type": "fence"})

run("6", "6.2-ascii-format", "markdown_format_ascii",
    {"file_path": ASCII_FILE, "mode": "FormatOnly", "dry_run": True})

run("6", "6.3-ascii-repair", "markdown_format_ascii",
    {"file_path": ASCII_FILE, "mode": "RepairSafe", "dry_run": True})

# ─────────────────────────────────────────────────────────────────────
# GROUP 7: Summarize & Extract
# ─────────────────────────────────────────────────────────────────────
# 7.1: Get outline to identify removable sections
run("7", "7.1-outline-analyze", "markdown_read_outline",
    {"file_path": TEST_FILE, "max_depth": 2})

# 7.2: Delete last 2 level-1 sections (dry_run) — run as separate calls since batch delete
# on same document would fail (IDs shift after first delete in patch-reparse cycle)
if sections:
    last_sections = [s for s in sections if s.get("level") == 1][-2:]
    if len(last_sections) >= 2:
        run("7", "7.2a-delete-sec", "markdown_edit",
            {"file_path": TEST_FILE,
             "operations": [{"operation": "Delete", "selector": {"id": last_sections[0]["id"]}}],
             "dry_run": True})
        run("7", "7.2b-delete-sec", "markdown_edit",
            {"file_path": TEST_FILE,
             "operations": [{"operation": "Delete", "selector": {"id": last_sections[1]["id"]}}],
             "dry_run": True})

# 7.3: Extract text of first 3 sections for summary
if sections:
    for i, s in enumerate(sections[:3]):
        if s.get("id"):
            run("7", f"7.3-extract-{i}", "markdown_read_block",
                {"file_path": TEST_FILE, "view": "text", "selector": {"id": s["id"]}})

# 7.4: Normalize after conceptual edit
run("7", "7.4-post-edit-norm", "markdown_normalize",
    {"file_path": TEST_FILE, "dry_run": True})

# ─────────────────────────────────────────────────────────────────────
# GROUP 8: End-to-End Workflow
# ─────────────────────────────────────────────────────────────────────
t_start = time.perf_counter()

# Step 1: Outline
r1, _ = tool_call("markdown_read_outline",
    {"file_path": TEST_FILE, "max_depth": 3}, "8.1a-outline")
r1["group"] = "8"
# Step 2: Search
r2, _ = tool_call("markdown_search",
    {"file_path": TEST_FILE, "query": "Invoice"}, "8.1b-search")
r2["group"] = "8"
# Step 3: Edit (dry_run)
r3, _ = tool_call("markdown_edit",
    {"file_path": TEST_FILE,
     "operations": [{"operation": "Replace", "selector": {"id": 2},
                    "content": "## Modified Product Requirements"}],
     "dry_run": True}, "8.1c-edit")
r3["group"] = "8"
# Step 4: Validate
r4, _ = tool_call("markdown_validate",
    {"file_path": TEST_FILE}, "8.1d-validate")
r4["group"] = "8"
# Step 5: Normalize
r5, _ = tool_call("markdown_normalize",
    {"file_path": TEST_FILE, "dry_run": True}, "8.1e-normalize")
r5["group"] = "8"

t_end = time.perf_counter()
e2e_ms = (t_end - t_start) * 1000

for label, r in [("8.1a-outline", r1), ("8.1b-search", r2), ("8.1c-edit", r3),
                  ("8.1d-validate", r4), ("8.1e-normalize", r5)]:
    status = "ERR" if r["is_error"] else "OK"
    print(f"  G8    {label:<25} {r['tool']:<30} {r['ms']:>6.1f}ms {status:>7} {r['output_chars']:>6}ch")

print(f"\n  E2E workflow total: {e2e_ms:.1f}ms (5 sequential calls)")

# ─────────────────────────────────────────────────────────────────────
# SUMMARY
# ─────────────────────────────────────────────────────────────────────
ok_results = [r for r in RESULTS if r["status"] == "OK"]
err_results = [r for r in RESULTS if r["status"] != "OK"]
times = [r["ms"] for r in ok_results]
sizes = [r["output_chars"] for r in ok_results]

print(f"\n{'='*90}")
print(f"SUMMARY: {len(RESULTS)} calls, {len(ok_results)} OK, {len(err_results)} ERR")
if err_results:
    print(f"\nERRORS:")
    for r in err_results:
        print(f"  - {r['label']}: {r['tool']} -> {r['response_preview'][:200]}")

if times:
    print(f"\nPERFORMANCE:")
    print(f"  Min: {min(times):.1f}ms | Max: {max(times):.1f}ms | Avg: {sum(times)/len(times):.1f}ms | Median: {sorted(times)[len(times)//2]:.1f}ms")

    # By group
    groups = {}
    for r in ok_results:
        g = r.get("group", "?")
        groups.setdefault(g, []).append(r["ms"])
    print(f"\n  By Group:")
    for g in sorted(groups.keys()):
        ts = groups[g]
        print(f"    Group {g}: avg {sum(ts)/len(ts):.1f}ms, median {sorted(ts)[len(ts)//2]:.1f}ms ({len(ts)} calls)")

    # By tool
    tool_stats = {}
    for r in ok_results:
        tool_stats.setdefault(r["tool"], []).append(r["ms"])
    print(f"\n  By Tool:")
    for t in sorted(tool_stats.keys()):
        ts = tool_stats[t]
        print(f"    {t}: avg {sum(ts)/len(ts):.1f}ms, median {sorted(ts)[len(ts)//2]:.1f}ms ({len(ts)} calls)")

if sizes:
    print(f"\nOUTPUT SIZE (token proxy):")
    print(f"  Total output: {sum(sizes):,} chars across {len(ok_results)} calls")
    print(f"  Avg per call: {sum(sizes)//len(sizes):,} chars")

# Write JSON results for comparison
output = {
    "total_calls": len(RESULTS),
    "ok_calls": len(ok_results),
    "err_calls": len(err_results),
    "e2e_ms": round(e2e_ms, 1),
    "results": RESULTS,
    "errors": [{"label": r["label"], "tool": r["tool"], "preview": r["response_preview"]} for r in err_results],
}
with open("/tmp/mcp_test_results.json", "w") as f:
    json.dump(output, f, indent=2, ensure_ascii=False)
print(f"\nResults written to /tmp/mcp_test_results.json")

# ── Cleanup ──────────────────────────────────────────────────────────
proc.kill()
proc.wait()
