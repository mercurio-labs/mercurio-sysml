"""Export bounded, signature-checked ANTLR decisions using the pinned build-time frontend."""
import argparse
import hashlib
import json
import os
from pathlib import Path
from generate_xtext_assignment_contracts import GRAMMAR
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]
PIN = '692170b71867353b8f90341e61556f49a5beb0e5'
HELPER = ROOT / 'tools/pilot-exporter/src/main/java/dev/mercurio/pilot/PilotPredictionExporter.java'
OUTPUT = ROOT / 'crates/mercurio-sysml/resources/metamodels/sysml-2.0-pilot-2026-08/xtext-prediction.extract.json'
GRAMMARS = {
    'org.omg.kerml.expressions.xtext.KerMLExpressions': ('org.omg.kerml.expressions.xtext', 'org/omg/kerml/expressions/xtext', 'KerMLExpressions'),
    'org.omg.kerml.xtext.KerML': ('org.omg.kerml.xtext', 'org/omg/kerml/xtext', 'KerML'),
    'org.omg.sysml.xtext.SysML': ('org.omg.sysml.xtext', 'org/omg/sysml/xtext', 'SysML'),
}


def digest(data):
    return hashlib.sha256(data).hexdigest()



class PredicateContext(dict):
    """Resolved first sets plus Xtext-provided predicate-hoisting targets."""
    def __init__(self, structured, context):
        metadata = structured['resolution_contexts'][context]
        super().__init__(metadata.get('first_set_predicates', {}))
        nodes = {}
        def index(value):
            if isinstance(value, dict):
                if 'id' in value and 'kind' in value:
                    nodes[value['id']] = value
                for child in value.values(): index(child)
            elif isinstance(value, list):
                for child in value: index(child)
        index(structured['grammars'])
        self.hoisted = {}
        for identity, target in metadata.get('hoisted_predicates', {}).items():
            if identity not in nodes or target not in nodes:
                raise ValueError('Unresolved hoisted predicate identity')
            self.hoisted[identity] = nodes[target]


def first_set_signature(identity, first_sets):
    symbols = (first_sets or {}).get(identity)
    if not symbols:
        raise ValueError('Missing resolved first-set predicate: ' + identity)
    if len(symbols) == 1:
        return symbols
    result = ['choice:begin']
    for index, symbol in enumerate(symbols):
        if index:
            result.append('choice:or')
        result.append(symbol)
    return result + ['choice:end']


def predicate_signature(node, first_sets):
    return (first_set_signature(node['id'], first_sets) if node['fields'].get('firstSetPredicated')
            else source_signature_body(node, first_sets))


def source_signature(node, first_sets=None):
    result = source_occurrence_signature(node, first_sets)
    cardinality = node['fields'].get('cardinality')
    return ['repeat:' + cardinality, *result, 'repeat:end'] if cardinality else result


def source_occurrence_signature(node, first_sets=None):
    result = source_signature_body(node, first_sets)
    if node['fields'].get('firstSetPredicated'):
        guard = first_set_signature(node['id'], first_sets)
        return ['predicate:begin', *guard, 'predicate:end', *result]
    hoisted = getattr(first_sets, 'hoisted', {}).get(node['id'])
    if hoisted is not None:
        guard = source_signature_body(hoisted, first_sets)
        return ['predicate:begin', *guard, 'predicate:end', *result]
    if node['fields'].get('predicated'):
        result = ['predicate:begin', *result, 'predicate:end', *result]
    return result


def source_signature_body(node, first_sets=None):
    kind, fields = node['kind'], node['fields']
    if kind == 'RuleCall':
        return ['call:' + fields['rule']['$ref'].split('::')[1]]
    if kind == 'Keyword':
        return ['keyword:' + fields['value']]
    if kind == 'Action':
        return []
    if kind in ['Assignment', 'CrossReference']:
        return source_signature(fields['terminal'], first_sets)
    if kind == 'Alternatives':
        result = ['choice:begin']
        for index, child in enumerate(fields['elements']):
            if index:
                result.append('choice:or')
            result.extend(source_signature(child, first_sets))
        return result + ['choice:end']
    if kind == 'Group':
        return [symbol for child in fields['elements'] for symbol in source_signature(child, first_sets)]
    raise ValueError('Unsupported source decision signature: ' + node['id'])


def decision_nodes(value, include_repetitions=False):
    if isinstance(value, dict):
        kind = value.get('kind')
        if kind == 'Alternatives' or (kind in ['Group', 'RuleCall', 'Assignment', 'CrossReference', 'Keyword']
                                      and value['fields'].get('cardinality') in (['?', '*', '+'] if include_repetitions else ['?'])):
            yield value
        for child in value.values():
            yield from decision_nodes(child, include_repetitions)
    elif isinstance(value, list):
        for child in value:
            yield from decision_nodes(child, include_repetitions)


def decision_signature(node, first_sets=None):
    if node['kind'] == 'Alternatives':
        return [source_signature(child, first_sets) for child in node['fields']['elements']]
    return [source_occurrence_signature(node, first_sets)]


