"""Export bounded pinned Pilot Step strategy observations; no validation."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]
PIN = '692170b71867353b8f90341e61556f49a5beb0e5'
HELPER = ROOT / 'tools/pilot-exporter/src/main/java/dev/mercurio/pilot/PilotMultiplicityDefaultsProbe.java'
OUTPUT = ROOT / 'docs/conformance/2026-08-support/multiplicity-defaults-pilot-controls.json'


def digest(data):
    return hashlib.sha256(data).hexdigest()


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
    paths = ['org.omg.sysml/model/SysML.ecore','org.omg.sysml.logic/src/main/java/org/omg/sysml/adapter/MultiplicityAdapter.java']
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
        subprocess.run([str(args.java_bin / ('javac' + suffix)), '-encoding', 'UTF-8', '-cp', str(jar), '-d', str(directory), str(HELPER),str(HELPER.with_name("PilotExpressionRedefinitionProbe.java"))], check=True, timeout=60)
        output = directory / 'result.json'
        subprocess.run([str(args.java_bin / ('java' + suffix)), '-cp', str(directory) + os.pathsep + str(jar), 'dev.mercurio.pilot.PilotMultiplicityDefaultsProbe', str(jar), str(output), str(args.pilot / paths[1])], check=True, timeout=60)
        cases = json.loads(output.read_text(encoding='utf-8'))
    if set(cases['bindings']) != {'Multiplicity','MultiplicityRange'} or len(cases['controls']) != 20 or len(cases['methods']) != 2:
        raise ValueError('Incomplete fixed multiplicity defaults matrix')
    provenance['tree_helper_sha256']=digest(HELPER.with_name('PilotExpressionRedefinitionProbe.java').read_bytes())
    provenance['java_sha256'] = digest((args.java_bin / ('java' + suffix)).read_bytes())
    provenance['javac_sha256'] = digest((args.java_bin / ('javac' + suffix)).read_bytes())
    provenance['java_version'] = subprocess.check_output([str(args.java_bin / ('java' + suffix)), '-version'], stderr=subprocess.STDOUT).decode().strip()
    release=ROOT.parent/'target/upstream/SysML-v2-Release'
    release_pin='fb97b754f29588b8e9c7a35f370880cd15eb29e7'
    if git('rev-parse','HEAD',repo=release).decode().strip()!=release_pin:raise ValueError('Changed release revision')
    libraries=[]
    for path in ['sysml.library/Kernel Libraries/Kernel Semantic Library/Base.kerml','sysml.library/Kernel Libraries/Kernel Data Type Library/ScalarValues.kerml']:
        raw=(release/path).read_bytes()
        if raw.replace(b'\r\n',b'\n')!=git('show',release_pin+':'+path,repo=release).replace(b'\r\n',b'\n'):raise ValueError('Changed release source')
        libraries.append({'uri':path,'text':raw.decode('utf-8'),'sha256':digest(raw)})
    cases['library_sources']=libraries
    provenance['release_commit']=release_pin
    result = {'schema': 'dev.mercurio.multiplicity-defaults-controls.v1',
              'scope': 'Resolved owner-sensitive Multiplicity defaults, two bindings, five owner categories and two membership kinds; selector and empty relevant-feature query only. No complete transformation or validation.',
              'provenance': provenance, **cases}
    text = json.dumps(result, indent=2, sort_keys=True) + '\n'
    if args.check:
        if OUTPUT.read_text(encoding='utf-8') != text:
            raise ValueError('Stale expression redefinition controls')
    else:
        OUTPUT.write_text(text, encoding='utf-8', newline='\n')
    print('Pinned Step strategy controls exported')


if __name__ == '__main__':
    main()
