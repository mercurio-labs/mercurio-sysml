"""Cache pinned passive-annotation endpoint observations for crossing readiness; no qualification."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]
PIN = '692170b71867353b8f90341e61556f49a5beb0e5'
HELPER = ROOT / 'tools/pilot-exporter/src/main/java/dev/mercurio/pilot/PilotCrossingAnnotationProbe.java'
OUTPUT = ROOT / 'docs/conformance/2026-08-support/crossing-annotation-pilot-controls.json'


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
    paths = [
        'org.omg.sysml/model/SysML.ecore',
        'org.omg.sysml.logic/src/main/java/org/omg/sysml/util/ElementUtil.java',
        'org.omg.sysml.logic/src/main/java/org/omg/sysml/util/NamespaceUtil.java',
        'org.omg.sysml.logic/src/main/java/org/omg/sysml/adapter/TypeAdapter.java',
        *['org.omg.sysml.logic/src/main/java/org/omg/sysml/delegate/setting/' + name + '_SettingDelegate.java'
          for name in ['Element_ownedAnnotation','Annotation_annotatingElement','Annotation_ownedAnnotatingElement','Annotation_owningAnnotatingElement','Namespace_ownedMember']],
    ]
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
        subprocess.run([str(args.java_bin / ('java' + suffix)), '-cp', str(directory) + os.pathsep + str(jar), 'dev.mercurio.pilot.PilotCrossingAnnotationProbe', str(output)], check=True, timeout=60)
        cases = json.loads(output.read_text(encoding='utf-8'))
    expected = {(kind, ownership) for kind in ("Documentation", "Comment", "TextualRepresentation", "MetadataFeature", "absent") for ownership in ("owned", "owning")}
    if len(cases) != 10 or {(r["kind"], r["ownership"]) for r in cases} != expected:
        raise ValueError("Incomplete annotation endpoint matrix")
    for row in cases:
        kind, ownership = row["kind"], row["ownership"]
        if row["annotating_kind"] != (None if kind == "absent" else kind):
            raise ValueError("Unexpected independent annotation endpoint")
        expected_metadata = ["endpoint"] if kind == "MetadataFeature" and ownership == "owned" else []
        if row["metadata"] != expected_metadata:
            raise ValueError("Unexpected independent metadata selection")
    provenance['java_sha256'] = digest((args.java_bin / ('java' + suffix)).read_bytes())
    provenance['javac_sha256'] = digest((args.java_bin / ('javac' + suffix)).read_bytes())
    provenance['java_version'] = subprocess.check_output([str(args.java_bin / ('java' + suffix)), '-version'], stderr=subprocess.STDOUT).decode().strip()
    result = {'schema': 'dev.mercurio.crossing-annotation-controls.v1',
              'scope': 'Ten independent factory controls verify three passive annotating kinds, MetadataFeature and absent endpoints, with owned/owning annotation forms. These are endpoint/selection observations, not transformed crossing observations or semantic metadata evaluation. Composition with existing binary crossing controls is verified separately in Rust.',
              'provenance': provenance, 'cases': cases}
    text = json.dumps(result, indent=2, sort_keys=True) + '\n'
    if args.check:
        if OUTPUT.read_text(encoding='utf-8') != text:
            raise ValueError('Stale metadata selection controls')
    else:
        OUTPUT.write_text(text, encoding='utf-8', newline='\n')
    print('Pinned annotation endpoints: ten kind/ownership controls')


if __name__ == '__main__':
    main()