def guarded_nodes(value, first_sets=None):
    if isinstance(value, dict):
        if value.get('fields', {}).get('predicated') or value.get('fields', {}).get('firstSetPredicated'):
            yield value
        hoisted = getattr(first_sets, 'hoisted', {}).get(value.get('id'))
        if hoisted is not None:
            yield hoisted
        for child in value.values():
            yield from guarded_nodes(child, first_sets)
    elif isinstance(value, list):
        for child in value:
            yield from guarded_nodes(child, first_sets)


def bind_source_nodes(contexts, structured):
    """Resolve exported choices and entry gates to unique Xtext identities."""
    rules = {r['id']: r for g in structured['grammars'] for r in g['rules']}
    for context, decisions in contexts.items():
        effective = structured['resolution_contexts'][context]['effective_rules']
        first_sets = PredicateContext(structured, context)
        requested, _ = decision_requests(structured, context)
        identities = {row['key']: row['source_id'] for row in requested}
        for name, decision in decisions.items():
            if name == 'keywords':
                continue
            candidates = []
            for node in decision_nodes(rules[effective[name.split('#')[0]]]['fields']['alternatives']):
                if (node['kind'] != 'Alternatives') != (decision.get('mode') == 'entry'):
                    continue
                try:
                    signature = decision_signature(node, first_sets)
                except ValueError:
                    continue
                if signature == decision['alternatives']:
                    candidates.append(node)
            binding = decision.get('contextual_binding')
            if binding is not None:
                rule_body = rules[effective[name.split('#')[0]]]['fields']['alternatives']
                if (source_signature(rule_body, first_sets) != binding['rule_signature']
                        or len(candidates) != binding['occurrences']
                        or not 0 <= binding['occurrence'] < len(candidates)):
                    raise ValueError('Changed contextual source decision identity: ' + context + '::' + name)
                candidates = [candidates[binding['occurrence']]]
            if len(candidates) != 1:
                raise ValueError('Missing or ambiguous source decision identity: ' + context + '::' + name)
            node = candidates[0]
            if identities.get(name) != node['id']:
                raise ValueError('Changed contextual source decision identity: ' + context + '::' + name)
            if 'exit_alternative' in decision and decision['exit_cardinality'] != node['fields'].get('cardinality'):
                raise ValueError('Source decision cardinality mismatch: ' + node['id'])
            decision['source_id'] = node['id']
            rule_body = rules[effective[name.split('#')[0]]]['fields']['alternatives']
            guards = list({guard['id']: guard for guard in guarded_nodes(rule_body, first_sets)}.values())
            for state in decision['states']:
                conditions = ([state['gate']] if 'gate' in state else [])
                conditions += [edge['condition'] for edge in state.get('predicate_edges', [])]
                for condition in conditions:
                    if condition['kind'] == 'always':
                        continue
                    matches = [guard for guard in guards if predicate_signature(guard, first_sets) == condition['signature']]
                    if len(matches) != 1:
                        raise ValueError('Missing or ambiguous guarded source identity: ' + node['id'])
                    condition['source_id'] = matches[0]['id']
                    if matches[0]['fields'].get('firstSetPredicated'):
                        condition['first_set'] = True


def decision_requests(structured, context, include_repetitions=False):
    rules = {r['id']: r for g in structured['grammars'] for r in g['rules']}
    requests, unsupported = [], {}
    first_sets = PredicateContext(structured, context)
    for name, identity in structured['resolution_contexts'][context]['effective_rules'].items():
        rule = rules[identity]
        if rule['kind'] != 'ParserRule':
            continue
        nodes = list(decision_nodes(rule['fields']['alternatives'], include_repetitions))
        choice_count = sum(node['kind'] == 'Alternatives' for node in nodes)
        for node in nodes:
            try:
                signature = decision_signature(node, first_sets)
            except ValueError as error:
                unsupported[node['id']] = str(error)
                continue
            mode = 'choice' if node['kind'] == 'Alternatives' else 'entry'
            if mode == 'choice':
                key = name if choice_count == 1 or node['id'] == identity + '/alternatives' else name + '#' + node['id'].split('/alternatives', 1)[1]
            else:
                key = name + '#entry' + node['id'].split('/alternatives', 1)[1]
            requests.append({'key': key, 'rule': name, 'source_id': node['id'], 'mode': mode,
                             'cardinality': node['fields'].get('cardinality') or '',
                             'alternatives': signature})
        local = [request for request in requests if request['rule'] == name]
        for request in local:
            identical = [r for r in local if r['mode'] == request['mode'] and r['alternatives'] == request['alternatives']]
            if len(identical) > 1:
                request['occurrence'] = identical.index(request)
                request['occurrences'] = len(identical)
                request['rule_signature'] = source_signature(rule['fields']['alternatives'], first_sets)
    return sorted(requests, key=lambda r: r['key']), unsupported



