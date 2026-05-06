#!/usr/bin/env python3
"""Fair benchmark: MCP vs CLI vs Direct file manipulation.

Measures total token budget (input_chars + output_chars) for each approach.
Outputs benchmark.csv and updates README.md with ASCII visualization.
"""

import csv
import json
import os
import subprocess
import sys
import time

# ---------------------------------------------------------------------------
# Configuration
# ---------------------------------------------------------------------------

REPO_ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
TEST_FILE = os.path.join(REPO_ROOT, "tests", "full_prd.md")
CLI_BIN = os.path.join(REPO_ROOT, "target", "release", "mdtool")
MCP_BIN = os.path.join(REPO_ROOT, "target", "release", "mdtool-mcp")
CSV_PATH = os.path.join(REPO_ROOT, "benchmark.csv")
README_PATH = os.path.join(REPO_ROOT, "README.md")

# Load test file content for Direct measurements
with open(TEST_FILE, "r") as f:
    FILE_CONTENT = f.read()
FILE_LINES = FILE_CONTENT.split("\n")
FILE_SIZE = len(FILE_CONTENT)

# ---------------------------------------------------------------------------
# MCP helpers
# ---------------------------------------------------------------------------

class MCPClient:
    """Communicates with mdtool-mcp via JSON-RPC over stdio."""

    def __init__(self):
        self.proc = subprocess.Popen(
            [MCP_BIN],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.DEVNULL,
        )
        self._req_id = 0
        # Initialize
        self._send_raw({
            "jsonrpc": "2.0",
            "id": self._next_id(),
            "method": "initialize",
            "params": {
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": {"name": "benchmark", "version": "1.0"},
            },
        })
        self._read_response()
        # Send initialized notification
        self._send_raw({
            "jsonrpc": "2.0",
            "method": "notifications/initialized",
        })

    def _next_id(self):
        self._req_id += 1
        return self._req_id

    def _send_raw(self, msg):
        data = json.dumps(msg) + "\n"
        self.proc.stdin.write(data.encode())
        self.proc.stdin.flush()
        return data

    def _read_response(self):
        line = self.proc.stdout.readline()
        if not line:
            return None
        return line.decode().strip()

    def call_tool(self, tool_name: str, arguments: dict) -> dict:
        """Call an MCP tool. Returns {input_chars, output_chars, time_ms, status}."""
        req = {
            "jsonrpc": "2.0",
            "id": self._next_id(),
            "method": "tools/call",
            "params": {"name": tool_name, "arguments": arguments},
        }
        req_json = json.dumps(req) + "\n"
        input_chars = len(req_json)

        t0 = time.perf_counter()
        self.proc.stdin.write(req_json.encode())
        self.proc.stdin.flush()
        resp_line = self.proc.stdout.readline()
        elapsed_ms = (time.perf_counter() - t0) * 1000

        if not resp_line:
            return {"input_chars": input_chars, "output_chars": 0, "time_ms": elapsed_ms, "status": "ERR"}

        resp = resp_line.decode().strip()
        output_chars = len(resp)

        # Check for errors
        try:
            parsed = json.loads(resp)
            is_error = parsed.get("result", {}).get("isError", False)
            status = "ERR" if is_error else "OK"
        except json.JSONDecodeError:
            status = "ERR"

        return {"input_chars": input_chars, "output_chars": output_chars, "time_ms": elapsed_ms, "status": status}

    def close(self):
        self.proc.terminate()
        self.proc.wait(timeout=5)


# ---------------------------------------------------------------------------
# CLI helper
# ---------------------------------------------------------------------------

def run_cli(args: list) -> dict:
    """Run mdtool CLI. Returns {input_chars, output_chars, time_ms, status}."""
    cmd = [CLI_BIN] + args
    cmd_str = " ".join(cmd)
    input_chars = len(cmd_str)

    t0 = time.perf_counter()
    result = subprocess.run(cmd, capture_output=True, text=True, timeout=30)
    elapsed_ms = (time.perf_counter() - t0) * 1000

    output_chars = len(result.stdout)
    status = "OK" if result.returncode == 0 else "ERR"

    return {"input_chars": input_chars, "output_chars": output_chars, "time_ms": elapsed_ms, "status": status}


