"""Build-time Pilot construction oracle; no sample qualification or timing."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import tempfile
import zipfile

ROOT = Path(__file__).resolve().parents[1]
PIN = '692170b71867353b8f90341e61556f49a5beb0e5'
HELPER = ROOT / 'tools/pilot-exporter/src/main/java/dev/mercurio/pilot/PilotOperandProbe.java'
OUTPUT = ROOT / 'docs/conformance/2026-08-support/operand-construction-pilot-controls.json'
SOURCES = [
    'delegate/setting/InvocationExpression_operand_SettingDelegate',
    'delegate/setting/FeatureValue_value_SettingDelegate',
    'delegate/setting/OwningMembership_ownedMemberElement_SettingDelegate',
    'adapter/InvocationExpressionAdapter', 'util/OperandEList',
    'util/ExpressionUtil', 'util/TypeUtil', 'util/FeatureUtil',
]

def digest(data):
    return hashlib.sha256(data).hexdigest()

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--java-bin', type=Path)
    parser.add_argument('--pilot', type=Path, default=ROOT.parent / 'target/support-2026-08/pilot-pinned-2026-08')
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    def git(*command):
        return subprocess.check_output(['git', '-C', str(args.pilot), *command])
    if git('rev-parse', 'HEAD').decode().strip() != PIN:
        raise ValueError('Unexpected Pilot revision')
    jar = args.pilot / 'org.omg.sysml.interactive/target/org.omg.sysml.interactive-0.62.0-SNAPSHOT-all.jar'
    source_hashes = {}
    for name in SOURCES:
        relative = 'org.omg.sysml.logic/src/main/java/org/omg/sysml/' + name + '.java'
        actual = (args.pilot / relative).read_bytes().replace(b'\r\n', b'\n')
        if actual != git('show', PIN + ':' + relative).replace(b'\r\n', b'\n'):
            raise ValueError('Modified pinned source: ' + relative)
        source_hashes[relative] = digest(actual)
    with zipfile.ZipFile(jar) as archive:
        class_hashes = {name: digest(archive.read('org/omg/sysml/' + name + '.class')) for name in SOURCES}
    provenance = {
        'pilot_commit': PIN, 'source_sha256': source_hashes,
        'jar_sha256': digest(jar.read_bytes()), 'class_sha256': class_hashes,
        'helper_sha256': digest(HELPER.read_bytes()),
    }
    suffix = '.exe' if os.name == 'nt' else ''
    def exe(name):
        return str(args.java_bin / (name + suffix)) if args.java_bin else name
    with tempfile.TemporaryDirectory(prefix='operand-probe-') as folder:
        folder = Path(folder)
        subprocess.run([exe('javac'), '-encoding', 'UTF-8', '-cp', str(jar), '-d', str(folder), str(HELPER)], check=True, timeout=60)
        output = folder / 'result.json'
        subprocess.run([exe('java'), '-cp', str(folder) + os.pathsep + str(jar),
                        'dev.mercurio.pilot.PilotOperandProbe', str(output)], check=True, timeout=60)
        cases = json.loads(output.read_text(encoding='utf-8'))
    if len(cases) != 7 or len({case['class'] for case in cases}) != 7:
        raise ValueError('Concrete invocation-class coverage changed')
    if provenance['jar_sha256'] != digest(jar.read_bytes()) or provenance['helper_sha256'] != digest(HELPER.read_bytes()):
        raise ValueError('Probe inputs changed during execution')
    result = {
        'schema': 'dev.mercurio.operand-construction-controls.v1',
        'scope': 'Two distinct equal-valued operands appended through the actual InternalEList adapter for all seven concrete InvocationExpression classes. Compare ordered stored containment, wrapper classes, visibility, input direction, literal values and ownership inverses. No library loading, argument derivation, specialization, reparenting, control-function bodies or full sample qualification.',
        'normative_boundary': 'operand is a Pilot Xtext parsing extension, not a normative stored property. Its append adapter constructs the input-parameter/FeatureValue structure described by KerML 1.1 Beta 2 sections 8.4.4.9.5 and 8.4.4.9.6. Its empty read facade is not InstantiationExpression.argument.',
        'provenance': provenance, 'cases': cases,
    }
    text = json.dumps(result, indent=2) + '\n'
    if args.check:
        if OUTPUT.read_text(encoding='utf-8') != text:
            raise ValueError('Operand construction evidence changed')
    else:
        OUTPUT.write_text(text, encoding='utf-8', newline='\n')
    print(f'Observed operand construction for {len(cases)} concrete invocation classes')

if __name__ == '__main__':
    main()
