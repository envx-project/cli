"""First-command settings changes must not rewrite legacy link provenance."""
import json
import os
import pty
from pathlib import Path
import shlex
import sqlite3
import subprocess
import tempfile

assert Path('/.dockerenv').exists(), 'run profile tests only inside Docker'
binary = shlex.split(os.environ.get('ENVX_COMMAND', '/artifacts/envx'))
project = {'project_id': 'bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb', 'path': '/work/project'}
identity = {'fingerprint': 'abc123', 'uuid': 'aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa', 'note': 'fixture', 'primary_user_id': 'fixture'}


def check(old_url, command, expected_origin):
    with tempfile.TemporaryDirectory() as temporary:
        home = Path(temporary)
        directory = home / '.config/envx'
        directory.mkdir(parents=True)
        path = directory / 'config.json'
        config = {'salt': 'preserve', 'primary_key': identity, 'sdk_url': old_url,
                  'settings': None, 'projects': [project], 'primary_key_password': 'fixture',
                  'primary_key_command': None, 'unknown': 'preserve'}
        original = json.dumps(config).encode()
        path.write_bytes(original)
        env = dict(os.environ, HOME=str(home))
        if command == ['config', 'edit']:
            editor = home / 'edit-fixture'
            editor.write_text("#!/usr/bin/python3\nimport json,sys,pathlib\np=pathlib.Path(sys.argv[1]);c=json.loads(p.read_text());c['sdk_url']='https://server-b.example';p.write_text(json.dumps(c))\n")
            editor.chmod(0o700)
            master, slave = pty.openpty()
            result = subprocess.run(binary + command, env=dict(env, EDITOR=str(editor), VISUAL=str(editor), TERM='xterm'), stdin=slave, stdout=slave, stderr=slave, timeout=10)
            os.close(slave)
            result.stderr = os.read(master, 65536).decode()
            os.close(master)
        else:
            result = subprocess.run(binary + command, env=env, text=True, capture_output=True)
        assert result.returncode == 0, result.stderr
        assert (directory / 'config.pre-sqlite.json').read_bytes() == original
        current = json.loads(path.read_bytes())
        assert current['projects'] == [project] and current['unknown'] == 'preserve'
        db = sqlite3.connect(directory / 'state.sqlite')
        rows = db.execute("SELECT scope,value FROM records WHERE namespace='projects'").fetchall()
        if expected_origin is None:
            assert rows == [], f'unknown-origin links claimed for a server: {rows}'
            assert 'envx link' in result.stderr, result.stderr
        else:
            assert len(rows) == 1
            expected = expected_origin + '|' + identity['uuid'] + '|' + identity['fingerprint']
            assert rows[0][0] == expected, (rows[0][0], expected)
            assert json.loads(rows[0][1]) == project
        # A later valid command must not resurrect or reattribute old JSON links.
        for _ in range(3):
            result = subprocess.run(binary + ['version'], env=env, text=True, capture_output=True)
            assert result.returncode == 0, result.stderr
        assert db.execute("SELECT scope,value FROM records WHERE namespace='projects'").fetchall() == rows
        assert db.execute("SELECT count(*) FROM migrations WHERE name='legacy-json-v1'").fetchone()[0] == 1
        print('PASS first-command legacy scope:', old_url, '->', command[-1], flush=True)


check('https://server-a.example', ['config', 'set', 'sdk_url', 'https://server-b.example'], 'https://server-a.example')
check('http://localhost:3000', ['config', 'unset', 'sdk_url'], 'http://localhost:3000')
check('https://server-a.example?invalid=query', ['config', 'set', 'sdk_url', 'https://server-b.example'], None)
check('not a URL', ['config', 'unset', 'sdk_url'], None)

check('https://server-a.example', ['config', 'edit'], 'https://server-a.example')
check('not a URL', ['config', 'edit'], None)
