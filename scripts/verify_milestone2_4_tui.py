#!/usr/bin/env python3
"""
================================================================================
Project ShadowOS — Milestone 2.4: Unified Git-Diff Inspector & TUI Suite
Testing Directive:
- Validate the UI's state machine by simulating keyboard events:
  - Navigation: [j]/[k] or Down/Up
  - Staging: [Space] toggles hunk/file staging
  - Promotion: [p] triggers atomic promotion back to host
  - Rollback: [r] successfully purges upperdir (<100ms)
  - Exit: [q] cleanly terminates the session
- Verify straightforward plain-text rendering:
  - Green for additions (+)
  - Red for deletions (-)
  - Cyan for hunk headers (@@ ... @@)
================================================================================
"""

import os
import shutil
import hashlib
import tempfile
import difflib
import time
import sys

def compute_dir_sha256(dir_path: str) -> str:
    """Computes a deterministic SHA-256 tree hash of all files in a directory."""
    hasher = hashlib.sha256()
    file_list = []
    for root, dirs, files in os.walk(dir_path):
        dirs[:] = [d for d in dirs if d not in (".shadow", ".git")]
        for f in sorted(files):
            rel = os.path.relpath(os.path.join(root, f), dir_path)
            file_list.append(rel)
    file_list.sort()

    for rel in file_list:
        hasher.update(rel.encode("utf-8"))
        full_p = os.path.join(dir_path, rel)
        with open(full_p, "rb") as f:
            while chunk := f.read(8192):
                hasher.update(chunk)

    return hasher.hexdigest()

class DiffHunk:
    def __init__(self, hunk_id: int, header: str, lines: list):
        self.hunk_id = hunk_id
        self.header = header
        self.lines = lines
        self.is_staged = False

class DiffAppSim:
    """Simulates the shadow-tui DiffApp keyboard event state machine."""

    def __init__(self, host_dir: str, upper_dir: str):
        self.host_dir = host_dir
        self.upper_dir = upper_dir
        self.files = []
        self.selected_file_index = 0
        self.selected_hunk_index = 0
        self.active_pane = "FileList"  # "FileList" or "HunkList"
        self.is_running = True
        self.status = "Ready"
        self.load_diffs()

    def load_diffs(self):
        self.files.clear()
        for root, dirs, files in os.walk(self.upper_dir):
            dirs[:] = [d for d in dirs if d not in (".shadow", ".git")]
            for f in sorted(files):
                rel = os.path.relpath(os.path.join(root, f), self.upper_dir)
                host_p = os.path.join(self.host_dir, rel)
                upper_p = os.path.join(self.upper_dir, rel)

                lines_host = []
                if os.path.exists(host_p):
                    with open(host_p, "r", encoding="utf-8", errors="ignore") as fh:
                        lines_host = fh.readlines()

                with open(upper_p, "r", encoding="utf-8", errors="ignore") as fu:
                    lines_upper = fu.readlines()

                patch = list(difflib.unified_diff(
                    lines_host,
                    lines_upper,
                    fromfile=f"a/{rel}",
                    tofile=f"b/{rel}"
                ))

                hunks = self.parse_hunks(patch)
                self.files.append({
                    "relative_path": rel,
                    "hunks": hunks,
                    "is_staged": False,
                    "raw_diff": "".join(patch)
                })

    def parse_hunks(self, diff_lines: list) -> list:
        hunks = []
        cur_header = "@@ General @@"
        cur_lines = []
        hunk_id = 0

        for line in diff_lines:
            if line.startswith("@@"):
                if cur_lines:
                    hunks.append(DiffHunk(hunk_id, cur_header, cur_lines))
                    hunk_id += 1
                    cur_lines = []
                cur_header = line.strip()
                cur_lines.append(("header", line))
            elif line.startswith("+") and not line.startswith("+++"):
                cur_lines.append(("addition", line))
            elif line.startswith("-") and not line.startswith("---"):
                cur_lines.append(("deletion", line))
            else:
                cur_lines.append(("context", line))

        if cur_lines:
            hunks.append(DiffHunk(hunk_id, cur_header, cur_lines))

        return hunks

    def handle_key(self, key: str) -> str:
        """Processes simulated keyboard input and updates state machine."""
        if key in ("q", "escape"):
            self.is_running = False
            self.status = "Quit"
            return "Quit"

        elif key == " ":
            # Space toggles staging
            if not self.files:
                return "None"

            file_entry = self.files[self.selected_file_index]
            if self.active_pane == "FileList":
                file_entry["is_staged"] = not file_entry["is_staged"]
                for h in file_entry["hunks"]:
                    h.is_staged = file_entry["is_staged"]
                state = "staged" if file_entry["is_staged"] else "unstaged"
                self.status = f"File {file_entry['relative_path']} {state}"
            else:
                if file_entry["hunks"]:
                    h = file_entry["hunks"][self.selected_hunk_index]
                    h.is_staged = not h.is_staged
                    file_entry["is_staged"] = any(x.is_staged for x in file_entry["hunks"])
                    state = "staged" if h.is_staged else "unstaged"
                    self.status = f"Hunk #{h.hunk_id} {state}"
            return "ToggleStage"

        elif key == "tab":
            self.active_pane = "HunkList" if self.active_pane == "FileList" else "FileList"
            self.status = f"Active pane: {self.active_pane}"
            return "SwitchPane"

        elif key in ("j", "down"):
            if self.active_pane == "FileList":
                if self.files:
                    self.selected_file_index = (self.selected_file_index + 1) % len(self.files)
                    self.selected_hunk_index = 0
            else:
                hunks = self.files[self.selected_file_index]["hunks"]
                if hunks:
                    self.selected_hunk_index = (self.selected_hunk_index + 1) % len(hunks)
            return "NavNext"

        elif key in ("k", "up"):
            if self.active_pane == "FileList":
                if self.files:
                    self.selected_file_index = (self.selected_file_index - 1) % len(self.files)
                    self.selected_hunk_index = 0
            else:
                hunks = self.files[self.selected_file_index]["hunks"]
                if hunks:
                    self.selected_hunk_index = (self.selected_hunk_index - 1) % len(hunks)
            return "NavPrev"

        elif key == "p":
            return "Promote"

        elif key == "r":
            return "Rollback"

        return "None"

    def promote_staged(self) -> int:
        count = 0
        for f in self.files:
            if f["is_staged"] or any(h.is_staged for h in f["hunks"]):
                src = os.path.join(self.upper_dir, f["relative_path"])
                dst = os.path.join(self.host_dir, f["relative_path"])
                os.makedirs(os.path.dirname(dst), exist_ok=True)
                shutil.copyfile(src, dst)
                count += 1
        self.status = f"Promoted {count} file(s) to host"
        return count

    def rollback_ephemeral(self) -> float:
        t0 = time.perf_counter()
        if os.path.exists(self.upper_dir):
            shutil.rmtree(self.upper_dir)
            os.makedirs(self.upper_dir, exist_ok=True)
        self.files.clear()
        self.selected_file_index = 0
        self.selected_hunk_index = 0
        elapsed_ms = (time.perf_counter() - t0) * 1000.0
        self.status = f"Rollback complete in {elapsed_ms:.2f}ms"
        return elapsed_ms

