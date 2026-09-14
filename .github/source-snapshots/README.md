# Pinned Swaplock generator input

Swaplock Core is private and disallows deploy keys. This private repository carries a compressed source snapshot so CI can reproduce generation without a cross-repository credential. The archive contains `LICENSE.txt`, `libraries/app`, `libraries/chain`, `libraries/protocol` and `libraries/plugins` from the exact Git commit in its name. It contains source only, no chain database, genesis secrets or build output. SHA-256 is checked before unpacking.

After a protocol change, run `snapshot-swaplock.py /path/to/swaplock-core FULL_COMMIT`, update `SWAPLOCK_CORE_COMMIT` in `ci.yml`, then regenerate the specification and bindings. Review the source commit and generated diff together. Keep only the currently pinned archive. Local development can still use `SWAPLOCK_CORE_REPO`.
