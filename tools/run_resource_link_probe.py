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
HELPER = ROOT / 'tools/pilot-exporter/src/main/java/dev/mercurio/pilot/PilotResourceLinkProbe.java'
OUTPUT = ROOT / 'docs/conformance/2026-08-support/resource-link-pilot-controls.json'


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
        subprocess.run([str(args.java_bin / ('java' + suffix)), '-cp', str(directory) + os.pathsep + str(jar), 'dev.mercurio.pilot.PilotResourceLinkProbe', str(output)], check=True, timeout=60)
        cases = json.loads(output.read_text(encoding='utf-8'))
    if len(cases) != 12 or any(len(case['links']) != 1 for case in cases):
        raise ValueError('Incomplete fixed resource matrix')
    for case in cases:
        case['exports'].sort(key=lambda export: (export['uri'], export['name'], export['kind']))
        if not case['description_service'].endswith('ResourceSetBasedResourceDescriptions'):
            raise ValueError('Live resource descriptions not configured')
        positive = case['name'] not in ['kerml_private_export', 'kerml_missing_endpoint']
        link = case['links'][0]
        if link['resolved'] != positive:
            raise ValueError('Unexpected cross-resource linking: ' + case['name'] + ': ' + str(case))
        expected_uri = None
        if case['name'].startswith('duplicate_export'):
            expected_uri = case['units'][1]['uri']
        elif positive:
            expected_uri = next(unit['uri'] for unit in case['units'] if '/provider.' in unit['uri'])
        if positive and (link.get('target_uri') != expected_uri or link.get('target_path') != ['A', 'T']):
            raise ValueError('Changed declared target: ' + case['name'])
        if positive and case['diagnostics']:
            raise ValueError('Unexpected positive linking diagnostic: ' + case['name'])
        if not positive and not case['diagnostics']:
            raise ValueError('Missing unresolved diagnostic: ' + case['name'])
        case['assertion'] = {'resolved': positive, 'target_uri': expected_uri,
                             'target_path': ['A', 'T'] if positive else None,
                             'boundary': 'Upstream duplicate export selects first loaded provider; native rejection is a separate fail-closed policy.' if case['name'].startswith('duplicate_export') else 'Stored FeatureTyping.type resource linking only.'}
    if provenance['jar_sha256'] != digest(jar.read_bytes()) or provenance['helper_sha256'] != digest(HELPER.read_bytes()):
        raise ValueError('Changed probe inputs')
    result = {'scope': 'Twelve bounded controls using individually parsed KerML/SysML Xtext resources in one ResourceSet. Xtext live ResourceSet descriptions configure upstream Pilot global scoping; all name lookup and lazy linking use upstream services. Observations cover FeatureTyping.type only, including declared target path, actual source URI and unresolved resource diagnostics. Duplicate exports record upstream selection rather than claiming ambiguity rejection. No full validation, persistence qualification, complete graph equivalence, implicit completion, or general resource conformance.',
              'provenance': provenance, 'cases': cases}
    text = json.dumps(result, indent=2, sort_keys=True) + '\n'
    if args.check:
        if OUTPUT.read_text(encoding='utf-8') != text:
            raise ValueError('Stale resource-link evidence')
    else:
        OUTPUT.write_text(text, encoding='utf-8', newline='\n')
    print('Pinned multi-resource links verified: twelve fixed controls')


if __name__ == '__main__':
    main()
