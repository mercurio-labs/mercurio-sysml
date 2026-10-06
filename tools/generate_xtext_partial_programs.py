"""Generate isolated experimental expression programs with explicit failure sites.

The finite-graph artifact supplies an opt-in candidate document path; it does
not replace the stable parser or increase admitted rule coverage. Structural/type/progress/provenance failures still abort generation.
Only unresolved prediction sites become mandatory runtime errors.
"""
import argparse
import hashlib
import json
from generate_xtext_fragment_programs import GRAMMAR, ECORE, SEMANTICS, PREDICTIONS, PROFILE, EXPRESSIONS, LANGUAGES, build

OUTPUT = PROFILE / 'xtext-partial-programs.json'

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--check', action='store_true')
    parser.add_argument('--finite-graph', action='store_true', help='Generate separate experimental programs with pre-analysis finite graph binding')
    args = parser.parse_args()
    graph_path = PROFILE / 'xtext-prediction-nfa.experimental.json'
    graph = json.loads(graph_path.read_text(encoding='utf-8')) if args.finite_graph else None
    output = PROFILE / 'xtext-finite-programs.experimental.json' if args.finite_graph else OUTPUT
    grammar, ecore, semantics = (json.loads(p.read_text(encoding='utf-8')) for p in [GRAMMAR, ECORE, SEMANTICS])
    result = {'schema': 'dev.mercurio.xtext-partial-programs.v1',
              'scope': ('Opt-in candidate document execution only; stable parser unchanged. No complete-rule qualification. Every unresolved prediction site must raise an error; no legacy-parser fallback.' if graph is not None else 'Experimental execution only. No production integration or complete-rule qualification. Every unresolved prediction site must raise an error; no legacy-parser fallback.'),
              'rules': {}, 'language_programs': {
                  language: build(grammar, ecore, semantics, roots=[EXPRESSIONS + 'OwnedExpression'] + ([context + '::RootNamespace'] if graph is not None else []),
                                  context=context, partial_predictions=True, experimental_graph=graph)
                  for language, context in LANGUAGES.items()},
              'source_sha256': {p.name: hashlib.sha256(p.read_bytes()).hexdigest() for p in [GRAMMAR, ECORE, SEMANTICS, PREDICTIONS]}}
    if graph is not None:
        result['source_sha256'][graph_path.name] = hashlib.sha256(graph_path.read_bytes()).hexdigest()
    text = json.dumps(result, indent=2) + '\n'
    if args.check:
        if output.read_text(encoding='utf-8') != text:
            raise ValueError('Stale experimental Xtext programs')
    else:
        output.write_text(text, encoding='utf-8', newline='\n')
    for language, program in result['language_programs'].items():
        print(language, len(program['rules']), 'emitted rules;', len(program['unsupported_prediction_sites']), 'explicit failure sites; NOT complete-rule coverage')

if __name__ == '__main__':
    main()
