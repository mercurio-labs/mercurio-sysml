"""Guard bounded resolved Usage variability; Rust owns graph navigation and scheduling."""
import argparse,hashlib,json
from pathlib import Path
from generate_expression_redefinitions import canonical,require
from run_usage_variability_probe import validate
ROOT=Path(__file__).resolve().parents[1]
INPUT=ROOT/'docs/conformance/2026-08-support/usage-variability-pilot-controls.json'
OUTPUT=ROOT/'crates/mercurio-sysml/src/language_frontend/lowering/emit/usage_variability_generated.rs'
TREE='ea205892ef967864aa5a8a04b3caad2095c074808f91644048cb4480387e1caf'
def walk(node):
    yield node
    for child in node.get('children',[]):yield from walk(child)
def generate(data):
    require(data['schema']=='dev.mercurio.usage-variability-controls.v1','Changed variability schema')
    validate(data)
    methods=data['variability_methods']
    require(hashlib.sha256(json.dumps(canonical(methods),sort_keys=True,separators=(',',':')).encode()).hexdigest()==TREE,'Changed resolved variability programs; review required')
    adapter=methods['org.omg.sysml.adapter.UsageAdapter#mayTimeVary']
    literals=[json.loads(n['source']) for n in walk(adapter) if n['kind']=='STRING_LITERAL']
    require(literals==['Occurrences::Occurrence','Links::SelfLink','Occurrences::HappensLink','Actions::Action'],'Changed role inventory')
    delegate=methods['org.omg.sysml.delegate.setting.Usage_mayTimeVary_SettingDelegate#basicGet']
    exclusions=[n['children'][1]['type'].split('.')[-1] for n in walk(delegate) if n['kind']=='INSTANCE_OF']
    require(len(exclusions)==2,'Changed exclusion inventory')
    code='// Generated resolved Usage variability bindings; navigation remains handwritten.\n'
    code+=f'pub(super) const PROGRAM_SHA: &str = "{TREE}";\n'
    for key,value in zip(['OCCURRENCE','SELF_LINK','HAPPENS_LINK','ACTION'],literals):code+=f'pub(super) const {key}: &str = "{value}";\n'
    code+='pub(super) const EXCLUDED: &[&str] = &['+','.join(json.dumps(k) for k in exclusions)+'];\n'
    code+='pub(super) const GETTER_BINDINGS: &[&str] = &['+','.join(json.dumps(k) for k in sorted(data['bindings']))+'];\n'
    return code
def main():
    p=argparse.ArgumentParser();p.add_argument('--check',action='store_true');a=p.parse_args();code=generate(json.loads(INPUT.read_text(encoding='utf-8')))
    if a.check:require(OUTPUT.read_text(encoding='utf-8')==code,'Stale variability program bindings')
    else:OUTPUT.write_text(code,encoding='utf-8',newline='\n')
    print('Usage variability: 3 resolved semantic programs, 47 getter bindings, 752 controls')
if __name__=='__main__':main()
