# Queued: SQLite operational state

Status: queued for design, not implemented. Follow the API authorization and atomic overwrite fixes; establish storage before implementing friends and messages.

Keep human-edited settings (API URL, password source, UI preferences) in config.json initially. Leave private keys and OS keyring credentials in their existing stores. Move machine-managed records to state.sqlite: friend aliases and pinned full public keys, link history and pending redemption receipts, project links, update-check metadata, and encrypted cache blobs. Cache ciphertext must remain encrypted; SQLite is not an encryption layer.

Use ordered schema migrations and an application-owned schema version. Run pending migrations and version updates transactionally under a writer lock; refuse to write schemas newer than the client understands. Enable foreign keys on every connection, bounded busy timeout, private file permissions, and crash-recovery tests. Keep a StateStore boundary so commands do not depend on SQL layout.

One-time legacy import:

1. Inventory supported historical JSON shapes from release history, not just today's Rust structs. Decode into tolerant legacy DTOs, validate into the current domain, and preserve unknown configuration fields.
2. Scope records by normalized API origin and local account UUID/fingerprint. Existing project/cache files lack complete account/server identity; do not guess for ambiguous records. Rebuild disposable caches when provenance cannot be established.
3. Import state and a completion marker in one transaction. Preserve original files. Retry after interruption without duplicates; do not continuously dual-write JSON and SQLite.
4. Test fixtures from each supported release, missing optional fields, malformed/truncated files, concurrent first launch, interruption, disk-full failures, and newer-schema rejection.
5. Choose a compatibility cutoff for pre-SQLite binaries. They will still write legacy JSON and cannot be made aware of the new database retroactively. Document downgrade limits and provide explicit import/export rather than silently merging diverged stores.

Local database only: do not use a live SQLite file as a multi-machine sync protocol. In particular WAL requires same-host access. Use SQLite backup APIs or consistent snapshots for backups; design cross-device trust export/import separately.

Involvement: requires intervention for the supported old-version/downgrade window and cross-device expectations. Agent research can determine historical config shapes and recommend the narrowest safe importer; Alex decides the compatibility policy. Blast radius: local persisted state and trust pins. No existing user state is to be migrated until that policy and recovery behavior are specified.

References: https://www.sqlite.org/lang_altertable.html ; https://www.sqlite.org/pragma.html#pragma_user_version ; https://www.sqlite.org/wal.html
