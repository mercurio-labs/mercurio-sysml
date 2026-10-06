"""Restore disposable 2026-08 assessment inputs from pinned upstream sources.

Run from mercurio-sysml. Historical reports are never rewritten. The generated
library has fresh path/time provenance; its new digest is recorded explicitly.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import urllib.request
import zipfile

from audit_release_samples import corpus, discover

PILOT = '692170b71867353b8f90341e61556f49a5beb0e5'
RELEASE = 'fb97b754f29588b8e9c7a35f370880cd15eb29e7'
JAR_SHA = 'b1ad9d64b1f0c75730facf25a5e2856bc9df2bb4bd39476df2fdf5ae68cd9350'
URL = 'https://github.com/Systems-Modeling/SysML-v2-Pilot-Implementation/releases/download/2026-08/jupyter-sysml-kernel-0.62.0.zip'
ROOT = Path(__file__).resolve().parents[1]


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def run(*args):
    subprocess.run([str(a) for a in args], check=True, cwd=ROOT)


def git(path, *args):
    return subprocess.check_output(['git', '-C', str(path), *args], text=True).strip()


def write(path, data):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(data, indent=2) + '\n', encoding='utf-8')


def checkout(path, repo, commit, seed=None):
    if not path.exists():
        if seed and subprocess.run(['git', '-C', str(seed), 'cat-file', '-e', commit + '^{commit}'],
                                   stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL).returncode == 0:
            run('git', 'clone', '--no-checkout', str(seed), path)
            run('git', '-C', path, 'checkout', '--detach', commit)
        else:
            run('git', 'clone', '--depth', '1', '--branch', '2026-08',
                f'https://github.com/Systems-Modeling/{repo}.git', path)
    if git(path, 'rev-parse', 'HEAD') != commit or git(path, 'status', '--porcelain'):
        raise ValueError(f'Expected clean pinned checkout; refusing to replace {path}')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--stage', choices=['inputs', 'export', 'library'], required=True)
    parser.add_argument('--artifacts', type=Path, default=ROOT.parent / 'target')
    parser.add_argument('--pilot-seed', type=Path)
    parser.add_argument('--java-bin', type=Path)
    args = parser.parse_args()
    root = args.artifacts.resolve()
    upstream = root / 'upstream'
    support = root / 'support-2026-08'
    upstream.mkdir(parents=True, exist_ok=True)
    support.mkdir(parents=True, exist_ok=True)
    pilot = upstream / 'SysML-v2-Pilot-Implementation'
    release = upstream / 'SysML-v2-Release'
    jar = upstream / 'kernel-0.62.0/sysml/jupyter-sysml-kernel-0.62.0-all.jar'
    if args.stage == 'inputs':
        checkout(pilot, 'SysML-v2-Pilot-Implementation', PILOT, args.pilot_seed)
        checkout(release, 'SysML-v2-Release', RELEASE)
        if not jar.exists():
            archive = upstream / 'jupyter-sysml-kernel-0.62.0.zip'
            if not archive.exists():
                partial = archive.with_suffix('.download')
                print('Downloading pinned Pilot runtime', flush=True)
                with urllib.request.urlopen(URL, timeout=120) as response, partial.open('wb') as output:
                    shutil.copyfileobj(response, output)
                partial.replace(archive)
            with zipfile.ZipFile(archive) as bundle:
                names = [n for n in bundle.namelist() if n.endswith('/' + jar.name)]
                if len(names) != 1:
                    raise ValueError('Runtime archive must contain exactly one expected JAR')
                jar.parent.mkdir(parents=True, exist_ok=True)
                jar.write_bytes(bundle.read(names[0]))
        if digest(jar) != JAR_SHA:
            raise ValueError('Pinned runtime JAR digest mismatch')
        seed = json.loads((ROOT / 'crates/mercurio-tools/corpus/pilot_corpus.seed.json').read_text(encoding='utf-8'))
        paths = discover(release)
        cases = corpus(release, paths, seed['support_dependencies'])
        if len(cases) != 310:
            raise ValueError(f'Expected 310 samples, got {len(cases)}')
        hashes = {}
        for case in cases:
            for name in case['input_files']:
                path = Path(name)
                relative = path.relative_to(release).as_posix()
                hashes[relative] = digest(path)
                if hashes[relative] != digest(pilot / relative):
                    raise ValueError('Release/Pilot source mismatch: ' + relative)
        historical = json.loads((ROOT / 'docs/conformance/2026-08-declaration-validation-assessment/source-lock.json').read_text(encoding='utf-8'))
        if hashes != historical['sources']:
            raise ValueError('Source corpus differs from the retained historical assessment')
        write(root / 'release-audit-2026-08-final/corpus.json', {'cases': cases})
        write(support / 'recovered-inputs.json', {'pilot_commit': PILOT, 'release_commit': RELEASE,
            'jar_sha256': digest(jar), 'sources': hashes, 'cases': len(cases),
            'historical_sources_verified': True, 'recipe_sha256': digest(Path(__file__))})
        print(f'Restored {len(cases)} cases; all source hashes and runtime JAR verified', flush=True)
        return
    if digest(jar) != JAR_SHA:
        raise ValueError('Pinned runtime JAR digest mismatch')
    checkout(pilot, 'SysML-v2-Pilot-Implementation', PILOT)
    java = str(args.java_bin / 'java.exe') if args.java_bin else 'java'
    javac = str(args.java_bin / 'javac.exe') if args.java_bin else 'javac'
    classes = support / 'stdlib-classes'
    classes.mkdir(exist_ok=True)
    export = support / 'pilot-stdlib-types-export.json'
    export_lock = support / 'stdlib-export-inputs.json'
    helper = ROOT / 'tools/pilot-exporter/src/main/java/dev/mercurio/pilot/PilotStdlibExporter.java'
    inputs = {'pilot_commit': PILOT, 'jar_sha256': digest(jar), 'helper_sha256': digest(helper)}
    if export.exists() and export_lock.exists():
        prior = json.loads(export_lock.read_text(encoding='utf-8'))
        if prior['inputs'] != inputs or prior['export_sha256'] != digest(export):
            raise ValueError('Existing stdlib export provenance changed')
    else:
        run(javac, '-cp', jar, '-d', classes, helper)
        run(java, '-Xmx3g', '-cp', str(classes) + os.pathsep + str(jar),
            'dev.mercurio.pilot.PilotStdlibExporter', pilot / 'sysml.library', export)
        write(export_lock, {'inputs': inputs, 'export_sha256': digest(export)})
    if args.stage == 'export':
        print('Pinned library export verified', flush=True)
        return
    library = support / 'stdlib.inherited-types.kir.json'
    run(ROOT / 'target/release/import_pilot_stdlib.exe', '--pilot-root', pilot,
        '--pilot-jar', jar, '--from-export', export, '--out', library,
        '--rulepack-out', support / 'stdlib.rulepack.json')
    document = json.loads(library.read_text(encoding='utf-8'))
    if len(document['elements']) != 17166:
        raise ValueError('Regenerated library element count differs from retained baseline')
    write(support / 'recovered-library.json', {'kir_elements': len(document['elements']),
        'sha256': digest(library), 'export_sha256': digest(export), 'pilot_commit': PILOT,
        'jar_sha256': digest(jar), 'provenance': 'Fresh export/import after deleted build artifacts; not byte-identical historical evidence.'})
    print('Candidate library regenerated:', digest(library), flush=True)


if __name__ == '__main__':
    main()
