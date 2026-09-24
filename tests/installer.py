"""Offline installer regression tests. Run inside Docker; never installs on the host."""
import hashlib
import io
import os
from pathlib import Path
import subprocess
import tarfile
import tempfile
import unittest

SCRIPT = Path(__file__).resolve().parents[1] / 'install.sh'


class InstallerTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name)
        self.bin = self.root / 'bin with spaces'
        self.bin.mkdir()
        self.mocks = self.root / 'mocks'
        self.mocks.mkdir()
        self.scratch = self.root / 'temp files'
        self.scratch.mkdir()
        self.archive = self.root / 'fixture.tar.gz'
        with tarfile.open(self.archive, 'w:gz') as archive:
            contents = b'#!/bin/sh\nprintf "fixture envx\\n"\n'
            entry = tarfile.TarInfo('envx')
            entry.size = len(contents)
            entry.mode = 0o755
            archive.addfile(entry, io.BytesIO(contents))
            # Extra archive entries must not reach the install directory.
            entry = tarfile.TarInfo('unwanted-file')
            entry.size = 4
            archive.addfile(entry, io.BytesIO(b'nope'))
        self.digest = hashlib.sha256(self.archive.read_bytes()).hexdigest()
        self.log = self.root / 'requests'
        curl = self.mocks / 'curl'
        curl.write_text('''#!/usr/local/bin/python3
import os,sys,pathlib
args=sys.argv[1:]
for option in ['--proto','--proto-redir']:
    assert args[args.index(option)+1] == '=https'
assert '--tlsv1.2' in args
url=args[-1]
assert url.startswith('https://')
with open(os.environ['REQUEST_LOG'],'a') as log: log.write(url+'\\n')
if '--write-out' in args:
    print('https://github.com/envx-project/cli/releases/tag/v2.14.0',end='')
    sys.exit(0)
destination=pathlib.Path(args[args.index('--output')+1])
if url.endswith('.sha256'):
    if os.environ.get('MISSING_CHECKSUM'): sys.exit(22)
    destination.write_text(os.environ['DIGEST']+'  archive.tar.gz\\n')
else:
    destination.write_bytes(pathlib.Path(os.environ['ARCHIVE']).read_bytes())
''')
        curl.chmod(0o755)
        self.env = os.environ | {
            'PATH': f'{self.mocks}:/usr/local/bin:/usr/bin:/bin',
            'TMPDIR': str(self.scratch),
            'ENVX_BIN_DIR': str(self.bin),
            'ENVX_ARCH': 'x86_64',
            'ENVX_PLATFORM': 'unknown-linux-musl',
            'ENVX_VERSION': '2.14.0',
            'REQUEST_LOG': str(self.log),
            'ARCHIVE': str(self.archive),
            'DIGEST': self.digest,
        }

    def tearDown(self):
        self.temp.cleanup()

    def run_installer(self, *args, success=True):
        result = subprocess.run(['sh', str(SCRIPT), *args], env=self.env,
                                text=True, capture_output=True)
        if success:
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        else:
            self.assertNotEqual(result.returncode, 0)
        self.assertEqual(list(self.scratch.iterdir()), [], 'temporary files leaked')
        return result

    def test_verified_install_respects_env_and_spaces(self):
        original = self.bin / 'test.txt'
        original.write_text('keep me')
        self.run_installer('--yes')
        result = subprocess.check_output([str(self.bin / 'envx')], text=True)
        self.assertEqual(result, 'fixture envx\n')
        self.assertEqual(original.read_text(), 'keep me')
        self.assertFalse((self.bin / 'unwanted-file').exists())
        self.assertIn('https://github.com/envx-project/cli/releases/download/v2.14.0/', self.log.read_text())

    def test_help_and_remove_do_not_use_network(self):
        self.run_installer('--help')
        self.assertFalse(self.log.exists())
        binary = self.bin / 'envx'
        binary.write_text('#!/bin/sh\n')
        binary.chmod(0o755)
        self.env['PATH'] = str(self.bin) + ':' + self.env['PATH']
        rm = self.mocks / 'rm'
        rm.write_text('#!/bin/sh\nfor arg do [ \"$arg\" != /tmp/envx ] || exit 99; done\nexec /bin/rm \"$@\"\n')
        rm.chmod(0o755)
        self.run_installer('--remove', '--yes')
        self.assertFalse(binary.exists())
        self.assertFalse(self.log.exists())

    def test_latest_uses_published_release_redirect(self):
        del self.env['ENVX_VERSION']
        self.run_installer('--yes')
        self.assertEqual(self.log.read_text().splitlines()[0], 'https://github.com/envx-project/cli/releases/latest')
        self.assertIn('/download/v2.14.0/', self.log.read_text())

    def test_bad_checksum_preserves_existing_binary(self):
        binary = self.bin / 'envx'
        binary.write_text('keep executable')
        self.env['DIGEST'] = '0' * 64
        result = self.run_installer('--yes', success=False)
        self.assertIn('checksum mismatch', result.stderr)
        self.assertEqual(binary.read_text(), 'keep executable')

    def test_missing_checksum_is_rejected_for_new_release(self):
        self.env['MISSING_CHECKSUM'] = '1'
        self.run_installer('--yes', success=False)
        self.assertFalse((self.bin / 'envx').exists())

    def test_legacy_release_missing_checksum_warns(self):
        self.env['MISSING_CHECKSUM'] = '1'
        self.env['ENVX_VERSION'] = '2.13.0'
        result = self.run_installer('--yes')
        self.assertIn('relying on HTTPS', result.stderr)

    def test_insecure_mirror_rejected_before_request(self):
        self.run_installer('--base-url', 'http://example.invalid/releases', success=False)
        self.assertFalse(self.log.exists())

    def test_unsupported_target_rejected_before_request(self):
        self.run_installer('--arch', 'aarch64', '--platform', 'unknown-linux-musl', success=False)
        self.assertFalse(self.log.exists())

    def test_missing_option_value_is_clear(self):
        result = self.run_installer('--bin-dir', success=False)
        self.assertIn('Missing value', result.stderr)
        self.assertFalse(self.log.exists())


if __name__ == '__main__':
    unittest.main()