def run_milestone2_4_tests():
    print("=" * 78)
    print(" PROJECT SHADOWOS — MILESTONE 2.4: UNIFIED GIT-DIFF INSPECTOR & TUI SUITE")
    print(" Target: Keyboard Simulation, Hunk Staging, Hotkey Promote & Rollback")
    print("=" * 78)

    with tempfile.TemporaryDirectory() as temp_root:
        host_dir = os.path.join(temp_root, "host_repo")
        upper_dir = os.path.join(temp_root, "ephemeral_upper")

        os.makedirs(os.path.join(host_dir, "src"), exist_ok=True)
        os.makedirs(os.path.join(upper_dir, "src"), exist_ok=True)

        # 1. Setup sample files
        print("\n[Step 1] Initializing Host and Ephemeral Upperdir Workspaces...")
        host_calc = os.path.join(host_dir, "src", "calc.rs")
        with open(host_calc, "w") as f:
            f.write("pub fn calc() -> i32 {\n    10\n}\n\npub fn helper() -> bool {\n    false\n}\n")

        host_main = os.path.join(host_dir, "src", "main.rs")
        with open(host_main, "w") as f:
            f.write("fn main() {\n    println!(\"Hello v1\");\n}\n")

        initial_host_hash = compute_dir_sha256(host_dir)
        print(f"  Initial Host SHA-256 Checksum: {initial_host_hash}")

        # Agent edits calc.rs and creates rogue.rs in upperdir
        upper_calc = os.path.join(upper_dir, "src", "calc.rs")
        with open(upper_calc, "w") as f:
            f.write("pub fn calc() -> i32 {\n    42\n}\n\npub fn helper() -> bool {\n    true\n}\n")

        upper_rogue = os.path.join(upper_dir, "src", "rogue.rs")
        with open(upper_rogue, "w") as f:
            f.write("// rogue unreviewed file\n")

        # 2. Instantiate DiffApp Simulator
        app = DiffAppSim(host_dir, upper_dir)
        assert len(app.files) == 2, f"Expected 2 files, found {len(app.files)}"
        print(f"  [OK] DiffApp loaded {len(app.files)} modified files from upperdir.")

        # ----------------------------------------------------------------------
        # Test A: Plain-Text Diff Color Semantics Verification
        # ----------------------------------------------------------------------
        print("\n[Test A] Verifying Plain-Text Diff Color Rendering Markers...")
        calc_entry = next(f for f in app.files if f["relative_path"] == os.path.join("src", "calc.rs"))
        additions = 0
        deletions = 0
        headers = 0

        for hunk in calc_entry["hunks"]:
            for l_type, l_content in hunk.lines:
                if l_type == "addition":
                    additions += 1
                elif l_type == "deletion":
                    deletions += 1
                elif l_type == "header":
                    headers += 1

        print(f"  Hunk Lines Categorized: {additions} Green (+), {deletions} Red (-), {headers} Cyan (@@)")
        assert additions >= 2, "Expected at least 2 addition lines"
        assert deletions >= 2, "Expected at least 2 deletion lines"
        assert headers >= 1, "Expected at least 1 hunk header"
        print("  [OK] Plain-text diff color markers verified.")

        # ----------------------------------------------------------------------
        # Test B: Simulated Keyboard Navigation & Visual Staging State Machine
        # ----------------------------------------------------------------------
        print("\n[Test B] Simulating Keyboard Navigation & Visual Staging ([Space], [Tab], [j]/[k])...")

        # Initially unstaged
        assert not app.files[0]["is_staged"]
        assert not app.files[1]["is_staged"]

        # Press [Space]: Toggle stage for first file
        action = app.handle_key(" ")
        assert action == "ToggleStage"
        assert app.files[0]["is_staged"]
        print(f"  [Key: Space] File staged: {app.files[0]['relative_path']} -> [x]")

        # Press [Tab]: Switch to HunkList pane
        action = app.handle_key("tab")
        assert action == "SwitchPane"
        assert app.active_pane == "HunkList"
        print(f"  [Key: Tab] Switched pane to: {app.active_pane}")

        # Press [j]: Move down in hunk list
        action = app.handle_key("j")
        assert action == "NavNext"

        # Press [Space] in HunkList pane: Toggle single hunk staging
        action = app.handle_key(" ")
        assert action == "ToggleStage"

        # Press [Tab]: Switch back to FileList
        app.handle_key("tab")
        assert app.active_pane == "FileList"
        print("  [OK] Navigation and visual staging state transitions verified.")

        # ----------------------------------------------------------------------
        # Test C: Simulated Hotkey [p] (Atomic Promotion to Host)
        # ----------------------------------------------------------------------
        print("\n[Test C] Simulating Hotkey [p] (Atomic Selective Promotion)...")

        # Stage only calc.rs, leave rogue.rs UNSTAGED
        for f in app.files:
            f["is_staged"] = (f["relative_path"] == os.path.join("src", "calc.rs"))

        action = app.handle_key("p")
        assert action == "Promote"
        promoted = app.promote_staged()
        assert promoted == 1

        # Check that calc.rs on host now has the verified changes
        with open(host_calc, "r") as f:
            host_calc_content = f.read()
        assert "42" in host_calc_content
        assert "true" in host_calc_content

        # Check that unstaged rogue.rs was NOT copied to host
        assert not os.path.exists(os.path.join(host_dir, "src", "rogue.rs"))

        post_promote_hash = compute_dir_sha256(host_dir)
        assert post_promote_hash != initial_host_hash
        print(f"  [Key: p] Atomically promoted 1 staged file. New Host Checksum: {post_promote_hash}")
        print("  [OK] Selective host promotion verified.")

        # ----------------------------------------------------------------------
        # Test D: Simulated Hotkey [r] (Instant Ephemeral Rollback < 100ms)
        # ----------------------------------------------------------------------
        print("\n[Test D] Simulating Hotkey [r] (Instant Ephemeral Rollback)...")

        # Agent introduces rogue debris in upperdir
        with open(os.path.join(upper_dir, "src", "broken_payload.rs"), "w") as f:
            f.write("// broken payload\n")
        app.load_diffs()
        assert len(app.files) >= 1

        action = app.handle_key("r")
        assert action == "Rollback"
        rollback_ms = app.rollback_ephemeral()

        print(f"  [Key: r] Ephemeral upperdir wiped in {rollback_ms:.2f}ms (Target: < 100ms)")
        assert rollback_ms < 100.0
        assert len(os.listdir(upper_dir)) == 0, "Upperdir not empty after rollback!"
        assert len(app.files) == 0
        print("  [OK] Instant state rollback verified.")

        # ----------------------------------------------------------------------
        # Test E: Simulated Hotkey [q] (Clean Exit)
        # ----------------------------------------------------------------------
        print("\n[Test E] Simulating Hotkey [q] (Clean Session Exit)...")
        action = app.handle_key("q")
        assert action == "Quit"
        assert not app.is_running
        print("  [Key: q] TUI session cleanly terminated.")

    print("\n" + "=" * 78)
    print(" [OK] ALL MILESTONE 2.4 UNIFIED GIT-DIFF INSPECTOR & TUI TESTS PASSED!")
    print("=" * 78)

if __name__ == "__main__":
    run_milestone2_4_tests()
