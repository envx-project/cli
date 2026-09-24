"""Deterministically change config during an API response; Docker profiles only."""
import json
import os
from pathlib import Path
import sqlite3
import subprocess
import tempfile
import threading
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

assert Path('/.dockerenv').exists()
home = Path(tempfile.mkdtemp(prefix='envx-operation-context-'))
env = dict(os.environ, HOME=str(home), XDG_CONFIG_HOME=str(home / '.config'))
binary = ['/opt/envx-libs/ld-linux-x86-64.so.2', '--library-path', '/opt/envx-libs', '/opt/envx']

def run(*args, data=''):
    return subprocess.run(binary + list(args), input=data, text=True, capture_output=True, env=env, cwd=home)

result = run('gen', '--username', 'context-fixture', '--passphrase', 'fixture-password', '--no-upload', '--json')
assert result.returncode == 0, result.stderr
public_key = json.loads(result.stdout)['public_key']
path = home / '.config/envx/config.json'
config = json.loads(path.read_text())
config['primary_key_password'] = 'fixture-password'
account = 'aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa'
config['primary_key']['uuid'] = account
project = 'bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb'
requests = []
mutations = []
scenario = 'read'

def change_server():
    latest = json.loads(path.read_text())
    latest['sdk_url'] = other_origin
    path.write_text(json.dumps(latest))

class Handler(BaseHTTPRequestHandler):
    def log_message(self, *args): pass
    def respond(self, status, value):
        body = json.dumps(value).encode()
        self.send_response(status)
        self.send_header('Content-Type', 'application/json')
        self.send_header('Content-Length', str(len(body)))
        self.end_headers()
        self.wfile.write(body)
    def do_GET(self):
        requests.append((self.server.server_port, self.path))
        value = []
        if self.path == f'/project/{project}/variables' and scenario in ('read', 'fallback'):
            assert self.server is source
            change_server()
            if scenario == 'fallback':
                self.respond(503, {'error':'synthetic offline failure'})
                return
        if self.path == f'/v2/project/{project}':
            value = {'project_id': project, 'project_name': 'context', 'users': [{'id':account,'username':'fixture','created_at':'2026-01-01','public_key':public_key}]}
            if scenario == 'write':
                assert self.server is source
                change_server()
        self.respond(200, value)
    def do_POST(self):
        self.rfile.read(int(self.headers.get('Content-Length', '0')))
        mutations.append((self.server.server_port, self.path))
        self.respond(200, [{'id':'cccccccc-cccc-4ccc-8ccc-cccccccccccc'}])

source = ThreadingHTTPServer(('127.0.0.1', 0), Handler)
other = ThreadingHTTPServer(('127.0.0.1', 0), Handler)
source_origin = f'http://127.0.0.1:{source.server_port}'
other_origin = f'http://127.0.0.1:{other.server_port}'
for server in (source, other): threading.Thread(target=server.serve_forever, daemon=True).start()

def select_source():
    config['sdk_url'] = source_origin
    path.write_text(json.dumps(config))

select_source()
result = run('variables', '--project-id', project, '--json')
assert result.returncode == 0, result.stderr
with sqlite3.connect(home / '.config/envx/state.sqlite') as db:
    scopes = [row[0] for row in db.execute("SELECT scope FROM records WHERE namespace='cache' AND key=?", (project,))]
assert len(scopes) == 1 and scopes[0].startswith(source_origin + '|'), scopes
assert account in scopes[0], scopes
assert json.loads(path.read_text())['sdk_url'] == other_origin
other_local = run('variables', '--project-id', project, '--local', '--json')
assert other_local.returncode != 0 and not other_local.stdout
select_source()
source_local = run('variables', '--project-id', project, '--local', '--json')
assert source_local.returncode == 0 and json.loads(source_local.stdout) == {}, source_local.stderr

scenario = 'fallback'
result = run('variables', '--project-id', project, '--json')
assert result.returncode == 0 and json.loads(result.stdout) == {}, result.stderr
assert 'using cached variables' in result.stderr
assert json.loads(path.read_text())['sdk_url'] == other_origin

scenario = 'write'
select_source()
result = run('set', '--project-id', project, '--yes', '--json', data='FIXTURE=not-a-real-secret\n')
assert result.returncode == 0, result.stderr
assert mutations == [(source.server_port, '/variables/set-many')], mutations
assert all(port == source.server_port for port, _ in requests), requests
assert json.loads(path.read_text())['sdk_url'] == other_origin
print('PASS: cache/read/fallback/recipient lookup/mutation stay on captured origin and exact account; concurrent config changes remain preserved')
