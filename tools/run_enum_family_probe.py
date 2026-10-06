"""Export independent selected-rule controls for all pinned enum declarations."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]
PIN = '692170b71867353b8f90341e61556f49a5beb0e5'
HELPER = ROOT / 'tools/pilot-exporter/src/main/java/dev/mercurio/pilot/PilotEnumFamilyProbe.java'
OUTPUT = ROOT / 'docs/conformance/2026-08-support/enum-family-pilot-controls.json'


def digest(data):
    return hashlib.sha256(data).hexdigest()



def validate(cases):
    imported = json.loads((ROOT / 'crates/mercurio-sysml/resources/metamodels/sysml-2.0-pilot-2026-08/enum-rules.extract.json').read_text(encoding='utf-8'))
    expected = {(r['language'], r['name'], d['token']): d for r in imported['rules'] for d in r['literals']}
    primary_carriers = {'FilterPackageMemberVisibility': 'FilterPackageMember',
        'PortionKind': 'PartUsage', 'TriggerFeatureKind': 'TriggerActionMember',
        'GuardFeatureKind': 'GuardExpressionMember', 'EffectFeatureKind': 'EffectBehaviorMember',
        'RequirementConstraintKind': 'RequirementConstraintMember', 'FramedConcernKind': 'FramedConcernMember',
        'RequirementVerificationKind': 'RequirementVerificationMember', 'ExposeVisibilityKind': 'Expose'}
    expected_models = set()
    for language, rule, token in expected:
        kerml = language == 'org.omg.kerml.xtext.KerML'
        carrier = ('NonFeatureMember' if kerml else 'PackageMember') if rule == 'VisibilityIndicator' else ('Feature' if kerml else 'AttributeUsage') if rule == 'FeatureDirection' else primary_carriers[rule]
        expected_models.add((language, rule, token, carrier))
        if rule == 'VisibilityIndicator': expected_models.add((language, rule, token, 'Import'))
        if rule == 'PortionKind': expected_models.update((language, rule, token, c) for c in ('PortionUsage', 'MergeNode'))
    controls = cases['controls']
    identities = [(c['language'], c['rule'], c['token'], c['polarity']) for c in controls]
    required = {(*key, polarity) for key in expected for polarity in ('positive', 'negative', 'boundary')}
    if len(identities) != len(set(identities)) or set(identities) != required:
        raise ValueError('Incomplete or duplicate enum declaration contexts')
    for c in controls:
        selected = expected[c['language'], c['rule'], c['token']]
        if c['accepted'] != (c['polarity'] == 'positive'):
            raise ValueError('Unexpected enum acceptance')
        if c['accepted'] and (c['literal'], c['value']) != (selected['literal'], selected['value']):
            raise ValueError('Unexpected Ecore enum value')
    models = cases['model_controls']
    identities = [(c['language'], c['rule'], c['token'], c['carrier_rule']) for c in models]
    primary = {(c['language'], c['rule'], c['token']) for c in models if c['primary']}
    if len(models) != 34 or len(identities) != len(set(identities)) or set(identities) != expected_models or primary != set(expected) or sum(c['primary'] for c in models) != 24:
        raise ValueError('Incomplete or duplicate enum caller contexts')
    for c in models:
        selected = expected[c['language'], c['rule'], c['token']]
        if (c['literal'], c['value']) != (selected['literal'], selected['value']):
            raise ValueError('Unexpected caller enum value')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--java-bin', type=Path, required=True)
    parser.add_argument('--pilot', type=Path, default=ROOT.parent / 'target/support-2026-08/pilot-pinned-2026-08')
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    def git(*command, repo=None):
        repo = repo or args.pilot
        # Git 2.36 ignores command-local safe.directory; isolate the exception
        # in an ephemeral global config rather than changing the user's config.
        with tempfile.TemporaryDirectory(prefix='resource-probe-git-') as directory:
            config = Path(directory) / 'gitconfig'
            config.write_text('[safe]\n\tdirectory = ' + repo.resolve().as_posix() + '\n', encoding='utf-8')
            environment = dict(os.environ, GIT_CONFIG_GLOBAL=str(config))
            return subprocess.check_output(['git', '-C', str(repo), *command], env=environment)
    if git('rev-parse', 'HEAD').decode().strip() != PIN:
        raise ValueError('Unexpected Pilot revision')
    paths = ['org.omg.sysml/model/SysML.ecore',
             'org.omg.kerml.xtext/src/org/omg/kerml/xtext/KerML.xtext',
             'org.omg.sysml.xtext/src/org/omg/sysml/xtext/SysML.xtext']
    sources = {}
    for path in paths:
        actual = (args.pilot / path).read_bytes().replace(b'\r\n', b'\n')
        if actual != git('show', PIN + ':' + path).replace(b'\r\n', b'\n'):
            raise ValueError('Changed pinned source: ' + path)
        sources[path] = digest(actual)
    jar = args.pilot / 'org.omg.sysml.interactive/target/org.omg.sysml.interactive-0.62.0-SNAPSHOT-all.jar'
    if digest(jar.read_bytes()) != '4512852638fc799505fe676955e8c78fedef1f3ce44ec90b57b34c05c31e6945':
        raise ValueError('Unexpected Pilot executable jar')
    provenance = {'pilot_commit': PIN, 'source_sha256': sources,
                  'jar_sha256': digest(jar.read_bytes()), 'helper_sha256': digest(HELPER.read_bytes()),
                  'driver_sha256': digest(Path(__file__).read_bytes())}
    suffix = '.exe' if os.name == 'nt' else ''
    with tempfile.TemporaryDirectory(prefix='resource-link-probe-') as directory:
        directory = Path(directory)
        subprocess.run([str(args.java_bin / ('javac' + suffix)), '-encoding', 'UTF-8', '-cp', str(jar), '-d', str(directory), str(HELPER)], check=True, timeout=60)
        output = directory / 'result.json'
        subprocess.run([str(args.java_bin / ('java' + suffix)), '-cp', str(directory) + os.pathsep + str(jar), 'dev.mercurio.pilot.PilotEnumFamilyProbe', str(output)], check=True, timeout=60)
        cases = json.loads(output.read_text(encoding='utf-8'))
    validate(cases)
    provenance['java_sha256'] = digest((args.java_bin / ('java' + suffix)).read_bytes())
    provenance['javac_sha256'] = digest((args.java_bin / ('javac' + suffix)).read_bytes())
    provenance['java_version'] = subprocess.check_output([str(args.java_bin / ('java' + suffix)), '-version'], stderr=subprocess.STDOUT).decode().strip()
    result = {'schema': 'dev.mercurio.enum-family-controls.v1',
              'scope': 'All 24 pinned enum declarations in both language contexts: generated upstream parser rule values, quoted-name rejection and keyword/ID boundaries. Includes 34 raw caller model observations spanning all 18 direct enum call sites. Linking, validation and whole-family qualification remain separate.',
              'provenance': provenance, **cases}
    text = json.dumps(result, indent=2, sort_keys=True) + '\n'
    if args.check:
        if OUTPUT.read_text(encoding='utf-8') != text:
            raise ValueError('Stale enum declaration controls')
    else:
        OUTPUT.write_text(text, encoding='utf-8', newline='\n')
    print('Pinned enum declaration controls exported')


if __name__ == '__main__':
    main()
