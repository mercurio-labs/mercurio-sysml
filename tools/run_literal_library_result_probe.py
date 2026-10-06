"""Export bounded literal result queries against actual pinned library resources; no validation."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]
PIN = '692170b71867353b8f90341e61556f49a5beb0e5'
HELPER = ROOT / 'tools/pilot-exporter/src/main/java/dev/mercurio/pilot/PilotLiteralLibraryResultProbe.java'
OUTPUT = ROOT / 'docs/conformance/2026-08-support/literal-library-result-pilot-controls.json'


def digest(data):
    return hashlib.sha256(data).hexdigest()


def validate_observations(cases):
    controls=cases.get('controls',[])
    if len(controls)!=8 or cases.get('resource_errors') or cases.get('provider')!='org.omg.kerml.xtext.library.KerMLLibraryProvider':
        raise ValueError('Incomplete/failed upstream literal result environment')
    expected=[('LiteralInteger','integerValue','LiteralIntegerEvaluation'),('LiteralRational','rationalValue','LiteralRationalEvaluation'),('LiteralBoolean','booleanValue','LiteralBooleanEvaluation'),('LiteralString','stringValue','LiteralStringEvaluation'),('LiteralInfinity','infinityValue','LiteralIntegerEvaluation'),('NullExpression','nullValue','NullEvaluation'),('LiteralInteger','bounds','LiteralIntegerEvaluation'),('LiteralInteger','bounds','LiteralIntegerEvaluation')]
    for case,(kind,owner,result) in zip(controls,expected):
        if (case.get('kind'),case.get('owner_path'),case.get('result_path'))!=(kind,['LiteralProbe',owner],['Performances',result]):
            raise ValueError('Changed literal result identity')
        if not case.get('result_present') or case.get('result_kind')!='Feature' or case.get('result_membership_kind')!='ReturnParameterMembership' or case.get('result_direction')!='out':
            raise ValueError('Changed inherited result ownership/direction')
        source=case.get('result_source_uri','')
        if not source.endswith('/Performances.kerml') or not case.get('result_uri','').startswith(source+'#'):
            raise ValueError('Result is not contained in the actual Performances resource')


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
    paths = ['org.omg.sysml/model/SysML.ecore','org.omg.sysml.logic/src/main/java/org/omg/sysml/util/TypeUtil.java','org.omg.sysml.logic/src/main/java/org/omg/sysml/adapter/ExpressionAdapter.java','org.omg.kerml.xtext/src/org/omg/kerml/xtext/library/KerMLLibraryProvider.xtend']
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
        subprocess.run([str(args.java_bin / ('java' + suffix)), '-cp', str(directory) + os.pathsep + str(jar), 'dev.mercurio.pilot.PilotLiteralLibraryResultProbe', str(ROOT / 'docs/conformance/2026-08-support/definition-pipeline-evidence/performances-import-source-inputs.json'), str(output)], check=True, timeout=120)
        cases = json.loads(output.read_text(encoding='utf-8'))
    validate_observations(cases)
    provenance['java_sha256'] = digest((args.java_bin / ('java' + suffix)).read_bytes())
    provenance['javac_sha256'] = digest((args.java_bin / ('javac' + suffix)).read_bytes())
    provenance['java_version'] = subprocess.check_output([str(args.java_bin / ('java' + suffix)), '-version'], stderr=subprocess.STDOUT).decode().strip()
    release=ROOT.parent/'target/upstream/SysML-v2-Release'
    release_pin='fb97b754f29588b8e9c7a35f370880cd15eb29e7'
    if git('rev-parse','HEAD',repo=release).decode().strip()!=release_pin:raise ValueError('Changed release revision')
    manifest=ROOT/'docs/conformance/2026-08-support/definition-pipeline-evidence/performances-import-source-inputs.json'
    library_inputs={}
    for filename in json.loads(manifest.read_text(encoding='utf-8'))['cases'][0]['input_files']:
        path=Path(filename);relative=path.relative_to(release).as_posix();raw=path.read_bytes()
        if raw.replace(b'\r\n',b'\n')!=git('show',release_pin+':'+relative,repo=release).replace(b'\r\n',b'\n'):raise ValueError('Changed pinned library')
        library_inputs[relative]=digest(raw)
    if len(library_inputs)!=16:raise ValueError('Changed fixed resource cohort')
    provenance['release_commit']=release_pin
    provenance['libraries']=library_inputs
    result = {'schema': 'dev.mercurio.literal-library-result-controls.v1',
              'scope': 'Eight literal/null expression result observations against sixteen actual pinned library sources, using upstream KerMLLibraryProvider. No validation or whole-resource qualification.',
              'provenance': provenance, **cases}
    text = json.dumps(result, indent=2, sort_keys=True) + '\n'
    if args.check:
        if OUTPUT.read_text(encoding='utf-8') != text:
            raise ValueError('Stale literal library controls')
    else:
        OUTPUT.write_text(text, encoding='utf-8', newline='\n')
    print('Pinned literal library results exported')


if __name__ == '__main__':
    main()
