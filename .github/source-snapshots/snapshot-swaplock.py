#!/usr/bin/env python3
"""Export exact tracked generator inputs, never a dirty checkout or submodules."""
import gzip
import hashlib
from pathlib import Path
import subprocess
import sys

repo, revision = sys.argv[1:]
commit = subprocess.check_output(["git", "-C", repo, "rev-parse", revision + "^{commit}"], text=True).strip()
archive = subprocess.check_output([
    "git", "-C", repo, "archive", "--format=tar", commit,
    "LICENSE.txt", "libraries/app", "libraries/chain", "libraries/protocol", "libraries/plugins",
])
path = Path(__file__).parent / ("swaplock-" + commit + ".tar.gz")
path.write_bytes(gzip.compress(archive, mtime=0))
path.with_suffix(path.suffix + ".sha256").write_text(hashlib.sha256(path.read_bytes()).hexdigest() + "  " + path.name + "\n")
print(path.name)
