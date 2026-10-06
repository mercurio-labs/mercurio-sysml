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
HELPER = ROOT / 'tools/pilot-exporter/src/main/java/dev/mercurio/pilot/PilotUsageVariabilityProbe.java'
OUTPUT = ROOT / 'docs/conformance/2026-08-support/usage-variability-pilot-controls.json'


def digest(data):
    return hashlib.sha256(data).hexdigest()


def validate(cases):
    from generate_fresh_transition_link import generate
    generate({'schema':'dev.mercurio.fresh-transition-link-controls.v1','methods':cases['methods']})
    required={'org.omg.sysml.adapter.UsageAdapter#mayTimeVary','org.omg.sysml.util.UsageUtil#mayTimeVary','org.omg.sysml.delegate.setting.Usage_mayTimeVary_SettingDelegate#basicGet'}
    if set(cases['variability_methods'])!=required:raise ValueError('Missing resolved variability dependencies')
    bindings=cases['bindings'];controls=cases['controls']
    expected={(k,p,m) for k in bindings for p in ['super','first_chain'] for m in ['fixed','variable','portion','composite_action','self_link','happens_link','composite_no_action','action_noncomposite']}
    keys=[(c['kind'],c['placement'],c['shape']) for c in controls]
    if not bindings or len(keys)!=len(expected) or set(keys)!=expected:raise ValueError('Incomplete usage compatibility matrix')
    if set(bindings.values())!={'org.omg.sysml.lang.sysml.impl.UsageImpl#isVariable'}:raise ValueError('Unresolved variability getter override')
    if any(c['is_variable']!=c['may_time_vary'] for c in controls):raise ValueError('Changed derived getter behavior')
    if len(bindings)!=47:raise ValueError('Changed concrete Usage inventory')
    for c in controls:
        if c['shape']=='fixed':
            if c['nodes']['a']!='DataType' or c['library_edges'] or c['may_time_vary']:raise ValueError('Fixed control acquired occurrence variability')
        elif c['nodes']['a']!='Class' or ['Specialization','a','Occurrences::Occurrence'] not in c['library_edges']:raise ValueError('Missing explicit occurrence owner')


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
    paths.append('org.omg.sysml.logic/src/main/java/org/omg/sysml/delegate/setting/Usage_mayTimeVary_SettingDelegate.java')
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
        subprocess.run([str(args.java_bin / ('java' + suffix)), '-cp', str(directory) + os.pathsep + str(jar), 'dev.mercurio.pilot.PilotUsageVariabilityProbe', str(jar), str(output), *[str(args.pilot / p) for p in paths[1:]]], check=True, timeout=90)
        cases = json.loads(output.read_text(encoding='utf-8'))
    validate(cases)
    profile = ROOT / 'crates/mercurio-sysml/resources/metamodels/sysml-2.0-pilot-2026-08'
    provenance['resolved_inputs_sha256'] = {p.name: digest(p.read_bytes()) for p in [profile/'feature-redefinitions.extract.json', profile/'featuring-queries.extract.json', profile/'ecore-effective.extract.json',profile/'compatibility.extract.json']}
    provenance['observation_helpers_sha256'] = {n: digest(HELPER.with_name(n+'.java').read_bytes()) for n in ('PilotCompatibilityExporter','PilotResultConstructionExporter','PilotOperandProbe','PilotOwnerTypingProbe','PilotStepStrategyProbe')}
    provenance['java_sha256'] = digest((args.java_bin / ('java' + suffix)).read_bytes())
    provenance['javac_sha256'] = digest((args.java_bin / ('javac' + suffix)).read_bytes())
    provenance['java_version'] = subprocess.check_output([str(args.java_bin / ('java' + suffix)), '-version'], stderr=subprocess.STDOUT).decode().strip()
    result = {'schema': 'dev.mercurio.usage-variability-controls.v1',
              'scope': 'Bounded Usage variable getter and compatibility branches across every concrete pinned Usage class, eight variability contexts and two call sites. Fixed owner is a DataType; variable, portion and composite-action owners explicitly specialize Occurrences::Occurrence. Preserve explicit and observed owner generalizations separately: completion flags do not establish actual upstream lifecycle completeness. Library targets are supplied dependency fixtures; full native consumption, model validity and actual library lifecycle remain unqualified. Earlier Class-owner observations remain immutable in usage-compatibility-pilot-controls.json.',
              'provenance': provenance, **cases}
    text = json.dumps(result, indent=2, sort_keys=True) + '\n'
    if args.check:
        if OUTPUT.read_text(encoding='utf-8') != text:
            raise ValueError('Stale Usage variability controls')
    else:
        OUTPUT.write_text(text, encoding='utf-8', newline='\n')
    print('Pinned Usage compatibility component:',len(cases['controls']),'controls')

if __name__=='__main__': main()
