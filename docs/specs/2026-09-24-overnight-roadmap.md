# Envx overnight delivery

User authorized complete implementation, Docker-isolated automatic SQLite migration, full two-model Comb/autofix over backend/frontend/CLI, realistic CLI testing, signed landing and publication including generated release PR.

1. Reproduce and fix invite redemption, variable ownership, atomic overwrite and secret-preview defects. Verify PostgreSQL regressions.
2. Parallel foundation: SQLite state/import + friends/messages API + CLI workflows. Keep settings JSON and key material in existing stores; no host profile testing.
3. Integrate versioned state, signed receipts/messages, immutable key pins and scoped identities. Include migration compatibility fixes and full-codebase verified findings.
4. Build/run entirely disposable Docker homes/databases. Exercise released prior version to new version migration twice, concurrency, no-effort upgrade, friend links, send/read/import, failed/expired/unauthorized operations. Try interactive and scripted CLI paths.
5. Comb baseline whole codebase using four independent OpenAI/Claude lenses. Apply verified fixes; review final feature diffs and one fix re-comb per land workflow.
6. Land API, verify migration deployment, then CLI; merge refreshed release PR and verify every binary asset. Publish website documentation/polish without overwriting existing local edits.

No production destructive tests, no secret output, no publishing unverified migrations. User allows ordinary product decisions while asleep. Consult github-copilot/claude-fable-5.1 through opencode if blocked.