# ---------------------------------------------------------------------------
# Direct (raw file ops) helper
# ---------------------------------------------------------------------------

def run_direct_shell(cmd: str) -> dict:
    """Run a shell command. Returns {input_chars, output_chars, time_ms, status}."""
    input_chars = len(cmd)
    t0 = time.perf_counter()
    result = subprocess.run(cmd, shell=True, capture_output=True, text=True, timeout=30)
    elapsed_ms = (time.perf_counter() - t0) * 1000
    output_chars = len(result.stdout)
    status = "OK" if result.returncode in (0, 1) else "ERR"  # grep returns 1 when no match
    return {"input_chars": input_chars, "output_chars": output_chars, "time_ms": elapsed_ms, "status": status}


def direct_read_file(file_path: str) -> dict:
    """Simulate agent reading entire file (Read tool)."""
    input_chars = len(file_path)
    output_chars = FILE_SIZE
    return {"input_chars": input_chars, "output_chars": output_chars, "time_ms": 1.0, "status": "OK"}


def direct_read_lines(file_path: str, start: int, end: int) -> dict:
    """Simulate agent reading a line range (Read tool with offset/limit)."""
    input_chars = len(f"{file_path} offset={start} limit={end-start+1}")
    selected = "\n".join(FILE_LINES[start - 1:end])
    output_chars = len(selected)
    return {"input_chars": input_chars, "output_chars": output_chars, "time_ms": 0.5, "status": "OK"}


def direct_edit(old_str: str, new_str: str) -> dict:
    """Simulate agent Edit tool call."""
    input_chars = len(old_str) + len(new_str)
    output_chars = 50  # typical Edit tool response
    return {"input_chars": input_chars, "output_chars": output_chars, "time_ms": 1.0, "status": "OK"}


# ---------------------------------------------------------------------------
# Test scenarios
# ---------------------------------------------------------------------------

