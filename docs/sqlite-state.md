# Local operational state

Starting with the SQLite release, envx automatically opens `~/.config/envx/state.sqlite` on the first command requiring configuration. No migration command is needed for existing envx profiles. The database uses transactional schema upgrades, a 10-second writer wait, private Unix permissions, and account/server scoped records. A future schema version is rejected with an upgrade instruction.

Human settings remain in `config.json`. Private/public key files and OS keyring entries stay where they are. SQLite contains project links, encrypted cache blobs, update metadata, and friend/message operational records; it is not an encryption layer. Cache bytes remain OpenPGP encrypted. Scope is the normalized API base URL and signing-key fingerprint, so setting a UUID after authentication does not strand state. Different accounts and servers cannot read each other's scoped records. Update-check metadata is global.

## Upgrade and recovery

The importer understands released v2.0 fingerprint-plus-keys configs, v2.6 object-key configs, and the current v2.13 shape. Optional absent fields are accepted. Unknown JSON fields are retained on subsequent settings writes. Missing or malformed required data produces an error without overwriting the source. Legacy project links and the completion marker enter SQLite in one transaction. A crash or concurrent startup cannot import a partial set. Existing duplicate directory links keep their first entry, matching the old lookup behavior.

`config.pre-sqlite.json` preserves the exact original bytes. Legacy `config.json` project entries, `version.json`, and encrypted cache files are retained. The old cache cannot prove its originating server or account, so envx intentionally fetches a fresh scoped cache online instead of trusting it. New cache writes only enter SQLite. Expiring OS-keyring sessions keep their timestamps in SQLite too; untrusted old `/tmp` expiry files are not imported, so those sessions require one unlock after upgrading. OS credentials and configured password sources remain unchanged. Old JSON project entries are never reimported after a successful migration, including after unlinking.

A pre-SQLite binary still sees the preserved legacy links and caches. Changes made with it do not update SQLite, and newer changes are not written back into its JSON. Downgrading therefore restores the pre-upgrade operational snapshot, not the current state. Keep `config.json`, `config.pre-sqlite.json`, `state.sqlite`, and the key directory together for recovery. Do not delete SQLite to "refresh" state: that also loses pinned trust and pending redemption records.

For the earlier `~/.config/envcli` directory, `envx config migrate` copies keys before publishing settings, retains the source, refuses to overwrite an established different profile, and safely retries an identical import. This historical command remains explicit to avoid choosing between two existing profiles.

Do not sync a live SQLite file between machines. For a consistent backup, stop envx commands first or use SQLite's online backup API. Pins and aliases are local to this machine; copying server results is not a substitute for trust transfer.

## Verification

Run `tests/state-migration-docker.sh`. Compilation runs on the host; every executable test runs inside a disposable network-disabled Docker container with a synthetic HOME. The suite covers real CLI first launch for historical fixtures, concurrent migration/fresh initialization, repeated starts after unlink, retained unknown fields and original files, future schema rejection, killed-writer rollback, malformed configuration, legacy envcli retry, encrypted cache round trips, scoped records, and SQLite full-database rollback.
