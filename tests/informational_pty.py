"""Run in Docker with --network none; ENVX_COMMAND selects the built CLI."""
import os,pty,subprocess,pathlib,sqlite3,tempfile,hashlib,shlex
binary=shlex.split(os.environ.get("ENVX_COMMAND", "/artifacts/envx"))
def snapshot(root):
 return {str(p.relative_to(root)):(hashlib.sha256(p.read_bytes()).hexdigest(),p.stat().st_mtime_ns) for p in root.rglob('*') if p.is_file()}
for case in ['malformed','future','readonly']:
 home=pathlib.Path(tempfile.mkdtemp()); profile=home/'.config/envx';profile.mkdir(parents=True)
 if case=='malformed':(profile/'config.json').write_text('{bad json')
 if case=='future':
  (profile/'config.json').write_text('{"salt":"x","primary_key":null,"sdk_url":null,"settings":null,"projects":[],"primary_key_password":null,"primary_key_command":null}')
  con=sqlite3.connect(profile/'state.sqlite');con.execute('PRAGMA user_version=999');con.close()
 before=snapshot(home)
 for args,expected in [(['--help'],0),(['--version'],0),(['--definitely-invalid'],2),(['project','--help'],0)]:
  master,slave=pty.openpty();env=dict(os.environ,HOME='/proc/envx-readonly' if case=='readonly' else str(home))
  proc=subprocess.Popen(binary+args,stdin=subprocess.DEVNULL,stdout=slave,stderr=slave,env=env);os.close(slave)
  assert proc.wait(timeout=2)==expected,(case,args,proc.returncode)
  os.close(master)
 assert snapshot(home)==before,case
print('PASS: PTY help/version/subcommandhelp/usage exit before malformed config, future DB, read-only HOME; no profile writes, network disabled')

# Exercise the public completion command, not just its parser construction.
home=pathlib.Path(tempfile.mkdtemp())
for shell in ['bash', 'zsh']:
    output=subprocess.check_output(binary+['completion', shell], env=dict(os.environ, HOME=str(home)), text=True)
    for command in ['friend-link', 'add-friend', 'inbox', 'project', 'variables']:
        assert command in output, (shell, command)
print('PASS: actual bash/zsh completion commands contain root subcommands')
