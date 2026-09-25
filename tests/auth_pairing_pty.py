#!/usr/bin/env python3
"""Real two-terminal auth pairing against a disposable API; never use a real profile."""
import datetime, fcntl, json, os, pty, re, select, signal, sqlite3, struct, subprocess, tempfile, termios, time
import http.client, http.server, threading, urllib.parse
from pathlib import Path
assert Path('/.dockerenv').exists(), 'Run only inside Docker'
root=Path(tempfile.mkdtemp(prefix='envx-auth-pairing-'))
binary=os.environ.get('ENVX_TEST_BIN','/opt/envx')
server=os.environ.get('ENVX_TEST_API','http://127.0.0.1:55440')
password='synthetic-pairing-passphrase'
upstream=urllib.parse.urlparse(server)
wire=[]
class RelayRecorder(http.server.BaseHTTPRequestHandler):
    def do_POST(self):
        body=self.rfile.read(int(self.headers.get('Content-Length',0)))
        conn=http.client.HTTPConnection(upstream.hostname,upstream.port,timeout=20)
        conn.request('POST',self.path,body,dict(self.headers))
        response=conn.getresponse();payload=response.read();conn.close()
        if '/auth/pairing/' in self.path: wire.append((self.path,body,payload))
        self.send_response(response.status);self.send_header('Content-Type','application/json');self.end_headers();self.wfile.write(payload)
    def do_GET(self):
        conn=http.client.HTTPConnection(upstream.hostname,upstream.port,timeout=20)
        conn.request('GET',self.path,headers=dict(self.headers));response=conn.getresponse();payload=response.read();conn.close()
        self.send_response(response.status);self.send_header('Content-Type','application/json');self.end_headers();self.wfile.write(payload)
    def log_message(self,*args): pass
proxy=http.server.ThreadingHTTPServer(('127.0.0.1',0),RelayRecorder)
threading.Thread(target=proxy.serve_forever,daemon=True).start()
server=f'http://127.0.0.1:{proxy.server_port}'

def env(user): return {**os.environ,'HOME':str(root/user),'USERPROFILE':str(root/user),'TERM':'xterm-256color'}
def run(user,*args,data=None,ok=True):
    result=subprocess.run([binary,*args],env=env(user),cwd=root/user,input=data,text=True,capture_output=True)
    if ok: assert result.returncode==0,(args[0],result.stderr)
    else: assert result.returncode!=0
    return result.stdout

def config(user): return root/user/'.config/envx/config.json'
def setup(user):
    (root/user).mkdir()
    run(user,'config','get','sdk_url')
    path=config(user); value=json.loads(path.read_text());value['sdk_url']=server;path.write_text(json.dumps(value))
    run(user,'config','get','sdk_url')
    run(user,'whoami',ok=False)
    db=sqlite3.connect(root/user/'.config/envx/state.sqlite')
    state=json.dumps({'last_update_check':datetime.datetime.now(datetime.timezone.utc).isoformat(),'latest_version':None}).encode()
    db.execute("INSERT OR REPLACE INTO records VALUES('global','updates','latest',?)",(state,));db.commit();db.close()
class Terminal:
    def __init__(self,user,*args):
        self.pid,self.fd=pty.fork()
        if self.pid==0:
            os.chdir(root/user);os.execve(binary,[binary,*args],env(user))
        fcntl.ioctl(self.fd,termios.TIOCSWINSZ,struct.pack('HHHH',40,180,0,0))
        self.output=b'';self.status=None
    def pump(self):
        if self.status is not None:return
        ready,_,_=select.select([self.fd],[],[],0.1)
        if ready:
            try: chunk=os.read(self.fd,65536)
            except OSError: chunk=b''
            self.output+=chunk
            if b'\x1b[6n' in chunk: os.write(self.fd,b'\x1b[1;1R')
        pid,status=os.waitpid(self.pid,os.WNOHANG)
        if pid:self.status=status
    def until(self,text,timeout=35):
        deadline=time.monotonic()+timeout
        while text.encode() not in self.output:
            self.pump()
            assert self.status is None,('exited',text,self.output.decode(errors='replace'))
            assert time.monotonic()<deadline,('timeout',text,self.output.decode(errors='replace'))
    def send(self,text):os.write(self.fd,text.encode()+b'\r')
    def finish(self):
        deadline=time.monotonic()+25
        while self.status is None:
            self.pump()
            if time.monotonic()>deadline:
                os.kill(self.pid,signal.SIGKILL);raise AssertionError(('hung terminal',self.output.decode(errors='replace')))
        os.close(self.fd)
        return os.waitstatus_to_exitcode(self.status)

