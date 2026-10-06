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
HELPER = ROOT / 'tools/pilot-exporter/src/main/java/dev/mercurio/pilot/PilotExpressionModelProbe.java'
OUTPUT = ROOT / 'docs/conformance/2026-08-support/expression-model-pilot-controls.json'


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
        'org.omg.kerml.xtext/src/org/omg/kerml/xtext/conversion/NonuniqueValueConverter.xtend',
        'org.omg.kerml.expressions.xtext/src/org/omg/kerml/expressions/xtext/KerMLExpressions.xtext',
        'org.omg.kerml.xtext/src/org/omg/kerml/xtext/KerML.xtext',
        'org.omg.sysml.xtext/src/org/omg/sysml/xtext/SysML.xtext',
        'org.omg.sysml.model/src/main/java/org/omg/sysml/lang/sysml/impl/FeatureChainExpressionImpl.java',
        'org.omg.sysml.model/src/main/java/org/omg/sysml/lang/sysml/impl/IndexExpressionImpl.java',
        'org.omg.sysml.logic/src/main/java/org/omg/sysml/util/NamespaceUtil.java',
        'org.omg.sysml.model/src/main/java/org/omg/sysml/lang/sysml/impl/SpecializationImpl.java',
        'org.omg.sysml.model/src/main/java/org/omg/sysml/lang/sysml/impl/SubclassificationImpl.java',
        'org.omg.sysml.model/src/main/java/org/omg/sysml/lang/sysml/impl/FeatureTypingImpl.java',
        'org.omg.sysml.model/src/main/java/org/omg/sysml/lang/sysml/impl/SubsettingImpl.java',
        'org.omg.sysml.model/src/main/java/org/omg/sysml/lang/sysml/impl/RedefinitionImpl.java',
        'org.omg.sysml.logic/src/main/java/org/omg/sysml/util/TypeUtil.java',
        'org.omg.sysml.logic/src/main/java/org/omg/sysml/util/FeatureUtil.java',
        'org.omg.sysml.logic/src/main/java/org/omg/sysml/adapter/NamespaceAdapter.java',
        'org.omg.sysml.logic/src/main/java/org/omg/sysml/adapter/TypeAdapter.java',
        'org.omg.sysml.logic/src/main/java/org/omg/sysml/adapter/ClassifierAdapter.java',
        'org.omg.sysml.logic/src/main/java/org/omg/sysml/adapter/ClassAdapter.java',
        'org.omg.sysml.logic/src/main/java/org/omg/sysml/adapter/BehaviorAdapter.java',
        'org.omg.sysml.logic/src/main/java/org/omg/sysml/delegate/setting/Type_ownedFeature_SettingDelegate.java',
        'org.omg.sysml.logic/src/main/java/org/omg/sysml/delegate/setting/FeatureMembership_ownedMemberFeature_SettingDelegate.java',
        'org.omg.sysml.logic/src/main/java/org/omg/sysml/delegate/setting/OwningMembership_ownedMemberElement_SettingDelegate.java',
        'org.omg.sysml.logic/src/main/java/org/omg/sysml/delegate/setting/Element_owningMembership_SettingDelegate.java',
        'org.omg.sysml.logic/src/main/java/org/omg/sysml/delegate/setting/Element_owningNamespace_SettingDelegate.java',
        'org.omg.sysml.logic/src/main/java/org/omg/sysml/delegate/setting/Membership_membershipOwningNamespace_SettingDelegate.java',
        'org.omg.sysml.logic/src/main/java/org/omg/sysml/delegate/setting/Relationship_owningRelatedElement_SettingDelegate.java',
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
    with tempfile.TemporaryDirectory(prefix='expression-model-probe-') as directory:
        directory = Path(directory)
        subprocess.run([str(args.java_bin / ('javac' + suffix)), '-encoding', 'UTF-8', '-cp',
                        str(jar), '-d', str(directory), str(HELPER)], check=True, timeout=60)
        output = directory / 'result.json'
        subprocess.run([str(args.java_bin / ('java' + suffix)), '-cp', str(directory) + os.pathsep + str(jar),
                        'dev.mercurio.pilot.PilotExpressionModelProbe', str(output)], check=True, timeout=60)
        cases = json.loads(output.read_text(encoding='utf-8'))
        owned_results = json.loads(Path(str(output) + '.results.json').read_text(encoding='utf-8'))
        inherited_results = json.loads(Path(str(output) + '.inherited.json').read_text(encoding='utf-8'))
        aliases = json.loads(Path(str(output) + '.aliases.json').read_text(encoding='utf-8'))
        specializations = json.loads(Path(str(output) + '.specializations.json').read_text(encoding='utf-8'))
        clauses = json.loads(Path(str(output) + '.clauses.json').read_text(encoding='utf-8'))
        declarations = json.loads(Path(str(output) + '.declarations.json').read_text(encoding='utf-8'))
        types = json.loads(Path(str(output) + '.types.json').read_text(encoding='utf-8'))
        definitions = json.loads(Path(str(output) + '.definitions.json').read_text(encoding='utf-8'))
        if len(definitions) != 22:
            raise ValueError('Incomplete SysML definition controls')
        if len(types) != 32:
            raise ValueError('Incomplete type declaration controls')
        if len(declarations) != 40:
            raise ValueError('Incomplete feature declaration controls')
        if len(clauses) != 28:
            raise ValueError('Incomplete specialization clause controls')
        if len(specializations) != 90:
            raise ValueError('Incomplete specialization controls')
        if len(aliases) != 10:
            raise ValueError('Incomplete reflective setter controls')
        if len(inherited_results) != 32:
            raise ValueError('Incomplete inherited result controls')
        if len(owned_results) != 60:
            raise ValueError('Incomplete owned result controls')
    if len(cases) != 92 or {case['language'] for case in cases} != {'kerml', 'sysml'}:
        raise ValueError('Incomplete expression-model probe')
    if provenance['jar_sha256'] != digest(jar.read_bytes()) or provenance['helper_sha256'] != digest(HELPER.read_bytes()):
        raise ValueError('Changed probe inputs')
    result = {'scope': 'Raw OwnedExpression arithmetic, control and reference construction trees in both pinned languages, including operand-delegate wrappers, inverse checks and attached non-expression namespace identities. Separate owned-result controls check current-graph parameter selection. Separate inherited-result controls use explicit Behavior graphs with ordered cyclic generalization and verified prerequisite outputs. No linking, automatic member generation, implicit generalization qualification, derived arguments, specification conformance or full sample qualification.',
              'provenance': provenance, 'cases': cases, 'owned_result_controls': owned_results, 'inherited_result_controls': inherited_results, 'reference_alias_controls': aliases, 'specialization_controls': specializations, 'specialization_clause_controls': clauses, 'feature_declaration_controls': declarations, 'type_declaration_controls': types, 'definition_controls': definitions}
    text = json.dumps(result, indent=2) + '\n'
    if args.check:
        if OUTPUT.read_text(encoding='utf-8') != text:
            raise ValueError('Stale expression-model evidence')
    else:
        OUTPUT.write_text(text, encoding='utf-8', newline='\n')
    print('Pinned expression models verified: 92 construction/acceptance cases and 60 owned/32 inherited-result/10 reference-setter controls; 90 owned-specialization and 28 enclosing-clause and 40 feature/32 type/22 SysML definition controls')


if __name__ == '__main__':
    main()
