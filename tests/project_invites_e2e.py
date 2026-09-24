#!/usr/bin/env python3
"""Run only in Docker, using synthetic keys and a disposable API database."""
import json
import os
from pathlib import Path
import shlex
import subprocess
import tempfile

assert Path('/.dockerenv').exists(), 'This test must run inside Docker'
command = shlex.split(os.environ.get('ENVX_TEST_COMMAND', '/opt/envx-libs/ld-linux-x86-64.so.2 --library-path /opt/envx-libs /opt/envx'))
server = os.environ.get('ENVX_TEST_API', 'http://127.0.0.1:3005')
root = Path(tempfile.mkdtemp(prefix='envx-project-invites-'))

def run(user, *args, data=None, ok=True):
    home = root / user
    home.mkdir(exist_ok=True)
    env = {**os.environ, 'HOME': str(home), 'XDG_CONFIG_HOME': str(home / '.config')}
    env.pop('DEV_MODE', None)
    result = subprocess.run(command + list(args), input=data, text=True, capture_output=True, env=env, cwd=home)
    if ok and result.returncode:
        raise AssertionError(f'{user} {args[0]} failed: {result.stderr}')
    if not ok:
        assert result.returncode != 0, f'{args[0]} unexpectedly succeeded'
    return result

def doc(user, *args, **kwargs):
    return json.loads(run(user, *args, **kwargs).stdout)

def project(user, name):
    return doc(user, 'project', 'new', '--name', name, '--no-link', '--json')['project_id']

def variables(user, project_id):
    return doc(user, 'variables', '--project-id', project_id, '--all', '--json')

def invite(user, project_id):
    return doc(user, 'invite', 'create', '--project-id', project_id, '--json')['code']

users = {}
for user in ('alice', 'bob', 'charlie'):
    run(user, 'gen', '--username', user, '--passphrase', 'fixture-password-not-real', '--no-upload', '--json')
    config = root / user / '.config/envx/config.json'
    value = json.loads(config.read_text())
    value['primary_key_password'] = 'fixture-password-not-real'
    value['sdk_url'] = server
    config.write_text(json.dumps(value))
    run(user, 'upload', '--username', user)
    users[user] = json.loads(config.read_text())['primary_key']['uuid']

shared = project('alice', 'snapshot-invite')
run('alice', 'set', '--project-id', shared, '--yes', data='TOKEN=old-value\nKEEP=untouched\n')
stale = invite('alice', shared)
# Atomic set replaces TOKEN with a new row ID and appends NEW.
run('alice', 'set', '--project-id', shared, '--yes', data='TOKEN=new-value\nNEW=added\n')
assert 'Regenerate' in run('bob', 'invite', 'accept', stale, ok=False).stderr
run('bob', 'variables', '--project-id', shared, '--all', '--json', ok=False)
assert variables('alice', shared) == {'TOKEN': 'new-value', 'KEEP': 'untouched', 'NEW': 'added'}
fresh = invite('alice', shared)
result = run('bob', 'invite', 'accept', fresh)
assert fresh.split(':')[0] not in result.stdout
assert 'Successfully joined' in result.stdout
expected = {'TOKEN': 'new-value', 'KEEP': 'untouched', 'NEW': 'added'}
assert variables('alice', shared) == expected
assert variables('bob', shared) == expected
run('charlie', 'invite', 'accept', fresh, ok=False)
run('charlie', 'variables', '--project-id', shared, '--all', '--json', ok=False)
# Direct add uses the same atomic rewrap boundary and preserves every member.
added = doc('alice', 'project', 'add-users', '--project-id', shared, '--json', users['charlie'])
assert added['added_user_ids'] == [users['charlie']]
for user in users:
    assert variables(user, shared) == expected
empty = project('alice', 'empty-invite')
run('bob', 'invite', 'accept', invite('alice', empty))
assert variables('bob', empty) == {}
print('PASS: isolated synthetic profiles; stale changed/replaced/added rows rejected; fresh invite preserves all values for both members; replay denied; atomic add-users decrypts for all three; empty project joins; invitation secrets absent from accept output')
