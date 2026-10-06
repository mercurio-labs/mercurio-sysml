"""Export bounded pinned Xtext library import inventory; no validation."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]
PIN = '692170b71867353b8f90341e61556f49a5beb0e5'
HELPER = ROOT / 'tools/pilot-exporter/src/main/java/dev/mercurio/pilot/PilotLibraryImportInventory.java'
OUTPUT = ROOT / 'docs/conformance/2026-08-support/library-import-inventory.json'


def digest(data):
    return hashlib.sha256(data).hexdigest()


def read_bytes(path):
    path = Path(path).resolve()
    if os.name == 'nt' and not str(path).startswith('\\\\?\\'):
        path = Path('\\\\?\\' + str(path))
    return path.read_bytes()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--java-bin', type=Path, required=True)
    parser.add_argument('--pilot', type=Path, default=ROOT.parent / 'target/support-2026-08/pilot-pinned-2026-08')
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    def git(*command):
        # Git 2.36 ignores command-local safe.directory; isolate the exception
        # in an ephemeral global config rather than changing the user's config.
        with tempfile.TemporaryDirectory(prefix='resource-probe-git-') as directory:
            config = Path(directory) / 'gitconfig'
            config.write_text('[safe]\n\tdirectory = ' + args.pilot.resolve().as_posix() + '\n', encoding='utf-8')
            environment = dict(os.environ, GIT_CONFIG_GLOBAL=str(config))
            return subprocess.check_output(['git', '-C', str(args.pilot), *command], env=environment)
    if git('rev-parse', 'HEAD').decode().strip() != PIN:
        raise ValueError('Unexpected Pilot revision')
    paths = [
        'org.omg.kerml.xtext/src/org/omg/kerml/xtext/KerML.xtext',
        'org.omg.sysml.xtext/src/org/omg/sysml/xtext/SysML.xtext',
        'org.omg.kerml.xtext/src/org/omg/kerml/xtext/naming/KerMLQualifiedNameConverter.xtend',
        'org.omg.sysml/model/SysML.ecore',
    ]
    sources = {}
    for path in paths:
        actual = (args.pilot / path).read_bytes().replace(b'\r\n', b'\n')
        if actual != git('show', PIN + ':' + path).replace(b'\r\n', b'\n'):
            raise ValueError('Changed pinned source: ' + path)
        sources[path] = digest(actual)
    jar = args.pilot / 'org.omg.sysml.interactive/target/org.omg.sysml.interactive-0.62.0-SNAPSHOT-all.jar'
    provenance = {'pilot_commit': PIN, 'source_sha256': sources,
                  'jar_sha256': digest(jar.read_bytes()), 'helper_sha256': digest(HELPER.read_bytes()),
                  'driver_sha256': digest(Path(__file__).read_bytes())}
    lock_path = ROOT / 'docs/conformance/2026-08-support/definition-pipeline-evidence/census-2026-10-02/input-lock.json'
    lock = json.loads(lock_path.read_text(encoding='utf-8'))
    inventory = lock['library_inventory']
    for row in inventory:
        if digest(read_bytes(row['path'])) != row['sha256']:
            raise ValueError('Changed library input: ' + row['release_path'])
    provenance['library_inputs'] = {row['release_path']: row['sha256'] for row in inventory}
    suffix = '.exe' if os.name == 'nt' else ''
    with tempfile.TemporaryDirectory(prefix='library-imports-') as directory:
        directory = Path(directory)
        subprocess.run([str(args.java_bin / ('javac' + suffix)), '-encoding', 'UTF-8', '-cp', str(jar), '-d', str(directory), str(HELPER)], check=True, timeout=60)
        inputs = directory / 'inputs.txt'
        inputs.write_text('\n'.join(row['path'] for row in inventory) + '\n', encoding='utf-8')
        output = directory / 'result.json'
        subprocess.run([str(args.java_bin / ('java' + suffix)), '-cp', str(directory) + os.pathsep + str(jar), 'dev.mercurio.pilot.PilotLibraryImportInventory', str(inputs), str(output)], check=True, timeout=60)
        resources = json.loads(output.read_text(encoding='utf-8'))
    if len(resources) != len(inventory):
        raise ValueError('Incomplete import inventory')
    for resource, source in zip(resources, inventory):
        if Path(resource.pop('path')).resolve() != Path(source['path']).resolve():
            raise ValueError('Resource identity mismatch')
        resource['release_path'] = source['release_path']
    provenance['java_sha256'] = digest((args.java_bin / ('java' + suffix)).read_bytes())
    provenance['javac_sha256'] = digest((args.java_bin / ('javac' + suffix)).read_bytes())
    provenance['java_version'] = subprocess.check_output([str(args.java_bin / ('java' + suffix)), '-version'], stderr=subprocess.STDOUT).decode().strip()
    result = {'schema': 'dev.mercurio.library-import-inventory.v1', 'scope': 'Parsed Xtext import declarations and package paths only; no semantic target linking, implicit global dependencies or native support qualification.', 'provenance': provenance, 'resources': resources}
    text = json.dumps(result, indent=2, sort_keys=True) + '\n'
    if args.check:
        if OUTPUT.read_text(encoding='utf-8') != text: raise ValueError('Stale library import inventory')
    else: OUTPUT.write_text(text, encoding='utf-8', newline='\n')
    print('Parsed Xtext library import inventory:', len(resources), 'resources')

if __name__ == '__main__': main()
