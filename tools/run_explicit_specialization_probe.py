"""Export pinned positive explicit ancestry and fixed portion featuring observations."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]
PIN = '692170b71867353b8f90341e61556f49a5beb0e5'
HELPER = ROOT / 'tools/pilot-exporter/src/main/java/dev/mercurio/pilot/PilotExplicitSpecializationProbe.java'
OUTPUT = ROOT / 'docs/conformance/2026-08-support/explicit-specialization-pilot-controls.json'


def digest(data):
    return hashlib.sha256(data).hexdigest()


def validate(cases):
    rows=cases['controls']
    expected={(k,s) for k in ('Feature','ReferenceUsage','ActionUsage','TransitionUsage') for s in ('direct','chain','diamond','cycle')}
    if len(rows)!=16 or {(r['kind'],r['shape']) for r in rows}!=expected:
        raise ValueError('Incomplete explicit proof matrix')
    if any(r['specializes'] is not True or r['complete'] is not False for r in rows):
        raise ValueError('Changed fresh positive proof')
    edges={'direct':[['root','goal']], 'chain':[['root','a'],['a','goal']], 'diamond':[['root','a'],['root','b'],['a','goal'],['b','goal']], 'cycle':[['root','a'],['a','root'],['a','goal']]}
    if any(r['edges']!=edges[r['shape']] for r in rows):
        raise ValueError('Changed explicit ancestry graph')
    featuring=cases['featuring_controls']
    if len(featuring)!=4 or {r['owner_kind'] for r in featuring}!={k for k,_ in expected}:
        raise ValueError('Incomplete fresh owner featuring matrix')
    if any(r['variable'] is not False or r['query']!=['owner'] or r['after']!=r['query'] or r['owner_complete'] is not False or r['candidate_complete'] is not False for r in featuring):
        raise ValueError('Changed fixed portion featuring stage')


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
    paths = ['org.omg.sysml/model/SysML.ecore', *['org.omg.sysml.logic/src/main/java/org/omg/sysml/'+p for p in ['adapter/FeatureAdapter.java', 'adapter/UsageAdapter.java', 'adapter/TransitionUsageAdapter.java', 'adapter/TypeAdapter.java', 'util/TypeUtil.java']]]
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
        subprocess.run([str(args.java_bin / ('javac' + suffix)), '-encoding', 'UTF-8', '-cp', str(jar), '-d', str(directory), str(HELPER)], check=True, timeout=60)
        output = directory / 'result.json'
        subprocess.run([str(args.java_bin / ('java' + suffix)), '-cp', str(directory) + os.pathsep + str(jar), 'dev.mercurio.pilot.PilotExplicitSpecializationProbe', str(jar), str(output), *[str(args.pilot / p) for p in paths[1:]]], check=True, timeout=90)
        cases = json.loads(output.read_text(encoding='utf-8'))
    validate(cases)
    profile = ROOT / 'crates/mercurio-sysml/resources/metamodels/sysml-2.0-pilot-2026-08'
    provenance['resolved_inputs_sha256'] = {p.name: digest(p.read_bytes()) for p in [profile/'feature-redefinitions.extract.json', profile/'featuring-queries.extract.json', profile/'ecore-effective.extract.json']}
    provenance['java_sha256'] = digest((args.java_bin / ('java' + suffix)).read_bytes())
    provenance['javac_sha256'] = digest((args.java_bin / ('javac' + suffix)).read_bytes())
    provenance['java_version'] = subprocess.check_output([str(args.java_bin / ('java' + suffix)), '-version'], stderr=subprocess.STDOUT).decode().strip()
    result = {'schema': 'dev.mercurio.explicit-specialization-controls.v1',
              'scope': 'Fresh positive explicit Subsetting ancestry across four concrete Feature adapters/four graph shapes; four fresh fixed-portion ReferenceUsage featuring contexts under explicitly typed owners. No complete provider, negative Pilot query, full transformation or resource qualification.',
              'provenance': provenance, **cases}
    text = json.dumps(result, indent=2, sort_keys=True) + '\n'
    if args.check:
        if OUTPUT.read_text(encoding='utf-8') != text:
            raise ValueError('Stale explicit specialization controls')
    else:
        OUTPUT.write_text(text, encoding='utf-8', newline='\n')
    print('Pinned positive explicit ancestry:',len(cases['controls']),'controls')


if __name__ == '__main__':
    main()