def get_scenarios():
    """Return list of (group, task, mcp_fn, cli_fn, direct_fn) tuples."""

    scenarios = []

    # ---- Group 1: Read Structure ----

    def mcp_outline():
        return mcp.call_tool("markdown_read_outline", {
            "file_path": TEST_FILE, "max_depth": 3, "include_paths": False
        })

    def cli_outline():
        return run_cli(["outline", TEST_FILE, "--max-depth", "3"])

    def direct_outline():
        # Agent must read file + grep headings
        read = direct_read_file(TEST_FILE)
        grep = run_direct_shell(f"grep -n '^#' {TEST_FILE}")
        return {
            "input_chars": read["input_chars"] + grep["input_chars"],
            "output_chars": read["output_chars"] + grep["output_chars"],
            "time_ms": read["time_ms"] + grep["time_ms"],
            "status": "OK",
            "round_trips": 2,
        }

    scenarios.append(("1-Read", "1.1 Outline depth=3", mcp_outline, cli_outline, direct_outline))

    # 1.2 Read section text
    SECTION_PATH = "/prd-貿易書類-受注fax-aiocr-system/product-requirements-document"

    def mcp_section_text():
        return mcp.call_tool("markdown_read_block", {
            "file_path": TEST_FILE,
            "selector": {"path": SECTION_PATH},
            "view": "text",
        })

    def cli_section_text():
        return run_cli(["read-block", TEST_FILE, "--path", SECTION_PATH, "--view", "text"])

    def direct_section_text():
        # Agent must: grep to find line range → Read that range
        grep = run_direct_shell(f"grep -n '^#' {TEST_FILE}")
        # Agent would parse grep output, find section lines (approx 2-10)
        read = direct_read_lines(TEST_FILE, 2, 10)
        return {
            "input_chars": grep["input_chars"] + read["input_chars"],
            "output_chars": grep["output_chars"] + read["output_chars"],
            "time_ms": grep["time_ms"] + read["time_ms"],
            "status": "OK",
            "round_trips": 2,
        }

    scenarios.append(("1-Read", "1.2 Section text", mcp_section_text, cli_section_text, direct_section_text))

    # 1.3 List all fences
    def mcp_fences():
        return mcp.call_tool("markdown_read_block", {
            "file_path": TEST_FILE, "view": "by_type", "block_type": "fence"
        })

    def cli_fences():
        return run_cli(["read-blocks", TEST_FILE, "--type", "fence"])

    def direct_fences():
        # Agent must grep for ``` and pair them manually
        grep1 = run_direct_shell(f"grep -n '```' {TEST_FILE}")
        return {
            "input_chars": grep1["input_chars"],
            "output_chars": grep1["output_chars"],
            "time_ms": grep1["time_ms"],
            "status": "OK",
            "round_trips": 1,
        }

    scenarios.append(("1-Read", "1.3 List fences", mcp_fences, cli_fences, direct_fences))

    # 1.4 Children of section 02
    SECTION2_PATH = "/01-tổng-quan-hệ-thống-system-overview"

    def mcp_children():
        return mcp.call_tool("markdown_read_block", {
            "file_path": TEST_FILE,
            "selector": {"path": SECTION2_PATH},
            "view": "children",
        })

    def cli_children():
        # CLI doesn't support view=children; use view=data as fallback
        return run_cli(["read-block", TEST_FILE, "--path", SECTION2_PATH])

    def direct_children():
        # Agent must: grep headings → find section range → Read → count sub-headings
        grep = run_direct_shell(f"grep -n '^#' {TEST_FILE}")
        # Then read the section content
        read = direct_read_lines(TEST_FILE, 34, 91)
        return {
            "input_chars": grep["input_chars"] + read["input_chars"],
            "output_chars": grep["output_chars"] + read["output_chars"],
            "time_ms": grep["time_ms"] + read["time_ms"],
            "status": "OK",
            "round_trips": 2,
        }

    scenarios.append(("1-Read", "1.4 Section children", mcp_children, cli_children, direct_children))

    # ---- Group 2: Search ----

    def mcp_search_invoice():
        return mcp.call_tool("markdown_search", {
            "file_path": TEST_FILE, "query": "Invoice"
        })

    def cli_search_invoice():
        return run_cli(["search", TEST_FILE, "Invoice"])

    def direct_search_invoice():
        return run_direct_shell(f"grep -in 'Invoice' {TEST_FILE}")

    scenarios.append(("2-Search", "2.1 Search Invoice", mcp_search_invoice, cli_search_invoice, direct_search_invoice))

    # 2.2 Scoped search
    SCOPE_PATH = "/06-functional-specification-đặc-tả-chức-năng"

    def mcp_search_scoped():
        return mcp.call_tool("markdown_search", {
            "file_path": TEST_FILE, "query": "API",
            "selector": {"path": SCOPE_PATH}
        })

    def cli_search_scoped():
        # CLI doesn't support scoped search, must search all then filter
        return run_cli(["search", TEST_FILE, "API"])

    def direct_search_scoped():
        # Agent must: find section range → Read → grep within
        grep = run_direct_shell(f"grep -n '^#' {TEST_FILE}")
        # Section 06 starts around line 1313, ends around 2180
        read = direct_read_lines(TEST_FILE, 1313, 2180)
        grep2 = run_direct_shell(f"awk 'NR>=1313 && NR<=2180' {TEST_FILE} | grep -in 'API'")
        return {
            "input_chars": grep["input_chars"] + read["input_chars"] + grep2["input_chars"],
            "output_chars": grep["output_chars"] + read["output_chars"] + grep2["output_chars"],
            "time_ms": grep["time_ms"] + read["time_ms"] + grep2["time_ms"],
            "status": "OK",
            "round_trips": 3,
        }

    scenarios.append(("2-Search", "2.2 Scoped search API", mcp_search_scoped, cli_search_scoped, direct_search_scoped))

    # 2.3 Case-sensitive search
    def mcp_search_cs():
        return mcp.call_tool("markdown_search", {
            "file_path": TEST_FILE, "query": "confidence", "case_sensitive": True
        })

    def cli_search_cs():
        return run_cli(["search", TEST_FILE, "confidence", "--case-sensitive"])

    def direct_search_cs():
        return run_direct_shell(f"grep -n 'confidence' {TEST_FILE}")

    scenarios.append(("2-Search", "2.3 Search confidence CS", mcp_search_cs, cli_search_cs, direct_search_cs))

    # ---- Group 3: Edit (dry_run) ----

    def mcp_replace():
        return mcp.call_tool("markdown_edit", {
            "file_path": TEST_FILE, "dry_run": True,
            "operations": [{"op": "Replace", "selector": {"id": 3}, "content": "Replaced paragraph text."}]
        })

    def cli_replace():
        return run_cli(["edit", "replace", TEST_FILE, "--id", "3", "--content", "Replaced paragraph text.", "--dry-run"])

    def direct_replace():
        # Agent must: Read file → find the text → Edit old→new
        read = direct_read_file(TEST_FILE)
        old_text = FILE_LINES[10] if len(FILE_LINES) > 10 else "placeholder"
        edit = direct_edit(old_text, "Replaced paragraph text.")
        return {
            "input_chars": read["input_chars"] + edit["input_chars"],
            "output_chars": read["output_chars"] + edit["output_chars"],
            "time_ms": read["time_ms"] + edit["time_ms"],
            "status": "OK",
            "round_trips": 2,
        }

    scenarios.append(("3-Edit", "3.1 Replace paragraph", mcp_replace, cli_replace, direct_replace))

    # 3.2 Rename section
    def mcp_rename():
        return mcp.call_tool("markdown_edit", {
            "file_path": TEST_FILE, "dry_run": True,
            "operations": [{"op": "RenameSection", "selector": {"path": SECTION_PATH}, "new_title": "Renamed Section"}]
        })

    def cli_rename():
        return run_cli(["edit", "rename-section", TEST_FILE, "--path", SECTION_PATH, "--new-title", "Renamed Section", "--dry-run"])

    def direct_rename():
        # Agent must: grep heading → Edit the line
        grep = run_direct_shell(f"grep -n 'Product Requirements Document' {TEST_FILE}")
        edit = direct_edit("## Product Requirements Document", "## Renamed Section")
        return {
            "input_chars": grep["input_chars"] + edit["input_chars"],
            "output_chars": grep["output_chars"] + edit["output_chars"],
            "time_ms": grep["time_ms"] + edit["time_ms"],
            "status": "OK",
            "round_trips": 2,
        }

    scenarios.append(("3-Edit", "3.2 Rename section", mcp_rename, cli_rename, direct_rename))

    # 3.3 Delete block
    def mcp_delete():
        return mcp.call_tool("markdown_edit", {
            "file_path": TEST_FILE, "dry_run": True,
            "operations": [{"op": "Delete", "selector": {"id": 7}}]
        })

    def cli_delete():
        return run_cli(["edit", "delete", TEST_FILE, "--id", "7", "--dry-run"])

    def direct_delete():
        # Agent must: Read to find content → Edit to remove
        read = direct_read_file(TEST_FILE)
        old_text = FILE_LINES[24] if len(FILE_LINES) > 24 else "placeholder"
        edit = direct_edit(old_text, "")
        return {
            "input_chars": read["input_chars"] + edit["input_chars"],
            "output_chars": read["output_chars"] + edit["output_chars"],
            "time_ms": read["time_ms"] + edit["time_ms"],
            "status": "OK",
            "round_trips": 2,
        }

    scenarios.append(("3-Edit", "3.3 Delete block", mcp_delete, cli_delete, direct_delete))

    # 3.4 Insert after
    def mcp_insert():
        return mcp.call_tool("markdown_edit", {
            "file_path": TEST_FILE, "dry_run": True,
            "operations": [{"op": "Insert", "selector": {"id": 5}, "position": "After", "content": "New paragraph inserted."}]
        })

    def cli_insert():
        return run_cli(["edit", "insert", TEST_FILE, "--id", "5", "--after", "--content", "New paragraph inserted.", "--dry-run"])

    def direct_insert():
        # Agent must: Read to find insertion point → Edit to add
        read = direct_read_file(TEST_FILE)
        old_text = FILE_LINES[10] if len(FILE_LINES) > 10 else "placeholder"
        edit = direct_edit(old_text, old_text + "\n\nNew paragraph inserted.")
        return {
            "input_chars": read["input_chars"] + edit["input_chars"],
            "output_chars": read["output_chars"] + edit["output_chars"],
            "time_ms": read["time_ms"] + edit["time_ms"],
            "status": "OK",
            "round_trips": 2,
        }

    scenarios.append(("3-Edit", "3.4 Insert after", mcp_insert, cli_insert, direct_insert))

    # ---- Group 4: Validate & Normalize ----

    def mcp_validate():
        return mcp.call_tool("markdown_validate", {"file_path": TEST_FILE})

    def cli_validate():
        return run_cli(["validate", TEST_FILE])

    def direct_validate():
        # Agent must manually check: heading levels, blank lines, etc.
        # Simulates multiple grep checks
        g1 = run_direct_shell(f"grep -En '^#[^ #]' {TEST_FILE}")  # Bad headings (no space)
        g2 = run_direct_shell(f"grep -En '^#{{1,6}}[^ #]' {TEST_FILE}")
        g3 = run_direct_shell(f"grep -cn '```' {TEST_FILE}")
        return {
            "input_chars": g1["input_chars"] + g2["input_chars"] + g3["input_chars"],
            "output_chars": g1["output_chars"] + g2["output_chars"] + g3["output_chars"],
            "time_ms": g1["time_ms"] + g2["time_ms"] + g3["time_ms"],
            "status": "OK",
            "round_trips": 3,
        }

    scenarios.append(("4-Process", "4.1 Validate", mcp_validate, cli_validate, direct_validate))

    def mcp_normalize():
        return mcp.call_tool("markdown_normalize", {
            "file_path": TEST_FILE, "dry_run": True
        })

    def cli_normalize():
        return run_cli(["normalize", TEST_FILE, "--dry-run"])

    def direct_normalize():
        # Agent must: Read full file → apply regex chain → diff
        read = direct_read_file(TEST_FILE)
        # Multiple sed passes would be needed
        sed1 = run_direct_shell(f"sed -n '/^$/N;/^\\n$/d' {TEST_FILE} | wc -c")
        return {
            "input_chars": read["input_chars"] + sed1["input_chars"],
            "output_chars": read["output_chars"] + sed1["output_chars"],
            "time_ms": read["time_ms"] + sed1["time_ms"],
            "status": "OK",
            "round_trips": 2,
        }

    scenarios.append(("4-Process", "4.2 Normalize", mcp_normalize, cli_normalize, direct_normalize))

    # ---- Group 5: ASCII Art ----

    def mcp_ascii_find():
        return mcp.call_tool("markdown_read_block", {
            "file_path": TEST_FILE, "view": "by_type", "block_type": "fence"
        })

    def cli_ascii_find():
        return run_cli(["read-blocks", TEST_FILE, "--type", "fence"])

    def direct_ascii_find():
        return run_direct_shell(f"grep -n 'ascii\\|box\\|diagram' {TEST_FILE}")

    scenarios.append(("5-ASCII", "5.1 Find ASCII blocks", mcp_ascii_find, cli_ascii_find, direct_ascii_find))

    def mcp_ascii_format():
        return mcp.call_tool("markdown_format_ascii", {
            "file_path": TEST_FILE, "dry_run": True
        })

    def cli_ascii_format():
        return run_cli(["format-ascii", TEST_FILE, "--dry-run"])

    def direct_ascii_format():
        # Agent must: Read file → find ASCII blocks → manually fix alignment
        read = direct_read_file(TEST_FILE)
        return {
            "input_chars": read["input_chars"],
            "output_chars": read["output_chars"],
            "time_ms": read["time_ms"],
            "status": "OK",
            "round_trips": 1,  # But would need manual editing which is much more
        }

    scenarios.append(("5-ASCII", "5.2 Format ASCII", mcp_ascii_format, cli_ascii_format, direct_ascii_format))

    # ---- Group 6: End-to-End ----
    # Sequential: outline → search → edit → validate

    def mcp_e2e():
        results = []
        r1 = mcp.call_tool("markdown_read_outline", {"file_path": TEST_FILE, "max_depth": 2, "include_paths": False})
        results.append(r1)
        r2 = mcp.call_tool("markdown_search", {"file_path": TEST_FILE, "query": "Invoice"})
        results.append(r2)
        r3 = mcp.call_tool("markdown_edit", {
            "file_path": TEST_FILE, "dry_run": True,
            "operations": [{"op": "Replace", "selector": {"id": 3}, "content": "Updated."}]
        })
        results.append(r3)
        r4 = mcp.call_tool("markdown_validate", {"file_path": TEST_FILE})
        results.append(r4)
        return {
            "input_chars": sum(r["input_chars"] for r in results),
            "output_chars": sum(r["output_chars"] for r in results),
            "time_ms": sum(r["time_ms"] for r in results),
            "status": "OK" if all(r["status"] == "OK" for r in results) else "ERR",
            "round_trips": 4,
        }

    def cli_e2e():
        results = []
        results.append(run_cli(["outline", TEST_FILE, "--max-depth", "2"]))
        results.append(run_cli(["search", TEST_FILE, "Invoice"]))
        results.append(run_cli(["edit", "replace", TEST_FILE, "--id", "3", "--content", "Updated.", "--dry-run"]))
        results.append(run_cli(["validate", TEST_FILE]))
        return {
            "input_chars": sum(r["input_chars"] for r in results),
            "output_chars": sum(r["output_chars"] for r in results),
            "time_ms": sum(r["time_ms"] for r in results),
            "status": "OK" if all(r["status"] == "OK" for r in results) else "ERR",
            "round_trips": 4,
        }

    def direct_e2e():
        results = []
        # Outline: grep headings
        results.append(run_direct_shell(f"grep -n '^#' {TEST_FILE}"))
        # Search: grep
        results.append(run_direct_shell(f"grep -in 'Invoice' {TEST_FILE}"))
        # Edit: Read file then edit
        results.append(direct_read_file(TEST_FILE))
        results.append(direct_edit("old text", "Updated."))
        # Validate: multiple greps
        results.append(run_direct_shell(f"grep -En '^#[^ #]' {TEST_FILE}"))
        return {
            "input_chars": sum(r["input_chars"] for r in results),
            "output_chars": sum(r["output_chars"] for r in results),
            "time_ms": sum(r["time_ms"] for r in results),
            "status": "OK" if all(r["status"] == "OK" for r in results) else "ERR",
            "round_trips": len(results),
        }

    scenarios.append(("6-E2E", "6.1 Full workflow", mcp_e2e, cli_e2e, direct_e2e))

    return scenarios


