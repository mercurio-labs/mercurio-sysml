"""Conservative bounded prediction over native Xtext IR, independent of model effects."""
import re

WIDTH = 2
LIMIT = 4096
CATEGORIES = {'<Name>', '<number:1>', '<number:2>', '<string>', '<comment>'}


def bounded_prefixes(programs, primitive_first):
    """Short tuples are complete paths; WIDTH tuples may have an unseen suffix."""
    summaries = {identity: set() for identity in programs}

    def product(left, right):
        result = {a if len(a) == WIDTH else (a + b)[:WIDTH]
                  for a in left for b in (right if len(a) < WIDTH else {()})}
        if len(result) > LIMIT:
            raise ValueError('Bounded prediction language exceeds limit')
        return result

    def prefixes(node, bare=False):
        kind = node['kind']
        if kind in ['create', 'capture']:
            values = {()}
        elif kind == 'call':
            values = summaries[node['rule']]
        elif kind in ['assign', 'contain', 'append_operand', 'link', 'cross_reference']:
            values = prefixes(node['terminal'])
        elif kind == 'token_datatype':
            values = prefixes(node['syntax'])
        elif kind == 'sequence':
            values = {()}
            for child in node['elements']:
                values = product(values, prefixes(child))
        elif kind == 'choice':
            values = set().union(*(prefixes(child) for child in node['elements']))
        else:
            heads, empty = primitive_first(node)
            values = {(head,) for head in heads}
            if empty:
                values.add(())
        cardinality = '' if bare else node['cardinality']
        if cardinality in ['*', '+']:
            repeated = set(values)
            while True:
                updated = repeated | product(repeated, values)
                if updated == repeated:
                    break
                repeated = updated
            values = repeated
        if cardinality in ['*', '?']:
            values = values | {()}
        return values

    while True:
        updated = {identity: prefixes(rule['body']) for identity, rule in programs.items()}
        if updated == summaries:
            return prefixes
        summaries = updated


def overlap(left, right):
    """Fail closed on lexical aliases and legacy split punctuation keywords."""
    def units(path):
        for symbol in path:
            # Legacy punctuation keywords can occupy one or several tokens.
            # Comparing character units is conservative across either layout.
            if symbol not in CATEGORIES and all(not (c.isalnum() or c == '_') for c in symbol):
                yield from symbol
            else:
                yield symbol
    for a, b in zip(units(left), units(right)):
        if a == b:
            continue
        if a in CATEGORIES or b in CATEGORIES:
            if a not in CATEGORIES:
                a, b = b, a
            if a == '<Name>' and b not in CATEGORIES and re.match(r"[A-Za-z_']", b):
                continue
            if a.startswith('<number:') and b not in CATEGORIES and b[:1].isdigit():
                continue
            return False
        return False
    return True


def disjoint(left, right):
    return bool(left) and bool(right) and not any(overlap(a, b) for a in left for b in right)
