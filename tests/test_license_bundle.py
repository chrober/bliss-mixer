import io
import hashlib
import json
from pathlib import Path
import subprocess
import sys
import tarfile
import tempfile
import unittest
import zipfile


SCRIPT = Path(__file__).resolve().parents[1] / 'scripts/build_license_bundle.py'


def add_file(tar, name, data):
    info = tarfile.TarInfo(name)
    info.size = len(data)
    tar.addfile(info, io.BytesIO(data))


class LicenseBundleTests(unittest.TestCase):
    def test_fuchsia_sys_supplement_includes_related_parent_crate_license(self):
        path = SCRIPT.parents[1] / 'third-party-licenses/fuchsia-zircon-sys-0.3.3-LICENSE'
        text = path.read_text(encoding='utf-8')
        self.assertIn('fuchsia-zircon 0.3.3', text)
        start = text.index('// Copyright 2016 The Fuchsia Authors.')
        end = text.index('\n\nThe following license', start)
        upstream = text[start:end] + '\n'
        self.assertEqual(hashlib.sha256(upstream.encode('utf-8')).hexdigest(),
                         'f82f9062a6dff28145c185f90f94c485ee bdc2bb4c6f1c840513709e6d228453'.replace(' ', ''))

    def test_actix_macros_upstream_mit_supplement_is_present(self):
        path = SCRIPT.parents[1] / 'third-party-licenses/actix-macros-0.1.3-LICENSE-MIT'
        self.assertTrue(path.is_file(), 'actix-macros upstream notice is missing')
        text = path.read_text(encoding='utf-8')
        self.assertIn('ab496a71b576b29a9fd07064c11eda55428fb552', text)
        upstream = text[text.index('Copyright (c) 2017'):]
        self.assertEqual(hashlib.sha256(upstream.encode('utf-8')).hexdigest(),
                         '48c3e116e98d2507222ddf53c72f2d4cd6dae3798ca665446e0014540b6e7d27')

    def test_copies_inventory_vendor_notices_and_supplements(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source, output = root / 'source.tar.gz', root / 'notices.zip'
            with tarfile.open(source, 'w:gz') as tar:
                add_file(tar, 'source/SOURCE_MANIFEST.json', json.dumps({'schema_version': 1, 'source_commit': 'a' * 40, 'packages': []}).encode())
                add_file(tar, 'source/vendor/example-1/LICENSE-MIT', b'license text')
                add_file(tar, 'source/vendor/example-1/nested/NOTICE', b'notice text')
                add_file(tar, 'source/vendor/example-1/AUTHORS', b'copyright and license')
                add_file(tar, 'source/vendor/example-1/src/lib.rs', b'not a notice')
                add_file(tar, 'source/third-party-licenses/missing-LICENSE', b'supplement')
            run = subprocess.run([sys.executable, str(SCRIPT), str(source), str(output)], capture_output=True, text=True)
            self.assertEqual(run.returncode, 0, run.stderr)
            with zipfile.ZipFile(output) as bundle:
                self.assertEqual(bundle.read('vendor/example-1/LICENSE-MIT'), b'license text')
                self.assertEqual(bundle.read('vendor/example-1/nested/NOTICE'), b'notice text')
                self.assertEqual(bundle.read('vendor/example-1/AUTHORS'), b'copyright and license')
                self.assertEqual(bundle.read('third-party-licenses/missing-LICENSE'), b'supplement')
                self.assertIn('SOURCE_MANIFEST.json', bundle.namelist())
                self.assertNotIn('vendor/example-1/src/lib.rs', bundle.namelist())
            self.assertNotEqual(subprocess.run([sys.executable, str(SCRIPT), str(source), str(output)], capture_output=True).returncode, 0)

    def test_rejects_unsafe_members(self):
        for unsafe in ('source/vendor/a/../../LICENSE', 'source/vendor/a/..\\LICENSE'):
            with self.subTest(unsafe=unsafe), tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                source, output = root / 'source.tar.gz', root / 'notices.zip'
                with tarfile.open(source, 'w:gz') as tar:
                    add_file(tar, 'source/SOURCE_MANIFEST.json', b'{"schema_version":1,"packages":[]}')
                    add_file(tar, unsafe, b'bad')
                self.assertNotEqual(subprocess.run([sys.executable, str(SCRIPT), str(source), str(output)], capture_output=True).returncode, 0)
                self.assertFalse(output.exists())

    def test_includes_manifest_declared_license_with_nonstandard_name(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source, output = root / 'source.tar.gz', root / 'notices.zip'
            declared = 'vendor/example-1/legal/terms.txt'
            manifest = {'schema_version': 1, 'packages': [{'notice_files': [declared]}]}
            with tarfile.open(source, 'w:gz') as tar:
                add_file(tar, 'source/SOURCE_MANIFEST.json', json.dumps(manifest).encode())
                add_file(tar, 'source/' + declared, b'license terms')
            run = subprocess.run([sys.executable, str(SCRIPT), str(source), str(output)], capture_output=True, text=True)
            self.assertEqual(run.returncode, 0, run.stderr)
            with zipfile.ZipFile(output) as bundle:
                self.assertEqual(bundle.read(declared), b'license terms')

    def test_rejects_symlinked_notice_instead_of_silently_omitting_it(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source, output = root / 'source.tar.gz', root / 'notices.zip'
            with tarfile.open(source, 'w:gz') as tar:
                add_file(tar, 'source/SOURCE_MANIFEST.json', b'{"schema_version":1,"packages":[]}')
                link = tarfile.TarInfo('source/vendor/example/LICENSE')
                link.type = tarfile.SYMTYPE
                link.linkname = 'elsewhere'
                tar.addfile(link)
            self.assertNotEqual(subprocess.run([sys.executable, str(SCRIPT), str(source), str(output)], capture_output=True).returncode, 0)
            self.assertFalse(output.exists())


if __name__ == '__main__':
    unittest.main()
