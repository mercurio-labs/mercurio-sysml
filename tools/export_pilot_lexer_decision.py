"""Export pinned upstream token prediction as a native-consumable finite table."""
import argparse, hashlib, json, os, subprocess, tempfile
from pathlib import Path
from export_pilot_name_delegates import ROOT, PROFILE, PILOT, PIN, digest
HELPER=ROOT/'tools/pilot-exporter/src/main/java/dev/mercurio/pilot/PilotLexerDecisionExporter.java'
OUTPUT=PROFILE/'lexer-decision.extract.json'
def validate(doc):
    if doc['schema']!='dev.mercurio.lexer-decision.v1' or doc['provenance']['pilot_revision']!=PIN: raise ValueError('Changed decision schema or pin')
    validate_table(doc)
    if 'kerml' not in doc: raise ValueError('Missing KerML lexer decision')
    validate_table(doc['kerml'])

def validate_table(doc):
    keyword_labels={label for label in doc['labels'] if label.startswith('T__')}
    if set(doc['keywords'])!=keyword_labels or any(not isinstance(v,str) or not v for v in doc['keywords'].values()): raise ValueError('Incomplete keyword identities')
    count=len(doc['accept'])
    if not count or any(len(doc[k])!=count for k in ('eot','eof','min','max','transition')): raise ValueError('Incomplete decision arrays')
    if len(doc['special'])<count: raise ValueError('Incomplete special-state array')
    for key in ('eot','eof'):
        if any(x < -1 or x>=count for x in doc[key]): raise ValueError('Invalid decision target')
    for row in doc['transition']:
        if any(x < -1 or x>=count for x in row): raise ValueError('Invalid transition target')
    if any(x < -1 or x>len(doc['labels']) for x in doc['accept']): raise ValueError('Invalid acceptance label')
    for special in set(doc['special'])-{-1}:
        ranges=doc['special_ranges'].get(str(special),[])
        next_code=-1
        for low,high,target in ranges:
            if low!=next_code or high<low or target < -1 or target>=count: raise ValueError('Incomplete special transition domain')
            next_code=high+1
        if next_code!=65536: raise ValueError('Incomplete special transition domain')

def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--java-bin',type=Path)
    parser.add_argument('--validate',action='store_true')
    parser.add_argument('--check',action='store_true')
    args=parser.parse_args()
    if args.validate:
        doc=json.loads(OUTPUT.read_text(encoding='utf-8')); validate(doc)
        if doc['provenance']['helper_sha256']!=digest(HELPER.read_bytes()) or doc['provenance']['driver_sha256']!=digest(Path(__file__).read_bytes()): raise ValueError('Changed export tooling')
        print('Lexer decision structure and tooling provenance valid'); return
    if args.java_bin is None: parser.error('--java-bin is required for export/reproduction')
    if subprocess.check_output(['git','-C',str(PILOT),'rev-parse','HEAD'],text=True).strip()!=PIN: raise ValueError('Changed Pilot pin')
    runtime=PILOT/'org.omg.sysml.interactive/target/org.omg.sysml.interactive-0.62.0-SNAPSHOT-all.jar'
    expected=json.loads((PROFILE/'xtext-prediction-nfa.experimental.json').read_text(encoding='utf-8'))['provenance']['runtime_sha256']
    if digest(runtime.read_bytes())!=expected: raise ValueError('Changed pinned runtime')
    suffix='.exe' if os.name=='nt' else ''
    with tempfile.TemporaryDirectory(prefix='lexer-decision-') as temp:
        raw=Path(temp)/'decision.json'
        subprocess.run([str(args.java_bin/('javac'+suffix)),'-encoding','UTF-8','-cp',str(runtime),'-d',temp,str(HELPER)],check=True,timeout=60)
        subprocess.run([str(args.java_bin/('java'+suffix)),'-cp',temp+os.pathsep+str(runtime),'dev.mercurio.pilot.PilotLexerDecisionExporter',str(raw)],check=True,timeout=60)
        doc=json.loads(raw.read_text(encoding='utf-8'))
    doc['provenance']={'pilot_revision':PIN,'runtime_sha256':expected,'helper_sha256':digest(HELPER.read_bytes()),'driver_sha256':digest(Path(__file__).read_bytes())}
    validate(doc)
    text=json.dumps(doc,separators=(',',':'),sort_keys=True)+'\n'
    if args.check:
        if OUTPUT.read_text(encoding='utf-8')!=text: raise ValueError('Stale lexer decision')
    else: OUTPUT.write_text(text,encoding='utf-8',newline='\n')
    print(len(doc['accept']),'upstream lexer decision states;',len(doc['special_ranges']),'exhaustively resolved special states')
if __name__=='__main__': main()
