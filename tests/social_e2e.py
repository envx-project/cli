#!/usr/bin/env python3
"""Real CLI/API social flow. Run ONLY inside Docker against a disposable API."""
import json
import os
from pathlib import Path
import shlex
import sqlite3
import subprocess
import tempfile

assert Path('/.dockerenv').exists(), 'This test must run inside Docker'
command = shlex.split(os.environ.get('ENVX_TEST_COMMAND', '/opt/envx-libs/ld-linux-x86-64.so.2 --library-path /opt/envx-libs /opt/envx'))
server = os.environ.get('ENVX_TEST_API', 'http://envx-shift-api:3000')
root = Path(tempfile.mkdtemp(prefix='envx-social-'))

def run(user, *args, data=None, ok=True):
    env = {**os.environ, 'HOME': str(root / user), 'XDG_CONFIG_HOME': str(root / user / '.config')}
    env.pop('DEV_MODE', None)
    (root / user).mkdir(exist_ok=True)
    result = subprocess.run(command + list(args), input=data, text=True, capture_output=True, env=env, cwd=root / user)
    if ok and result.returncode:
        raise AssertionError(f'{user} {args[0]} failed: {result.stderr}')
    if not ok:
        assert result.returncode != 0, f'{args[0]} unexpectedly succeeded'
    return result

def doc(user, *args, **kwargs):
    return json.loads(run(user, *args, **kwargs).stdout)

def config(user):
    return root / user / '.config/envx/config.json'

users = {}
for user in ('alice', 'bob', 'charlie'):
    run(user, 'gen', '--username', user, '--passphrase', 'fixture-password-not-real', '--no-upload', '--json')
    value = json.loads(config(user).read_text())
    value['primary_key_password'] = 'fixture-password-not-real'
    value['sdk_url'] = server
    config(user).write_text(json.dumps(value))
    run(user, 'upload', '--username', user)
    users[user] = json.loads(config(user).read_text())['primary_key']['uuid']

link = doc('alice', 'friend-link', '--json')
assert link['code'].split(':')[0].count('-') == 1
label, marker, encoded = link['code'].split(':')
foreign = json.loads(bytes.fromhex(encoded))
foreign['server'] = 'http://127.0.0.1:9'
foreign_code = ':'.join((label, marker, json.dumps(foreign).encode().hex()))
assert 'another server' in run('bob', 'add-friend', foreign_code, ok=False).stderr
run('bob', 'add-friend', link['code'], '--alias', 'bad\nname', ok=False)
bob_pin = doc('bob', 'add-friend', link['code'], '--alias', 'alice', '--json')
assert bob_pin['user_id'] == users['alice']
run('charlie', 'add-friend', link['code'], '--json', ok=False)
# The original redeemer recovers the result without duplicating the friendship.
assert doc('bob', 'add-friend', link['code'], '--json')['user_id'] == users['alice']
friends = doc('alice', 'friends', '--json')
assert len(friends) == 1 and friends[0]['trust'] == 'trusted'
run('alice', 'friends', '--rename', users['bob'], '--alias', 'bob', '--json')
history = doc('alice', 'friend-link', '--list', '--json')
assert history[0]['redeemed_by']['id'] == users['bob']
assert 'token' not in history[0]

targeted = doc('alice', 'friend-link', users['charlie'], '--json')
run('bob', 'add-friend', targeted['code'], '--json', ok=False)
assert doc('charlie', 'add-friend', targeted['code'], '--json')['user_id'] == users['alice']
revoked = doc('alice', 'friend-link', '--json')
run('alice', 'friend-link', '--revoke', revoked['link']['id'])
run('charlie', 'add-friend', revoked['code'], '--json', ok=False)

# An unexpected pinned fingerprint blocks send before any plaintext is submitted.
db = sqlite3.connect(root / 'alice' / '.config/envx/state.sqlite')
row = db.execute("SELECT scope,value FROM records WHERE namespace='friend' AND key=?", (users['bob'],)).fetchone()
changed_pin = json.loads(row[1]); changed_pin['fingerprint'] = '0' * 40
db.execute("UPDATE records SET value=? WHERE scope=? AND namespace='friend' AND key=?", (json.dumps(changed_pin).encode(), row[0], users['bob'])); db.commit()
assert 'key changed' in run('alice', 'send', 'bob', '--stdin', data='never-send', ok=False).stderr.lower()
db.execute("UPDATE records SET value=? WHERE scope=? AND namespace='friend' AND key=?", (row[1], row[0], users['bob'])); db.commit(); db.close()

secret = 'test-only secret with spaces = not metadata\n'
sent = doc('alice', 'send', 'bob', '--stdin', '--expires', '24h', '--json', data=secret)
inbox = doc('bob', 'inbox', '--json')
assert any(item['id'] == sent['id'] for item in inbox)
assert all(item.get('ciphertext') is None for item in inbox)
assert secret.strip() not in json.dumps(inbox)
assert run('bob', 'read', sent['id']).stdout == secret
assert run('alice', 'read', sent['id']).stdout == secret
run('charlie', 'read', sent['id'], ok=False)
output = root / 'bob' / 'secret.txt'
run('bob', 'read', sent['id'], '--output', str(output))
assert output.read_text() == secret and output.stat().st_mode & 0o777 == 0o600
run('bob', 'read', sent['id'], '--output', str(output), ok=False)
run('bob', 'inbox', '--delete', sent['id'])
run('bob', 'read', sent['id'], ok=False)
assert run('alice', 'read', sent['id']).stdout == secret

project = doc('bob', 'project', 'new', '--name', 'message-import', '--json')
project_id = project['id'] if 'id' in project else project['project_id']
run('bob', 'set', '--project-id', project_id, '--yes', data='TOKEN=old-value\nKEEP=untouched\n')
variables = doc('alice', 'send', 'bob', '--env', '--stdin', '--json', data='TOKEN=new-value\nNEW=added\n')
preview = doc('bob', 'import', 'message', variables['id'], '--project-id', project_id, '--dry-run', '--json')
assert preview['names'] == ['NEW', 'TOKEN'] and preview['conflicts'] == ['TOKEN']
assert 'new-value' not in json.dumps(preview)
result = doc('bob', 'import', 'message', variables['id'], '--project-id', project_id, '--yes', '--json')
assert result['applied'] is True
values = doc('bob', 'variables', '--project-id', project_id, '--all', '--json')
assert values == {'TOKEN': 'new-value', 'KEEP': 'untouched', 'NEW': 'added'}
run('alice', 'friends', '--remove', 'bob', '--json')
run('alice', 'send', 'bob', '--stdin', data='blocked', ok=False)
assert doc('bob', 'read', variables['id'], '--json')['payload']['kind'] == 'variables'
print('PASS: 3 independent CLI profiles, single-use/target/revoke/recovery, trust, encrypted handoffs, metadata privacy, access denial, deletion, private output, atomic import, removal')
