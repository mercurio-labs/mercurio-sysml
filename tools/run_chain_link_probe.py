"""Focused build-time Pilot chain-link oracle; no full corpus or validation run."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]
PIN = '692170b71867353b8f90341e61556f49a5beb0e5'
HELPER = ROOT / 'tools/pilot-exporter/src/main/java/dev/mercurio/pilot/PilotChainLinkProbe.java'
OUTPUT = ROOT / 'docs/conformance/2026-08-support/chain-link-pilot-controls.json'


def digest(data):
    return hashlib.sha256(data).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--java-bin', type=Path, required=True)
    parser.add_argument('--pilot', type=Path, default=ROOT.parent / 'target/support-2026-08/pilot-pinned-2026-08')
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    def git(*command):
        return subprocess.check_output(['git', '-C', str(args.pilot), *command])
    if git('rev-parse', 'HEAD').decode().strip() != PIN:
        raise ValueError('Unexpected Pilot revision')
    paths = [
        'org.omg.kerml.xtext/src/org/omg/kerml/xtext/conversion/KerMLValueConverterService.xtend',
        'org.omg.kerml.expressions.xtext/src/org/omg/kerml/expressions/xtext/KerMLExpressions.xtext',
        'org.omg.kerml.xtext/src/org/omg/kerml/xtext/KerML.xtext',
        'org.omg.sysml.xtext/src/org/omg/sysml/xtext/SysML.xtext',
        'org.omg.kerml.xtext/src/org/omg/kerml/xtext/scoping/KerMLScopeProvider.xtend',
        'org.omg.kerml.xtext/src/org/omg/kerml/xtext/scoping/KerMLScope.xtend',
        'org.omg.sysml.logic/src/main/java/org/omg/sysml/util/FeatureUtil.java',
        'org.omg.sysml.logic/src/main/java/org/omg/sysml/util/TypeUtil.java',
        'org.omg.sysml.logic/src/main/java/org/omg/sysml/util/NamespaceUtil.java',
    ]
    sources = {}
    for path in paths:
        actual = (args.pilot / path).read_bytes().replace(b'\r\n', b'\n')
        if actual != git('show', PIN + ':' + path).replace(b'\r\n', b'\n'):
            raise ValueError('Changed pinned source: ' + path)
        sources[path] = digest(actual)
    jar = args.pilot / 'org.omg.sysml.interactive/target/org.omg.sysml.interactive-0.62.0-SNAPSHOT-all.jar'
    provenance = {'pilot_commit': PIN, 'source_sha256': sources,
                  'jar_sha256': digest(jar.read_bytes()), 'helper_sha256': digest(HELPER.read_bytes())}
    suffix = '.exe' if os.name == 'nt' else ''
    with tempfile.TemporaryDirectory(prefix='feature-chain-probe-') as directory:
        directory = Path(directory)
        subprocess.run([str(args.java_bin / ('javac' + suffix)), '-encoding', 'UTF-8', '-cp',
                        str(jar), '-d', str(directory), str(HELPER)], check=True, timeout=60)
        output = directory / 'result.json'
        subprocess.run([str(args.java_bin / ('java' + suffix)), '-cp', str(directory) + os.pathsep + str(jar),
                        'dev.mercurio.pilot.PilotChainLinkProbe', str(output)], check=True, timeout=60)
        cases = json.loads(output.read_text(encoding='utf-8'))
        names = json.loads(Path(str(output) + '.names.json').read_text(encoding='utf-8'))
        if len(names) != 6 or sum(case['accepted'] for case in names) != 4:
            raise ValueError('Changed identification controls')
    if len(cases) != 58 or [len(case['links']) for case in cases] != [3] + [2] * 41 + [3] * 8 + [2] * 8:
        raise ValueError('Incomplete feature-chain probe')
    for index, case in enumerate(cases[50:]):
        first, last = case['links']
        if not first['resolved'] or first.get('target_path') != ['C', 'sameThing']:
            raise ValueError('Changed reciprocal scope origin')
        if last['resolved'] != (index < 4):
            raise ValueError('Changed reciprocal positive/negative controls')
        if index < 4 and last.get('target_path') != ['Base', 'Anything', 'self']:
            raise ValueError('Changed reciprocal inherited target')
    if provenance['jar_sha256'] != digest(jar.read_bytes()) or provenance['helper_sha256'] != digest(HELPER.read_bytes()):
        raise ValueError('Changed probe inputs')
    result = {'scope': 'Actual KerML Xtext resource linking for typed FeatureChain references. Ordered targets and unresolved links are observed independently. Six additional raw declaration-identification acceptance controls. Eight source-order/cross-specialization dependency controls compare chained reference targets only; complete Feature transformation remains unqualified. Eight reciprocal-subsetting controls observe inherited chain resolution in both member and library declaration orders. No full validation, specification qualification, or complete scope coverage.',
              'provenance': provenance, 'cases': cases[:29], 'candidate_scope_controls': cases[29:50], 'reciprocal_scope_controls': cases[50:], 'identification_controls': names}
    text = json.dumps(result, indent=2) + '\n'
    if args.check:
        if OUTPUT.read_text(encoding='utf-8') != text:
            raise ValueError('Stale feature-chain evidence')
    else:
        OUTPUT.write_text(text, encoding='utf-8', newline='\n')
    print('Pinned chain links verified: twenty-nine resource controls plus twenty-one candidate scope controls plus eight reciprocal-scope controls')


if __name__ == '__main__':
    main()
