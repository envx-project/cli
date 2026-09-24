import os, pathlib, tempfile, json, subprocess, sqlite3, shlex
assert pathlib.Path('/.dockerenv').exists()
home=pathlib.Path(tempfile.mkdtemp()); directory=home/'.config/envx';directory.mkdir(parents=True)
path=directory/'config.json';project={'project_id':'bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb','path':'/work/project'}
config={'salt':'preserve','primary_key':None,'sdk_url':'https://api.envx.sh?bad=query','settings':None,'projects':[project],'primary_key_password':'preserve-password','primary_key_command':None,'custom':'preserve-extra'}
original=json.dumps(config);path.write_text(original)
binary=shlex.split(os.environ.get("ENVX_COMMAND", "/artifacts/envx"))
def run(*args):return subprocess.run(binary+list(args),capture_output=True,text=True,env=dict(os.environ,HOME=str(home)))
p=run('get','config','--json');assert p.returncode!=0,p.stdout
assert path.read_text()==original
p=run('config','get','sdk_url');assert p.returncode==0,p.stderr
p=run('config','set','sdk_url','https://api.envx.sh');assert p.returncode==0,p.stderr
current=json.loads(path.read_text());assert current['sdk_url']=='https://api.envx.sh'
for field in ['salt','projects','primary_key_password','custom']:assert current[field]==config[field]
assert (directory/'config.pre-sqlite.json').read_text()==original
conn=sqlite3.connect(directory/'state.sqlite');assert conn.execute("SELECT COUNT(*) FROM records WHERE namespace='projects'").fetchone()[0]==0
assert 'envx link' in p.stderr
p=run('version');assert p.returncode==0,p.stderr
assert conn.execute("SELECT COUNT(*) FROM records WHERE namespace='projects'").fetchone()[0]==0
print('PASS invalid URL repair preserves unknown/private fields and original links in JSON+backup without assigning unknown-origin links to the repaired server')
