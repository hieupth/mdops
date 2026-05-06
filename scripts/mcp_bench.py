#!/usr/bin/env python3
"""MCP test harness with unbuffered I/O for mdtool-mcp."""
import subprocess, json, time, sys, os, select

MCP_BIN = "./target/release/mdtool-mcp"
CONCEPT_MD = "specs/concept.md"

proc = subprocess.Popen(
    [MCP_BIN],
    stdin=subprocess.PIPE,
    stdout=subprocess.PIPE,
    stderr=subprocess.PIPE,
    bufsize=0  # unbuffered
)

def send(msg):
    data = (json.dumps(msg) + "\n").encode()
    proc.stdin.write(data)
    proc.stdin.flush()

def recv():
    """Read one line from stdout (unbuffered)."""
    line = b""
    while True:
        ch = proc.stdout.read(1)
        if not ch:
            return None
        if ch == b"\n":
            break
        line += ch
    if not line:
        return None
    return json.loads(line.decode())

def tool_call(name, args, label):
    send({"jsonrpc": "2.0", "id": label, "method": "tools/call", "params": {"name": name, "arguments": args}})
    t0 = time.perf_counter()
    resp = recv()
    t1 = time.perf_counter()
    return resp, (t1 - t0) * 1000

results = []

try:
    # Initialize
    send({"jsonrpc": "2.0", "id": 0, "method": "initialize",
          "params": {"protocolVersion": "2024-11-05", "capabilities": {},
                     "clientInfo": {"name": "test", "version": "1.0"}}})
    resp = recv()
    print(f"Initialize: OK — {resp['result']['serverInfo']['name']} v{resp['result']['serverInfo']['version']}")

    # Initialized notification (no response expected)
    send({"jsonrpc": "2.0", "method": "notifications/initialized"})
    time.sleep(0.05)

    # List tools
    send({"jsonrpc": "2.0", "id": "list", "method": "tools/list", "params": {}})
    resp = recv()
    tools = resp.get("result", {}).get("tools", [])
    print(f"Tools: {len(tools)} registered")
    for t in tools:
        print(f"  - {t['name']}")

    # Run test calls
    print(f"\n{'='*70}")
    print(f"{'Label':<20} {'Tool':<30} {'Time':>8} {'Status':>8} {'Size':>8}")
    print(f"{'='*70}")

    tests = [
        ("outline",     "markdown_read_outline",  {"file_path": CONCEPT_MD, "max_depth": 6}),
        ("read-id1",    "markdown_read_block",    {"file_path": CONCEPT_MD, "selector": {"id": 1}, "view": "data", "include_text": True}),
        ("read-id50",   "markdown_read_block",    {"file_path": CONCEPT_MD, "selector": {"id": 50}, "view": "text"}),
        ("tree",        "markdown_read_block",    {"file_path": CONCEPT_MD, "view": "tree", "depth": 3}),
        ("search-1",    "markdown_search",        {"file_path": CONCEPT_MD, "query": "BlockNode", "case_sensitive": False}),
        ("search-2",    "markdown_search",        {"file_path": CONCEPT_MD, "query": "patch_and_reparse", "case_sensitive": False}),
        ("search-3",    "markdown_search",        {"file_path": CONCEPT_MD, "query": "fn ", "case_sensitive": True}),
        ("validate",    "markdown_validate",      {"file_path": CONCEPT_MD}),
        ("normalize",   "markdown_normalize",     {"file_path": CONCEPT_MD, "dry_run": True}),
        ("ascii",       "markdown_format_ascii",  {"file_path": CONCEPT_MD, "mode": "FormatOnly", "dry_run": True}),
        ("edit-replace","markdown_edit",          {"file_path": CONCEPT_MD, "operations": [{"operation": "Replace", "selector": {"id": 5}, "content": "**Replaced.**"}], "dry_run": True}),
        ("edit-insert", "markdown_edit",          {"file_path": CONCEPT_MD, "operations": [{"operation": "Insert", "selector": {"id": 10}, "content": "\n> New block\n", "position": "After"}], "dry_run": True}),
        ("edit-delete", "markdown_edit",          {"file_path": CONCEPT_MD, "operations": [{"operation": "Delete", "selector": {"id": 5}}], "dry_run": True}),
        ("edit-batch",  "markdown_edit",          {"file_path": CONCEPT_MD, "operations": [{"operation": "Replace", "selector": {"id": 5}, "content": "A"}, {"operation": "Insert", "selector": {"id": 10}, "content": "B", "position": "After"}], "dry_run": True}),
    ]

    for label, tool, args in tests:
        t0 = time.perf_counter()
        resp, ms = tool_call(tool, args, label)
        t1 = time.perf_counter()
        ms = (t1 - t0) * 1000

        is_error = resp.get("result", {}).get("isError", False) if resp else True
        status = "ERR" if is_error else "OK"
        content = resp.get("result", {}).get("content", []) if resp else []
        size = sum(len(c.get("text", "")) for c in content) if content else 0

        print(f"  {label:<18} {tool:<30} {ms:>6.1f}ms {status:>8} {size:>6}ch")
        results.append((label, tool, ms, status, size))

    print(f"{'='*70}")

    # Performance summary
    ok = [r for r in results if r[3] == "OK"]
    errs = [r for r in results if r[3] != "OK"]
    if ok:
        times = [r[2] for r in ok]
        print(f"\nPERFORMANCE SUMMARY ({CONCEPT_MD}, 1172 lines, ~48KB)")
        print(f"  Total calls: {len(results)}, Success: {len(ok)}, Errors: {len(errs)}")
        print(f"  Min: {min(times):.1f}ms | Max: {max(times):.1f}ms | Avg: {sum(times)/len(times):.1f}ms | Median: {sorted(times)[len(times)//2]:.1f}ms")

        cats = {"Read": [], "Search": [], "Write": [], "Process": []}
        for r in ok:
            if "edit" in r[0]: cats["Write"].append(r[2])
            elif "search" in r[0]: cats["Search"].append(r[2])
            elif r[0] in ("validate","normalize","ascii"): cats["Process"].append(r[2])
            else: cats["Read"].append(r[2])

        for cat, ts in cats.items():
            if ts:
                print(f"  {cat}: avg {sum(ts)/len(ts):.1f}ms, median {sorted(ts)[len(ts)//2]:.1f}ms ({len(ts)} calls)")

finally:
    proc.kill()
    proc.wait()
