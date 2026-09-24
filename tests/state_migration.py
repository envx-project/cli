"""Run inside tests/state-migration-docker.sh, never against a real profile."""
import concurrent.futures
import json
import os
from pathlib import Path
import signal
import sqlite3
import subprocess
import tempfile

BINARY = '/artifacts/envx'
FIXTURES = Path('/fixtures')


def invoke(home, *args, ok=True, cwd=None):
    env = dict(os.environ, HOME=str(home))
    env.pop('DEV_MODE', None)
    result = subprocess.run([BINARY, *args], env=env, cwd=cwd, capture_output=True, text=True)
    assert (result.returncode == 0) == ok, (args, result.stdout, result.stderr)
    return result


def profile(fixture=None):
    home = Path(tempfile.mkdtemp())
    directory = home / '.config/envx'
    directory.mkdir(parents=True)
    if fixture:
        (directory / 'config.json').write_bytes(fixture.read_bytes())
    return home, directory


for fixture in sorted(FIXTURES.glob('*.json')):
    home, directory = profile(fixture)
    original = fixture.read_bytes()
    (directory / 'version.json').write_text('{"latest_version":"99.0.0"}')
    (directory / 'cache').mkdir()
    (directory / 'cache/old.envx').write_bytes(b'old encrypted cache')
    with concurrent.futures.ThreadPoolExecutor(max_workers=12) as pool:
        list(pool.map(lambda _: invoke(home, 'version'), range(24)))
    db = sqlite3.connect(directory / 'state.sqlite')
    assert db.execute('pragma integrity_check').fetchone()[0] == 'ok'
    assert db.execute('select count(*) from records where namespace="projects"').fetchone()[0] == 1
    assert db.execute('select count(*) from migrations').fetchone()[0] == 1
    assert (directory / 'config.json').read_bytes() == original
    assert (directory / 'config.pre-sqlite.json').read_bytes() == original
    assert (directory / 'cache/old.envx').read_bytes() == b'old encrypted cache'
    assert (directory / 'state.sqlite').stat().st_mode & 0o777 == 0o600
    # Unlink persists only in SQLite; repeated starts never resurrect old JSON.
    Path('/work/project').mkdir(parents=True, exist_ok=True)
    invoke(home, 'unlink', cwd='/work/project')
    invoke(home, 'version')
    assert db.execute('select count(*) from records where namespace="projects"').fetchone()[0] == 0
    assert (directory / 'config.json').read_bytes() == original
    invoke(home, 'config', 'set', 'settings.loud', 'true')
    assert json.loads((directory / 'config.json').read_text())['unknown_future_setting'] == {'preserve': True}
    # Future schema fails closed, without touching settings.
    before = (directory / 'config.json').read_bytes()
    db.execute('pragma user_version=999')
    invoke(home, 'version', ok=False)
    assert (directory / 'config.json').read_bytes() == before
    print('PASS migration, concurrent startup, unlink, settings preservation, future guard:', fixture.name)

# Concurrent empty-profile startup cannot expose a partially written config.
home, directory = profile()
with concurrent.futures.ThreadPoolExecutor(max_workers=12) as pool:
    list(pool.map(lambda _: invoke(home, 'version'), range(24)))
json.loads((directory / 'config.json').read_text())
print('PASS concurrent fresh startup')

# Interrupted transaction rolls back when the next client opens the database.
dbpath = directory / 'state.sqlite'
proc = subprocess.Popen(['python3', '-c', '''import sqlite3,sys,time
c=sqlite3.connect(sys.argv[1]); c.execute("BEGIN IMMEDIATE")
c.execute("INSERT INTO records VALUES('test','test','partial',x'00')")
print('ready',flush=True); time.sleep(60)
''', str(dbpath)], stdout=subprocess.PIPE, text=True)
assert proc.stdout.readline().strip() == 'ready'
proc.kill(); proc.wait()
invoke(home, 'version')
db = sqlite3.connect(dbpath)
assert db.execute('select count(*) from records where key="partial"').fetchone()[0] == 0
assert db.execute('pragma integrity_check').fetchone()[0] == 'ok'
print('PASS killed writer recovery')

# Malformed config is never rewritten, even with ENVX_DEBUG.
home, directory = profile()
(directory / 'config.json').write_text('{"primary_key_password":"fixture-secret",')
result = invoke(home, 'version', ok=False)
assert 'fixture-secret' not in result.stderr + result.stdout
assert (directory / 'config.json').read_text().endswith(',')
print('PASS damaged config preserved')

# Legacy envcli migration updates in-memory config and preserves original key tree.
home, directory = profile()
legacy = home / '.config/envcli'
(legacy / 'keys/abc123').mkdir(parents=True)
(legacy / 'config.json').write_bytes((FIXTURES / 'v2.0.json').read_bytes())
(legacy / 'keys/abc123/private.key').write_text('fixture-key')
invoke(home, 'config', 'migrate')
invoke(home, 'config', 'migrate')
assert (legacy / 'keys/abc123/private.key').read_text() == 'fixture-key'
assert (directory / 'keys/abc123/private.key').read_text() == 'fixture-key'
db = sqlite3.connect(directory / 'state.sqlite')
assert db.execute('select count(*) from records where namespace="projects"').fetchone()[0] == 1
assert json.loads((directory / 'config.json').read_text())['primary_key'] == 'abc123'
print('PASS legacy envcli import and retry')
