"""Focused build-time Pilot parse-value oracle; no resolution or full corpus run."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]
PIN = '692170b71867353b8f90341e61556f49a5beb0e5'
HELPER = ROOT / 'tools/pilot-exporter/src/main/java/dev/mercurio/pilot/PilotExpandedDecisionProbe.java'
OUTPUT = ROOT / 'docs/conformance/2026-08-support/expanded-decision-pilot-controls.json'


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
        'org.omg.sysml.logic/src/main/java/org/omg/sysml/util/ElementUtil.java',
        'org.omg.kerml.expressions.xtext/src/org/omg/kerml/expressions/xtext/KerMLExpressions.xtext',
        'org.omg.kerml.xtext/src/org/omg/kerml/xtext/KerML.xtext',
        'org.omg.sysml.xtext/src/org/omg/sysml/xtext/SysML.xtext',
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
    with tempfile.TemporaryDirectory(prefix='expanded-decision-probe-') as directory:
        directory = Path(directory)
        subprocess.run([str(args.java_bin / ('javac' + suffix)), '-encoding', 'UTF-8', '-cp',
                        str(jar), '-d', str(directory), str(HELPER)], check=True, timeout=60)
        output = directory / 'result.json'
        subprocess.run([str(args.java_bin / ('java' + suffix)), '-cp', str(directory) + os.pathsep + str(jar),
                        'dev.mercurio.pilot.PilotExpandedDecisionProbe', str(output)], check=True, timeout=60)
        cases = json.loads(output.read_text(encoding='utf-8'))
        names = json.loads(Path(str(output) + '.names.json').read_text(encoding='utf-8'))
        if len(names) != 26:
            raise ValueError('Incomplete normalized-name probe')
    if len(cases) != 37:
        raise ValueError('Incomplete expanded-decision probe')
    if provenance['jar_sha256'] != digest(jar.read_bytes()) or provenance['helper_sha256'] != digest(HELPER.read_bytes()):
        raise ValueError('Changed probe inputs')
    result = {'scope': 'Nine expanded-budget, upstream predicate-retry, first-set and hoisted-predicate decision families in KerML and SysML: independent generated-parser node evidence. Native checks cover decision execution only, not complete model construction. No linking, normalization, specification conformance or full sample qualification.',
              'provenance': provenance, 'cases': cases}
    text = json.dumps(result, indent=2) + '\n'
    if args.check:
        if OUTPUT.read_text(encoding='utf-8') != text:
            raise ValueError('Stale expanded-decision evidence')
    else:
        OUTPUT.write_text(text, encoding='utf-8', newline='\n')
    name_output = OUTPUT.with_name('name-values-pilot-controls.json')
    name_result = {'scope': 'Parsed raw declaredName and explicit ElementUtil.unescapeString values. Native represented names follow pinned KerML 1.1 section 8.2.2.3; raw Pilot lexical storage remains distinct. This does not qualify scoping or complete name resolution.',
                   'provenance': provenance, 'cases': names}
    name_text = json.dumps(name_result, indent=2) + '\n'
    if args.check:
        if name_output.read_text(encoding='utf-8') != name_text:
            raise ValueError('Stale represented-name evidence')
    else:
        name_output.write_text(name_text, encoding='utf-8', newline='\n')
    print('Pinned expanded-decision values verified: 37 branches and 26 represented names')


if __name__ == '__main__':
    main()
