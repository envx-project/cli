#!/usr/bin/env python3
"""Interactive secret-sharing checks, exclusively inside a disposable Docker profile."""
import datetime
import fcntl
import json
import os
from pathlib import Path
import pty
import re
import select
import shlex
import signal
import sqlite3
import struct
import subprocess
import tempfile
import termios
import time

assert Path('/.dockerenv').exists(), 'Run only inside Docker'
command = shlex.split(os.environ.get('ENVX_TEST_COMMAND', '/opt/envx-libs/ld-linux-x86-64.so.2 --library-path /opt/envx-libs /opt/envx'))
reuse = os.environ.get('ENVX_PTY_PROFILE_ROOT')
root = Path(reuse) if reuse else Path(tempfile.mkdtemp(prefix='envx-pty-'))
server = os.environ.get('ENVX_TEST_API', 'http://envx-shift-api:3000')

def env(user):
    return {**os.environ, 'HOME': str(root / user), 'TERM': 'xterm-256color'}

def run(user, *args, data=None):
    result = subprocess.run(command + list(args), env=env(user), cwd=root / user, input=data, text=True, capture_output=True)
    assert result.returncode == 0, (args[0], result.stderr)
    return result.stdout

def doc(user, *args, **kwargs):
    return json.loads(run(user, *args, **kwargs))

users = {}
if not reuse:
    for user in ('alice', 'bob'):
        (root / user).mkdir()
        doc(user, 'gen', '--username', user, '--passphrase', 'synthetic-pty-passphrase', '--no-upload', '--json')
        path = root / user / '.config/envx/config.json'
        value = json.loads(path.read_text())
        value.update(primary_key_password='synthetic-pty-passphrase', sdk_url=server)
        path.write_text(json.dumps(value))
        run(user, 'upload', '--username', user)
        users[user] = json.loads(path.read_text())['primary_key']['uuid']
        db = sqlite3.connect(root / user / '.config/envx/state.sqlite')
        state = json.dumps({'last_update_check': datetime.datetime.now(datetime.timezone.utc).isoformat(), 'latest_version': None}).encode()
        db.execute("INSERT OR REPLACE INTO records VALUES('global','updates','latest',?)", (state,)); db.commit(); db.close()
    
    link = doc('alice', 'friend-link', '--json')
    doc('bob', 'add-friend', link['code'], '--alias', 'alice', '--json')
    doc('alice', 'friends', '--rename', users['bob'], '--alias', 'bob', '--json')
else:
    for user in ('alice', 'bob'):
        users[user] = json.loads((root / user / '.config/envx/config.json').read_text())['primary_key']['uuid']
doc('alice', 'friends', '--rename', users['bob'], '--alias', 'local-bob', '--json')

class Terminal:
    def __init__(self, user, *args):
        self.pid, self.fd = pty.fork()
        if self.pid == 0:
            os.chdir(root / user)
            os.execvpe(command[0], command + list(args), env(user))
        fcntl.ioctl(self.fd, termios.TIOCSWINSZ, struct.pack('HHHH', 40, 160, 0, 0))
        self.output = b''
        self.status = None
    def pump(self, seconds=0.2):
        end = time.monotonic() + seconds
        while time.monotonic() < end:
            ready, _, _ = select.select([self.fd], [], [], min(0.1, max(0, end-time.monotonic())))
            if not ready: continue
            try: chunk = os.read(self.fd, 65536)
            except OSError: break
            self.output += chunk
            # Crossterm asks for cursor position before rendering.
            if b'\x1b[6n' in chunk: os.write(self.fd, b'\x1b[1;1R')
        pid, status = os.waitpid(self.pid, os.WNOHANG)
        if pid: self.status = status
    def until(self, text, timeout=20):
        deadline = time.monotonic()+timeout
        while text.encode() not in self.output:
            self.pump()
            if self.status is not None: raise AssertionError(('exited before prompt', text, self.output.decode(errors='replace')))
            assert time.monotonic()<deadline, ('prompt timeout', text, self.output.decode(errors='replace'))
    def send(self, value): os.write(self.fd, value)
    def finish(self):
        deadline = time.monotonic()+25
        while self.status is None:
            self.pump()
            if time.monotonic()>deadline:
                os.kill(self.pid, signal.SIGKILL); raise AssertionError('PTY command timed out')
        os.close(self.fd)
        return os.waitstatus_to_exitcode(self.status)

before = doc('bob', 'inbox', '--json')
terminal = Terminal('alice', 'send', 'local-bob')
terminal.until('Secret:')
secret = 'synthetic-hidden-pty-secret-7751'
terminal.send(secret.encode()+b'\r')
terminal.pump(1)
assert secret.encode() not in terminal.output, 'Secret echoed in terminal'
assert b'Confirmation' not in terminal.output, 'Send should accept one hidden entry'
assert terminal.finish() == 0
assert secret.encode() not in terminal.output, 'Secret echoed after submission'
messages = doc('bob', 'inbox', '--json')
assert len(messages) == len(before)+1
message = next(item for item in messages if item['id'] not in {x['id'] for x in before})
assert run('bob', 'read', message['id']) == secret

terminal = Terminal('alice', 'send', 'local-bob')
terminal.until('Secret:')
terminal.send(b'synthetic-cancelled-secret\x03')
assert terminal.finish() == 0
assert b'synthetic-cancelled-secret' not in terminal.output
assert len(doc('bob', 'inbox', '--json')) == len(messages)

terminal = Terminal('alice', 'friends')
terminal.until('Friend options')
assert b'local-bob' in terminal.output and b'trusted' in terminal.output
assert b'local-bob' in terminal.output.split(b'Friend options', 1)[1], 'Menu should show the local alias'
assert secret.encode() not in terminal.output
terminal.send(b'\r')
assert terminal.finish() == 0

terminal = Terminal('alice', 'send')
assert terminal.finish() != 0
assert b"FRIEND" in terminal.output and b"Usage:" in terminal.output
assert secret.encode() not in run('bob', 'inbox').encode()
print('PASS: PTY hidden entry and cancel, friend menu, missing-argument guidance, metadata-only inbox')
print('FIXTURE_ROOT='+str(root))