def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--java-bin', type=Path, required=True)
    parser.add_argument('--pilot', type=Path, default=ROOT.parent / 'target/support-2026-08/pilot-pinned-2026-08')
    parser.add_argument('--check', action='store_true')
    parser.add_argument('--output', type=Path, default=OUTPUT)
    parser.add_argument('--state-limit', type=int, default=10000)
    parser.add_argument('--configuration-limit', type=int, default=10000000)
    parser.add_argument('--analysis-timeout', type=int, default=60, help='Whole-context abort timeout; never enables partial output or timed prediction fallback')
    args = parser.parse_args()
    if args.state_limit <= 0 or args.configuration_limit <= 0 or args.analysis_timeout <= 0:
        parser.error('Analysis limits must be positive')
    def git(*command):
        return subprocess.check_output(['git', '-C', str(args.pilot), *command])
    if git('rev-parse', 'HEAD').decode().strip() != PIN:
        raise ValueError('Unexpected Pilot revision')
    sources, grammars = {}, {}
    for context, (project, package, name) in GRAMMARS.items():
        generated = f'{project}/src-gen/{package}/parser/antlr/internal/Internal{name}.g'
        for path in [generated, f'{project}/src/{package}/{name}.xtext']:
            actual = (args.pilot / path).read_bytes().replace(b'\r\n', b'\n')
            if actual != git('show', PIN + ':' + path).replace(b'\r\n', b'\n'):
                raise ValueError('Changed pinned grammar: ' + path)
            sources[path] = digest(actual)
        grammars[context] = args.pilot / generated
    structured = json.loads(GRAMMAR.read_text(encoding='utf-8'))
    if structured['provenance']['pilot_revision'] != PIN:
        raise ValueError('Structured grammar has a different Pilot pin')
    for path, expected in structured['provenance']['sources_sha256'].items():
        if path.endswith('.xtext') and digest((args.pilot / path).read_bytes()) != expected:
            raise ValueError('Structured/generated grammar source mismatch: ' + path)
    tool = args.pilot / 'org.omg.sysml.xtext/.antlr-generator-3.2.0-patch.jar'
    runtime = args.pilot / 'org.omg.sysml.interactive/target/org.omg.sysml.interactive-0.62.0-SNAPSHOT-all.jar'
    provenance = {'pilot_commit': PIN, 'source_sha256': sources, 'tool_sha256': digest(tool.read_bytes()),
                  'runtime_sha256': digest(runtime.read_bytes()), 'helper_sha256': digest(HELPER.read_bytes())}
    suffix = '.exe' if os.name == 'nt' else ''
    contexts, unsupported_contexts = {}, {}
    with tempfile.TemporaryDirectory(prefix='prediction-export-') as directory:
        directory = Path(directory)
        classpath = os.pathsep.join(map(str, [tool, runtime]))
        subprocess.run([str(args.java_bin / ('javac' + suffix)), '-encoding', 'UTF-8', '-cp', classpath,
                        '-d', str(directory), str(HELPER)], check=True, timeout=60)
        for context, grammar in grammars.items():
            out = directory / 'decision.json'
            requests, unsupported = decision_requests(structured, context)
            scope = directory / 'requests.json'
            scope.write_text(json.dumps(requests), encoding='utf-8')
            subprocess.run([str(args.java_bin / ('java' + suffix)), '-Xmx2g', '-cp', str(directory) + os.pathsep + classpath,
                            'dev.mercurio.pilot.PilotPredictionExporter', str(grammar), str(out), str(scope),
                            str(args.state_limit), str(args.configuration_limit)],
                           check=True, timeout=args.analysis_timeout)
            exported = json.loads(out.read_text(encoding='utf-8'))
            contexts[context] = exported['decisions']
            unsupported_contexts[context] = dict(sorted({**unsupported, **exported['unsupported']}.items()))
    bind_source_nodes(contexts, structured)
    result = {'schema': 'dev.mercurio.xtext-prediction.v1',
              'structured_grammar_sha256': digest(json.dumps(structured, sort_keys=True).encode('utf-8')),
              'scope': 'Definition-driven discovery of parser alternative and optional-entry decisions in every effective grammar context. Unmapped and unsupported decisions are listed separately. ANTLR frontend interpretation of pinned generated grammars; no generated Java execution. Resolved syntax predicates and constant-true fallback conditions are retained; native execution needs a bound syntax probe. Other predicate expressions and analysis warnings/errors remain explicitly unsupported. Import alone does not qualify parsing, token adaptation, models or specification compliance.',
              'analysis_limits': {'states': args.state_limit, 'configuration_additions': args.configuration_limit, 'timed_fallback': False},
              'provenance': provenance, 'contexts': contexts, 'unsupported': unsupported_contexts}
    # Java immutable-map iteration order varies between processes. Sort object
    # keys only; arrays retain ordered predicate priority and DFA state identities.
    text = json.dumps(result, indent=2, sort_keys=True) + '\n'
    if args.check:
        if args.output.read_text(encoding='utf-8') != text:
            raise ValueError('Stale upstream prediction export')
    else:
        args.output.write_text(text, encoding='utf-8', newline='\n')
    print('Upstream prediction export current:', sum(len(v) - 1 for v in contexts.values()), 'contextual decisions;', sum(len(v) for v in unsupported_contexts.values()), 'explicitly unsupported')


if __name__ == '__main__':
    main()