def pair(target):
    source=Terminal('source','auth','link');source.until('Keep this terminal open')
    link=re.search(rb"envx auth login '([^']+)'",source.output).group(1).decode()
    assert re.fullmatch(re.escape(server)+r'/#envx-pair-v1=[a-f0-9-]{36}',link)
    receiver=Terminal(target,'auth','login',link);receiver.until('Waiting for that machine to approve')
    source.until("Receiving terminal's verification code")
    code=re.search(rb'Verification code:\r?\n([a-f0-9]{64})',receiver.output).group(1).decode()
    return source,receiver,code

for user in ('source','receiver','rejected','wrongpass','cancelled','cancelpass'):setup(user)
run('source','auth','gen','--username','pairing-test','--passphrase',password,'--json')
v=json.loads(config('source').read_text());v['primary_key_password']=password;config('source').write_text(json.dumps(v))
project=json.loads(run('source','project','new','--name','pairing-secret','--no-link','--json'))['project_id']
run('source','set','--project-id',project,'--yes',data='PAIRING_TEST=synthetic-project-secret\n')
source,receiver,code=pair('receiver')
source.send(code);assert source.finish()==0
receiver.until('Existing identity passphrase');receiver.send(password);assert receiver.finish()==0,receiver.output.decode(errors='replace')
assert password.encode() not in receiver.output
assert json.loads(config('source').read_text())['primary_key']['uuid']==json.loads(config('receiver').read_text())['primary_key']['uuid']
v=json.loads(config('receiver').read_text());assert v.get('primary_key_password') is None
v['primary_key_password']=password;config('receiver').write_text(json.dumps(v))
assert json.loads(run('receiver','variables','--project-id',project,'--all','--json'))=={'PAIRING_TEST':'synthetic-project-secret'}
run('receiver','auth','status')
source,receiver,code=pair('rejected');source.send('0'*64)
assert source.finish()!=0
assert receiver.finish()!=0
assert json.loads(config('rejected').read_text()).get('primary_key') is None
assert not list((root/'rejected/.config/envx/keys').glob('*/private.key'))
source,receiver,code=pair('wrongpass');source.send(code);assert source.finish()==0
receiver.until('Existing identity passphrase');receiver.send('wrong-passphrase')
assert receiver.finish()!=0
assert json.loads(config('wrongpass').read_text()).get('primary_key') is None
assert not list((root/'wrongpass/.config/envx/keys').glob('*/private.key'))
existing=Terminal('receiver','auth','login',server+'/#envx-pair-v1=00000000-0000-4000-8000-000000000000')
assert existing.finish()!=0;assert b'will not replace' in existing.output
source,receiver,code=pair('cancelled');os.write(source.fd,b'\x03')
assert source.finish()!=0;assert receiver.finish()!=0
assert json.loads(config('cancelled').read_text()).get('primary_key') is None
source,receiver,code=pair('cancelpass');source.send(code);assert source.finish()==0
receiver.until('Existing identity passphrase');os.write(receiver.fd,b'\x03');assert receiver.finish()!=0
assert json.loads(config('cancelpass').read_text()).get('primary_key') is None
for path,request,response in wire:
    for content in (request,response):
        assert password.encode() not in content
        assert b'PRIVATE KEY' not in content
        assert b'secret_key' not in content
        assert b'fingerprint' not in content
        try: data=json.loads(content)
        except json.JSONDecodeError: continue
        if isinstance(data,dict) and data.get('message'):
            raw=bytes.fromhex(data['message'])
            assert b'PRIVATE KEY' not in raw and password.encode() not in raw
print('PASS: real pairing; invitation contains only session ID; matching identity authenticates and decrypts existing project; passphrase not persisted in config or echoed; wrong verification/passphrase install nothing; existing identity preserved; cancellation restores control and installs nothing; wire contains no identity/passphrase plaintext')
