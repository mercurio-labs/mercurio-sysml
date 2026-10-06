"""Export bounded pinned Pilot resolved-input binding transformation observations; no validation."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]
PIN = '692170b71867353b8f90341e61556f49a5beb0e5'
HELPER = ROOT / 'tools/pilot-exporter/src/main/java/dev/mercurio/pilot/PilotFreshTransitionLinkProbe.java'
OUTPUT = ROOT / 'docs/conformance/2026-08-support/fresh-transition-link-pilot-controls.json'


def digest(data):
    return hashlib.sha256(data).hexdigest()


def validate(cases):
    controls=cases['controls']
    if len(controls)!=16 or {(c['role_mask'],c['preceding_parameter']) for c in controls}!={(m,p) for m in range(8) for p in (False,True)} or len(cases['methods'])!=8:
        raise ValueError('Incomplete fresh transition-link controls')
    for c in controls:
        if set(c.get('library_inputs',{}))!={'dataValues','objects','occurrences'} or c.get('role_input',{}).get('complete') is not True or any(v['complete'] is not True for v in c['library_inputs'].values()):raise ValueError('Incomplete supplied library stage')
        if c['complete_before'] is not False or c['after']['complete'] is not True or c['after']!=c['repeat'] or c['may_time_vary'] is not False:
            raise ValueError('Changed fresh fixed transition-link lifecycle')


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
    paths = ['org.omg.sysml/model/SysML.ecore', *['org.omg.sysml.logic/src/main/java/org/omg/sysml/'+p for p in ['adapter/ReferenceUsageAdapter.java','adapter/UsageAdapter.java','adapter/FeatureAdapter.java','adapter/TransitionUsageAdapter.java','adapter/TypeAdapter.java','util/UsageUtil.java','util/TypeUtil.java','util/ConnectorUtil.java','util/FeatureUtil.java']]]
    paths.append('org.omg.sysml.model/src/main/java/org/omg/sysml/lang/sysml/impl/UsageImpl.java')
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
        subprocess.run([str(args.java_bin / ('javac' + suffix)), '-encoding', 'UTF-8', '-cp', str(jar), '-d', str(directory), str(HELPER), *[str(HELPER.with_name(n+'.java')) for n in ('PilotCompatibilityExporter','PilotResultConstructionExporter','PilotOperandProbe','PilotOwnerTypingProbe','PilotStepStrategyProbe')]], check=True, timeout=60)
        output = directory / 'result.json'
        subprocess.run([str(args.java_bin / ('java' + suffix)), '-cp', str(directory) + os.pathsep + str(jar), 'dev.mercurio.pilot.PilotFreshTransitionLinkProbe', str(jar), str(output), *[str(args.pilot / p) for p in paths[1:]]], check=True, timeout=90)
        cases = json.loads(output.read_text(encoding='utf-8'))
    validate(cases)
    profile = ROOT / 'crates/mercurio-sysml/resources/metamodels/sysml-2.0-pilot-2026-08'
    provenance['resolved_inputs_sha256'] = {p.name: digest(p.read_bytes()) for p in [profile/'feature-redefinitions.extract.json', profile/'featuring-queries.extract.json', profile/'ecore-effective.extract.json']}
    provenance['observation_helpers_sha256'] = {n: digest(HELPER.with_name(n+'.java').read_bytes()) for n in ('PilotCompatibilityExporter','PilotResultConstructionExporter','PilotOperandProbe','PilotOwnerTypingProbe','PilotStepStrategyProbe')}
    provenance['java_sha256'] = digest((args.java_bin / ('java' + suffix)).read_bytes())
    provenance['javac_sha256'] = digest((args.java_bin / ('javac' + suffix)).read_bytes())
    provenance['java_version'] = subprocess.check_output([str(args.java_bin / ('java' + suffix)), '-version'], stderr=subprocess.STDOUT).decode().strip()
    result = {'schema': 'dev.mercurio.fresh-transition-link-controls.v1',
              'scope': 'Fresh fixed ReferenceUsage transition-link lifecycle under supplied complete owner/type/library stages. Variable and nonfresh contexts and actual libraries remain open.',
              'provenance': provenance, **cases}
    text = json.dumps(result, indent=2, sort_keys=True) + '\n'
    if args.check:
        if OUTPUT.read_text(encoding='utf-8') != text:
            raise ValueError('Stale fixed featuring controls')
    else:
        OUTPUT.write_text(text, encoding='utf-8', newline='\n')
    print('Pinned fresh fixed transition-link lifecycle:',len(cases['controls']),'controls')

if __name__=='__main__': main()
