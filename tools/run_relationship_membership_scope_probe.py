"""Export bounded pinned Pilot multi-resource linking observations; no validation."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]
PIN = '692170b71867353b8f90341e61556f49a5beb0e5'
HELPER = ROOT / 'tools/pilot-exporter/src/main/java/dev/mercurio/pilot/PilotRelationshipMembershipScopeProbe.java'
OUTPUT = ROOT / 'docs/conformance/2026-08-support/relationship-membership-scope-pilot-controls.json'


def digest(data):
    return hashlib.sha256(data).hexdigest()


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
        'org.omg.kerml.xtext/src/org/omg/kerml/xtext/scoping/KerMLScopeProvider.xtend',
        'org.omg.kerml.xtext/src/org/omg/kerml/xtext/scoping/KerMLScope.xtend',
        'org.omg.kerml.xtext/src/org/omg/kerml/xtext/scoping/KerMLGlobalScopeProvider.xtend',
        'org.omg.kerml.xtext/src/org/omg/kerml/xtext/scoping/KerMLGlobalScope.xtend',
        'org.omg.kerml.xtext/src/org/omg/kerml/xtext/KerMLRuntimeModule.xtend',
        'org.omg.kerml.xtext/src/org/omg/kerml/xtext/naming/KerMLQualifiedNameProvider.java',
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
    suffix = '.exe' if os.name == 'nt' else ''
    with tempfile.TemporaryDirectory(prefix='resource-link-probe-') as directory:
        directory = Path(directory)
        subprocess.run([str(args.java_bin / ('javac' + suffix)), '-encoding', 'UTF-8', '-cp', str(jar), '-d', str(directory), str(HELPER)], check=True, timeout=60)
        output = directory / 'result.json'
        subprocess.run([str(args.java_bin / ('java' + suffix)), '-cp', str(directory) + os.pathsep + str(jar), 'dev.mercurio.pilot.PilotRelationshipMembershipScopeProbe', str(output)], check=True, timeout=60)
        cases = json.loads(output.read_text(encoding='utf-8'))
    if len(cases)!=3 or any(len(c['links'])!=1 for c in cases):raise ValueError('Incomplete three-case relationship membership scope matrix')
    for case in cases:
        positive=not case['name'].startswith('Redefinition_')
        link=case['links'][0];expected=['P','host','b']
        if link['resolved']!=positive or (positive and link['target_path']!=expected):raise ValueError('Unexpected scope observation: '+str(case))
        if positive and case['diagnostics']:raise ValueError('Unexpected positive diagnostic: '+str(case))
        if not positive and not case['diagnostics']:raise ValueError('Missing negative diagnostic')
        specific=link.get('specific_path',link.get('type_disjoined_path'))
        if specific!=['P','host','a']:raise ValueError('Changed specific endpoint observation')
        case['assertion']={'resolved':positive,'target_path':expected if positive else None,'specific_path':specific,'scope':'Stored standalone relationship endpoints; local Redefinition target excluded. No validation.'}
    if provenance['jar_sha256'] != digest(jar.read_bytes()) or provenance['helper_sha256'] != digest(HELPER.read_bytes()):
        raise ValueError('Changed probe inputs')
    result = {'scope': 'Three pinned Xtext controls for standalone declarations inside a Feature. Two local endpoint positives and a local Redefinition exclusion; no validation or complete scope qualification.',
              'provenance': provenance, 'cases': cases}
    text = json.dumps(result, indent=2, sort_keys=True) + '\n'
    if args.check:
        if OUTPUT.read_text(encoding='utf-8') != text:
            raise ValueError('Stale resource-link evidence')
    else:
        OUTPUT.write_text(text, encoding='utf-8', newline='\n')
    print('Pinned standalone relationship scope: three fixed controls')


if __name__ == '__main__':
    main()
