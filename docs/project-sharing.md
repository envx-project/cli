# Project sharing protocol

Project invitations and `project add-users` use protocol version 1. Clients fetch one encrypted project snapshot, decrypt that exact response locally, and submit a complete rewrap with its snapshot token. The server compares row IDs, encrypted values, metadata, membership, and recipient public keys before writing. An invitation is claimed and its recipient joins only in the transaction that successfully replaces every encrypted value. Empty projects are supported.

Preparing an invitation does not join the project or consume the code. If data, membership, or recipient keys change, acceptance returns `project_snapshot_stale`; the author must create a fresh invitation. Decryption failures and failed uploads leave the invitation unclaimed. Legacy invitations require regeneration. Old sharing clients receive an explicit upgrade error; existing memberships are preserved.

All current project members retain the existing permission to write values and manage membership. This does not introduce owner or administrator roles. Ordinary variable writes retain their existing API and do not gain snapshot preconditions; an older writer can still upload ciphertext it prepared before a membership change. Snapshot protection applies to these invitation and add-users operations.

The server stores at most 32 active invitations and 16 MiB of active invitation ciphertext per author. Each invitation payload is limited to 1 MiB and expires after one hour. Creating another invitation clears that author's expired ciphertext. Accepted invitations clear their ciphertext immediately.

To run the real CLI/API regression, execute `tests/project_invites_e2e.py` inside Docker with a disposable API and `ENVX_TEST_API` set. The script refuses to run outside Docker and creates all key/config profiles under the container's temporary directory.
