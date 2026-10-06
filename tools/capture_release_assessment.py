"""Freeze or verify exact candidate sources and binaries for a release assessment."""
import argparse, hashlib, json, os, platform, shutil, subprocess
from pathlib import Path


def digest(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def command(*args):
    return subprocess.check_output(args, stderr=subprocess.STDOUT, text=True).strip()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--root', type=Path, required=True)
    parser.add_argument('--verify', action='store_true')
    args = parser.parse_args()
    path = args.root / 'build-provenance.json'
    if args.verify:
        saved = json.loads(path.read_text(encoding='utf-8'))
        for category in ['source_files', 'binaries', 'assessment_scripts_sha256']:
            for name, expected in saved[category].items():
                if digest(name) != expected:
                    raise ValueError('Candidate changed: ' + name)
        if digest('tools/pilot-exporter/src/main/java/dev/mercurio/pilot/PilotModelExporter.java') != saved['java_source_sha256']:
            raise ValueError('Oracle source changed')
        print('Candidate sources, binaries, oracle and assessment scripts verified')
        return
    if path.exists():
        raise ValueError('Refusing to overwrite frozen provenance; use --verify or a new directory')
    files = {}
    for base in [Path('.'), Path('../mercurio-foundation')]:
        for name in command('git', '-C', str(base), 'ls-files', '--cached', '--others', '--exclude-standard').splitlines():
            item = base / name
            if item.is_file() and ((name.startswith('crates/') and item.suffix in ['.rs', '.toml', '.json', '.sysml', '.kerml', '.rhai', '.yaml', '.yml', '.txt']) or (name.startswith('tools/') and item.suffix in ['.py', '.java']) or name in ['Cargo.toml', 'Cargo.lock', '.cargo/config.toml']):
                files[item.as_posix()] = digest(item)
    repositories = {name: {'path': str(base.resolve()), 'head': command('git', '-C', str(base), 'rev-parse', 'HEAD'), 'branch': command('git', '-C', str(base), 'branch', '--show-current')} for name,base in [('outer',Path('..')),('sysml',Path('.')),('foundation',Path('../mercurio-foundation'))]}
    binaries = {str(Path('target/release')/(name+'.exe')): digest(Path('target/release')/(name+'.exe')) for name in ['audit_release_compile','compare_pilot_semantics','import_pilot_stdlib']}
    scripts = {str(p): digest(p) for p in Path('tools').glob('*release*assessment.py')}
    scripts['tools/release_expression_projection.py'] = digest('tools/release_expression_projection.py')
    upstream = {}
    for name in ['SysML-v2-Pilot-Implementation','SysML-v2-Release']:
        base = str(Path('../target/upstream')/name)
        upstream[name] = {'commit': command('git','-C',base,'rev-parse','HEAD'), 'tag':command('git','-C',base,'describe','--tags','--exact-match'), 'clean': not command('git','-C',base,'status','--porcelain')}
        if not upstream[name]['clean']:
            raise ValueError('Dirty upstream input: '+name)
    result = {'toolchains':{'rustc':command('rustc','--version'),'cargo':command('cargo','--version'),'java':command('java','-version')}, 'source_files':files, 'source_fingerprint':hashlib.sha256(json.dumps(files,sort_keys=True).encode()).hexdigest(), 'repositories':repositories, 'upstream_repositories':upstream, 'binaries':binaries, 'assessment_scripts_sha256':scripts, 'java_source_sha256':digest('tools/pilot-exporter/src/main/java/dev/mercurio/pilot/PilotModelExporter.java'), 'native_profile':'release, optimized', 'pilot_heap':'-Xmx3g', 'benchmark':'fresh processes, 3 repetitions, alternating engine order; export/build excluded; warm uncontrolled OS cache', 'platform':platform.platform(), 'logical_cpus':os.cpu_count(), 'semantic_export':'CheckMode.ALL with source index disabled; ordered reference export enabled', 'pilot_benchmark_index':'published source .index.json enabled', 'qualification':'open; native validation and implicit semantics incomplete'}
    snapshot = (args.root / 'candidate-snapshot').resolve()
    if os.name == 'nt':
        snapshot = Path('\\\\?\\' + str(snapshot))
    for name, expected in {**files, **binaries}.items():
        original = Path(name)
        destination = snapshot / original.resolve().relative_to(Path('..').resolve())
        destination.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(original, destination)
        if digest(destination) != expected:
            raise ValueError('Snapshot changed while copying: '+name)
    result['snapshot'] = {'path':'candidate-snapshot', 'paths_relative_to':'outer worktree', 'verified':True}
    path.write_text(json.dumps(result,indent=2)+'\n',encoding='utf-8')
    print('Frozen',len(files),'source files and',len(binaries),'binaries')


if __name__ == '__main__':
    main()