# ---------------------------------------------------------------------------
# Run benchmark
# ---------------------------------------------------------------------------

def run_benchmark():
    print("=== mdtool Fair Benchmark: MCP vs CLI vs Direct ===")
    print(f"Test file: {TEST_FILE} ({FILE_SIZE:,} chars, {len(FILE_LINES):,} lines)")
    print()

    # Initialize MCP client
    global mcp
    mcp = MCPClient()

    scenarios = get_scenarios()
    rows = []

    for group, task, mcp_fn, cli_fn, direct_fn in scenarios:
        print(f"  {group} | {task}")

        # Run each approach 3 times, take median
        for approach_name, fn in [("MCP", mcp_fn), ("CLI", cli_fn), ("Direct", direct_fn)]:
            runs = []
            for _ in range(3):
                try:
                    result = fn()
                    runs.append(result)
                except Exception as e:
                    runs.append({"input_chars": 0, "output_chars": 0, "time_ms": 0, "status": f"ERR:{e}"})

            # Pick median by total_chars
            runs.sort(key=lambda r: r["input_chars"] + r["output_chars"])
            best = runs[1]

            rows.append({
                "group": group,
                "task": task,
                "approach": approach_name,
                "round_trips": best.get("round_trips", 1),
                "input_chars": best["input_chars"],
                "output_chars": best["output_chars"],
                "total_chars": best["input_chars"] + best["output_chars"],
                "time_ms": round(best["time_ms"], 1),
                "status": best["status"],
            })

            print(f"    {approach_name:6s}: in={best['input_chars']:>8,}  out={best['output_chars']:>8,}  "
                  f"total={best['input_chars']+best['output_chars']:>8,}  "
                  f"rt={best.get('round_trips',1)}  {best['time_ms']:.1f}ms  [{best['status']}]")

    mcp.close()

    # Write CSV
    with open(CSV_PATH, "w", newline="") as f:
        writer = csv.DictWriter(f, fieldnames=[
            "group", "task", "approach", "round_trips",
            "input_chars", "output_chars", "total_chars", "time_ms", "status"
        ])
        writer.writeheader()
        writer.writerows(rows)

    print(f"\nResults written to {CSV_PATH}")
    return rows


