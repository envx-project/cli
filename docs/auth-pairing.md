# Pair an existing identity with another machine

On the machine where envx already works:

```sh
envx auth link
```

Keep that terminal open. Copy the printed `envx auth login 'https://…'` command
and run it on the new machine. The invitation contains a server URL and a random
session identifier, not your identity or a decryption secret. It can be sent
through a messaging service; someone with the invitation can race to request
pairing, but cannot obtain your identity merely by possessing the invitation.

The new machine shows a verification code. Copy that complete code into the
original machine's waiting terminal. Only enter a code from a terminal you
control. This explicitly approves that receiving machine. A mismatch cancels
the transfer before any identity material is sent. If someone else claims your
invitation first, cancel and generate a fresh one.

The new machine asks for your existing identity passphrase. The passphrase is
never transferred. Login verifies the identity and authenticates it against the
server before installing it. Existing identities and key files are not replaced.
The OS keyring may cache the passphrase, just as it does for `envx gen`; it is
never saved into `config.json` by login. Run `envx link` to connect a local project
directory after login.

Both machines need the pairing-capable CLI and API. Pairings expire after ten
minutes. Ctrl-C cancels a pending transfer. Interrupted transfers can be restarted
with a fresh invitation. If installation fails after key files are written, the
passphrase-protected files are retained. A fresh pairing can resume installation
when those files match the transferred identity exactly; different files are never
overwritten or deleted. Restoring an identity does not copy machine-local configuration,
project directory links, aliases or trust pins.

## Security contract

The clients use `Noise_XX_25519_ChaChaPoly_SHA256` via `snow`, with fresh handshake
keys for every pairing and the server URL and session identifier bound into the prologue. The
verification code is the complete 256-bit handshake hash. The source releases the
identity only after the user enters the receiving terminal's exact code. The
server does not receive that code. Do not accept a code supplied by a stranger.

The server relays bounded opaque handshake messages and an authenticated encrypted
identity bundle. The original OpenPGP private key remains passphrase-protected
inside that bundle. The bundle contains no passphrase. A receiver capability is
generated locally and hashed in server storage; it is not included in the link.
A database snapshot or invitation alone cannot decrypt a transfer. A malicious
relay can interrupt or substitute a peer, but substitution changes the verification
code and must be rejected by the user. This is not protection against a compromised
endpoint or a user approving an attacker's terminal.

One receiver may claim each invitation. Active sessions are capped at three per
account, twelve per client IPv4 address or IPv6 /64, and 1,024 globally. Server transitions are atomic, source
operations require the existing account, receiver operations require its separate
capability, and acknowledgement or cancellation deletes the relay row. A periodic
sweep removes expired rows; the ten-minute access deadline is enforced on requests,
not only by cleanup. Backups may retain ciphertext but have no Noise decryption key.

The new machine receives the same private identity. It has the same authority as
the original machine; this does not introduce independently revocable device keys.

## Commands

`auth status` checks server authentication. `auth gen`, `auth register`, and
`auth export` group the existing identity commands. Legacy `envx auth`, `gen`,
`upload`, and `export` remain supported. Project invites are unchanged.

Protocol references: https://noiseprotocol.org/noise.html and https://docs.rs/snow/0.10.0/.
