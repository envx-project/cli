"""Run the real parallel Cargo test command with a sentinel user profile in Docker."""
import hashlib
import json
import os
from pathlib import Path
import subprocess
import tempfile


def snapshot(root):
    return {
        str(path.relative_to(root)): (
            path.stat().st_mode,
            hashlib.sha256(path.read_bytes()).hexdigest() if path.is_file() else None,
        )
        for path in sorted(root.rglob('*'))
    }


with tempfile.TemporaryDirectory(prefix='envx-sentinel-home-') as temporary:
    home = Path(temporary)
    profile = home / '.config/envx'
    (profile / 'keys/sentinel').mkdir(parents=True)
    (profile / 'config.json').write_text(json.dumps({
        'salt': 'sentinel', 'primary_key': None,
        'sdk_url': 'https://sentinel.example', 'settings': None,
        'projects': [], 'primary_key_password': 'sentinel-only-password',
        'primary_key_command': None,
    }))
    (profile / 'keys/sentinel/private.key').write_bytes(b'sentinel private key: never read or replace')
    (profile / 'keys/sentinel/public.key').write_bytes(b'sentinel public key: never read or replace')
    (profile / 'state.sqlite').write_bytes(b'sentinel database: not a test fixture')
    (home / '.profile').write_bytes(b'sentinel shell profile')
    before = snapshot(home)
    environment = dict(os.environ, HOME=str(home), USERPROFILE=str(home))
    # Even an inherited override is ignored except by the explicitly invoked
    # subprocess concurrency worker. Ordinary tests own their temporary homes.
    environment['ENVX_TEST_HOME'] = str(home)
    command = ['cargo', 'test', '--locked', '--offline', '--', '--test-threads=8']
    result = subprocess.run(command, env=environment)
    assert snapshot(home) == before, 'cargo test changed the sentinel HOME'
    assert result.returncode == 0, f'parallel cargo test failed ({result.returncode})'
    print('PASS actual parallel cargo test: sentinel HOME/USERPROFILE unchanged, no skips', flush=True)