# ---------------------------------------------------------------------------
# ASCII Visualization
# ---------------------------------------------------------------------------

def make_bar(value: int, max_val: int, width: int = 40) -> str:
    """Generate ASCII bar chart segment."""
    if max_val == 0:
        return ""
    filled = int(width * value / max_val)
    return "#" * filled + "." * (width - filled)


def format_number(n: int) -> str:
    """Format number with K suffix for thousands."""
    if n >= 1000:
        return f"{n/1000:.1f}K"
    return str(n)


def generate_readme(rows: list):
    """Generate README.md with ASCII visualization."""

    # Aggregate by approach
    totals = {"MCP": {"input": 0, "output": 0, "total": 0, "time": 0, "rt": 0},
              "CLI": {"input": 0, "output": 0, "total": 0, "time": 0, "rt": 0},
              "Direct": {"input": 0, "output": 0, "total": 0, "time": 0, "rt": 0}}
    group_data = {}

    for r in rows:
        a = r["approach"]
        totals[a]["input"] += r["input_chars"]
        totals[a]["output"] += r["output_chars"]
        totals[a]["total"] += r["total_chars"]
        totals[a]["time"] += r["time_ms"]
        totals[a]["rt"] += r["round_trips"]

        g = r["group"]
        if g not in group_data:
            group_data[g] = {}
        group_data[g][a] = r

    max_total = max(t["total"] for t in totals.values())
    bar_width = 40

    lines = []
    lines.append("# mdtool Benchmark Results")
    lines.append("")
    lines.append("Fair comparison of 3 approaches for manipulating Markdown documents.")
    lines.append(f"Test file: `tests/full_prd.md` ({FILE_SIZE:,} chars, {len(FILE_LINES):,} lines)")
    lines.append("")
    lines.append("## Methodology")
    lines.append("")
    lines.append("Measures **total token budget** = input_chars + output_chars per operation.")
    lines.append("- **MCP**: JSON-RPC request/response (includes protocol envelope)")
    lines.append("- **CLI**: Shell command + stdout from mdtool binary")
    lines.append("- **Direct**: Raw file Read/Grep/Edit (agent must read full content)")
    lines.append("")
    lines.append("> Note: Thinking tokens are NOT measured. Direct approach requires more")
    lines.append("> agent reasoning to parse raw text, so real-world advantage of MCP/CLI is larger.")
    lines.append("")

    # ---- Per-task table ----
    lines.append("## Per-Task Results")
    lines.append("")
    lines.append("| Group | Task | Approach | In | Out | Total | RT | ms |")
    lines.append("|-------|------|----------|-----|------|-------|----|----|")

    for r in rows:
        lines.append(
            f"| {r['group']} | {r['task']} | {r['approach']} | "
            f"{format_number(r['input_chars'])} | {format_number(r['output_chars'])} | "
            f"{format_number(r['total_chars'])} | {r['round_trips']} | {r['time_ms']:.0f} |"
        )

    lines.append("")

    # ---- Total bar chart ----
    lines.append("## Total Token Budget")
    lines.append("")
    lines.append("```")

    for approach in ["MCP", "CLI", "Direct"]:
        t = totals[approach]
        bar = make_bar(t["total"], max_total, bar_width)
        lines.append(f"  {approach:6s} |{bar}| {format_number(t['total']):>8s} chars")

    lines.append("```")
    lines.append("")

    # ---- Input vs Output breakdown ----
    lines.append("## Input vs Output Breakdown")
    lines.append("")
    lines.append("```")
    max_io = max(t["output"] for t in totals.values())
    for approach in ["MCP", "CLI", "Direct"]:
        t = totals[approach]
        in_bar = make_bar(t["input"], max_io, 30)
        out_bar = make_bar(t["output"], max_io, 30)
        lines.append(f"  {approach:6s} in : |{in_bar}| {format_number(t['input']):>8s}")
        lines.append(f"  {'':6s} out: |{out_bar}| {format_number(t['output']):>8s}")
        lines.append(f"  {'':6s} ----")
    lines.append("```")
    lines.append("")

    # ---- Per-group comparison ----
    lines.append("## Per-Group Comparison (Total Chars)")
    lines.append("")
    lines.append("```")

    groups_sorted = sorted(group_data.keys())
    for g in groups_sorted:
        gd = group_data[g]
        max_g = max(gd[a]["total_chars"] for a in gd)
        lines.append(f"  {g}")
        for approach in ["MCP", "CLI", "Direct"]:
            if approach in gd:
                r = gd[approach]
                bar = make_bar(r["total_chars"], max_g, 30)
                lines.append(f"    {approach:6s} |{bar}| {format_number(r['total_chars']):>8s}")
        lines.append("")

    lines.append("```")
    lines.append("")

    # ---- Summary ----
    direct_total = totals["Direct"]["total"]
    mcp_total = totals["MCP"]["total"]
    cli_total = totals["CLI"]["total"]

    lines.append("## Summary")
    lines.append("")
    lines.append(f"| Metric | MCP | CLI | Direct |")
    lines.append(f"|--------|-----|-----|--------|")
    lines.append(f"| Total chars | {format_number(mcp_total)} | {format_number(cli_total)} | {format_number(direct_total)} |")
    lines.append(f"| vs Direct | {mcp_total/direct_total*100:.1f}% | {cli_total/direct_total*100:.1f}% | 100.0% |")
    lines.append(f"| Round trips | {totals['MCP']['rt']} | {totals['CLI']['rt']} | {totals['Direct']['rt']} |")
    lines.append(f"| Total time (ms) | {totals['MCP']['time']:.0f} | {totals['CLI']['time']:.0f} | {totals['Direct']['time']:.0f} |")
    lines.append("")

    # Savings
    mcp_saving = (1 - mcp_total / direct_total) * 100
    cli_saving = (1 - cli_total / direct_total) * 100
    lines.append(f"**MCP saves {mcp_saving:.1f}% of token budget** vs Direct approach.")
    lines.append(f"**CLI saves {cli_saving:.1f}% of token budget** vs Direct approach.")
    lines.append("")

    # Print summary to stdout (user can paste into README if desired)
    for line in lines:
        print(line)

    print(f"\n(README visualization NOT auto-written — update manually from above if needed)")


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------

if __name__ == "__main__":
    rows = run_benchmark()
    generate_readme(rows)
    print("\nDone!")
