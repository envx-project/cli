"""CLI output smoke test with a synthetic home and loopback API; run in Docker.

ENVX_TEST_COMMAND selects the built CLI (and optional loader arguments).
"""
import http.server
import json
import os
from pathlib import Path
import shlex
import sqlite3
import subprocess
import tempfile
import threading

COMMAND = shlex.split(os.environ.get("ENVX_TEST_COMMAND", "envx"))
HOME = tempfile.TemporaryDirectory(prefix="envx-output-")
os.environ.update(HOME=HOME.name, USERPROFILE=HOME.name)
OWNER = "11111111-1111-4111-8111-111111111111"
PEER = "22222222-2222-4222-8222-222222222222"
PROJECT = "33333333-3333-4333-8333-333333333333"
STAMP = "2026-09-25T06:57:22Z"
users = []


def run(*args):
    result = subprocess.run(COMMAND + list(args), input="", capture_output=True, text=True)
    assert result.returncode == 0, result.stderr
    return result.stdout


class API(http.server.BaseHTTPRequestHandler):
    def log_message(self, *args):
        pass

    def reply(self, body):
        self.send_response(200)
        self.send_header("Content-Type", "application/json")
        self.end_headers()
        self.wfile.write(json.dumps(body).encode())

    def do_GET(self):
        if self.path.startswith("/v2/friends"):
            self.reply([dict(user=dict(id=PEER, username="account", public_key="key", fingerprint="pin"), created_at=STAMP, receipts=[])])
        elif self.path.startswith("/v2/messages"):
            self.reply([dict(id=PROJECT, sender_id=PEER, recipient_id=OWNER, created_at=STAMP, expires_at=None, sender_public_key="", recipient_public_key="", ciphertext=None)])
        elif self.path == f"/v2/project/{PROJECT}":
            self.reply(dict(project_id=PROJECT, project_name="Fixture project", users=users))
        elif self.path == f"/project/{PROJECT}/variables":
            self.reply([])
        else:
            self.send_error(404)

    def do_POST(self):
        size = int(self.headers.get("Content-Length", 0))
        if size:
            self.rfile.read(size)
        if self.path == "/test-auth":
            self.reply(dict(diagnostic="server response"))
        elif self.path == "/variables/update-many":
            self.reply([])
        else:
            self.send_error(404)

    def do_DELETE(self):
        body = json.loads(self.rfile.read(int(self.headers["Content-Length"])))
        assert body["user_ids"] == [PEER]
        self.reply(None)


server = http.server.ThreadingHTTPServer(("127.0.0.1", 0), API)
threading.Thread(target=server.serve_forever, daemon=True).start()
run("gen", "--username", "fixture", "--passphrase", "fixture-password", "--no-upload", "--json")
config_path = Path(HOME.name) / ".config/envx/config.json"
config = json.loads(config_path.read_text())
config.update(primary_key_password="fixture-password", sdk_url=f"http://127.0.0.1:{server.server_port}")
config["primary_key"]["uuid"] = OWNER
config_path.write_text(json.dumps(config))
public_key = run("export")
users.extend([dict(id=OWNER, username="owner", created_at=STAMP, public_key=public_key), dict(id=PEER, username="account", created_at=STAMP, public_key="key")])
run("inbox")
scope = f'{config["sdk_url"]}|{OWNER}|{config["primary_key"]["fingerprint"]}'
with sqlite3.connect(config_path.parent / "state.sqlite") as database:
    database.execute("INSERT INTO records VALUES (?, 'friend', ?, ?)", (scope, PEER, json.dumps(dict(user_id=PEER, fingerprint="pin", public_key="key", alias="Teammate")).encode()))

for args in [("project", "list-users"), ("project", "info"), ("get", "project")]:
    output = run(*args, "--project-id", PROJECT)
    assert "Teammate (@account) · 22222222" in output, output
    assert PEER not in output and "BEGIN PGP" not in output, output
    verbose = run(*args, "--project-id", PROJECT, "--verbose")
    assert PEER in verbose and "Teammate (@account)" in verbose
    raw = json.loads(run(*args, "--project-id", PROJECT, "--json"))
    assert "Teammate" not in json.dumps(raw)

assert "From Teammate (@account)" in run("inbox")
assert "Teammate (@account) · 22222222 · trusted" in run("friends")
assert "Fingerprint: pin" in run("friends", "--verbose")
assert run("auth").strip() == "Authenticated successfully."
assert "server response" in run("auth", "--verbose")
for flags in [[], ["--verbose"], ["--json"]]:
    output = run("project", "remove-user", "--project-id", PROJECT, "--user-id", PEER, "--yes", *flags)
    assert "BEGIN PGP" not in output
    if "--json" in flags:
        assert json.loads(output)["removed_user_ids"] == [PEER]
    else:
        assert "Removed Teammate (@account) from Fixture project. Re-encrypted 0 variables." in output
        assert (PEER in output) == ("--verbose" in flags)
server.shutdown()
print("PASS: aliases across project and social output, concise summaries, verbose details, unchanged JSON")
