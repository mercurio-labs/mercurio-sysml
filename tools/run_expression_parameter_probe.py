"""Export resolved instantiation type dispatch and bounded default observations; no validation."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]
PIN = '692170b71867353b8f90341e61556f49a5beb0e5'
HELPER = ROOT / 'tools/pilot-exporter/src/main/java/dev/mercurio/pilot/PilotExpressionParameterProbe.java'
OUTPUT = ROOT / 'docs/conformance/2026-08-support/expression-parameter-pilot-controls.json'


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
    paths = ['org.omg.sysml/model/SysML.ecore', *['org.omg.sysml.logic/src/main/java/org/omg/sysml/'+p for p in ['adapter/ExpressionAdapter.java','adapter/FeatureAdapter.java','util/FeatureUtil.java','util/TypeUtil.java']]]
    sources = {}
    for path in paths:
        actual = (args.pilot / path).read_bytes().replace(b'\r\n', b'\n')
        if actual != git('show', PIN + ':' + path).replace(b'\r\n', b'\n'):
            raise ValueError('Changed pinned source: ' + path)
        sources[path] = digest(actual)
    jar = args.pilot / 'org.omg.sysml.interactive/target/org.omg.sysml.interactive-0.62.0-SNAPSHOT-all.jar'
    if digest(jar.read_bytes()) != '4512852638fc799505fe676955e8c78fedef1f3ce44ec90b57b34c05c31e6945': raise ValueError('Changed pinned Pilot runtime')
    provenance = {'pilot_commit': PIN, 'source_sha256': sources,
                  'jar_sha256': digest(jar.read_bytes()), 'helper_sha256': digest(HELPER.read_bytes()),
                  'driver_sha256': digest(Path(__file__).read_bytes())}
    suffix = '.exe' if os.name == 'nt' else ''
    with tempfile.TemporaryDirectory(prefix='resource-link-probe-') as directory:
        directory = Path(directory)
        subprocess.run([str(args.java_bin / ('javac' + suffix)), '-encoding', 'UTF-8', '-cp', str(jar), '-d', str(directory), *[str(HELPER.with_name(h)) for h in ['PilotExpressionParameterProbe.java','PilotFeatureRedefinitionExporter.java','PilotMixedParameterProbe.java']]], check=True, timeout=60)
        output = directory / 'result.json'
        subprocess.run([str(args.java_bin / ('java' + suffix)), '-cp', str(directory) + os.pathsep + str(jar), 'dev.mercurio.pilot.PilotExpressionParameterProbe', str(jar), str(output), *[str(args.pilot / p) for p in paths[1:3]]], check=True, timeout=90)
        cases = json.loads(output.read_text(encoding='utf-8'))
    if len(cases['methods']) != 1 or len(cases['bindings']) != 20 or len(cases['controls']) != 960: raise ValueError('Incomplete expression parameter inventory: '+str((list(cases['methods']),list(cases['bindings']),len(cases['controls']))))
    for helper in ['PilotFeatureRedefinitionExporter.java','PilotMixedParameterProbe.java']: provenance[helper] = digest(HELPER.with_name(helper).read_bytes())
    provenance['java_sha256'] = digest((args.java_bin / ('java' + suffix)).read_bytes())
    provenance['javac_sha256'] = digest((args.java_bin / ('javac' + suffix)).read_bytes())
    provenance['java_version'] = subprocess.check_output([str(args.java_bin / ('java' + suffix)), '-version'], stderr=subprocess.STDOUT).decode().strip()
    result = {'schema': 'dev.mercurio.expression-parameter-controls.v1',
              'scope': 'Expression positional queries on supplied complete Function ancestry across all 20 bindings. Transition guards, full lifecycle and native publication remain separate.',
              'provenance': provenance, **cases}
    text = json.dumps(result, indent=2, sort_keys=True) + '\n'
    if args.check:
        if OUTPUT.read_text(encoding='utf-8') != text:
            raise ValueError('Stale expression parameter controls')
    else:
        OUTPUT.write_text(text, encoding='utf-8', newline='\n')
    print('Pinned expression parameter strategy:',len(cases['methods']),'typed programs,',len(cases['controls']),'independent observations')


if __name__ == '__main__':
    main()

