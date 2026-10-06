"""Import the common featuring producer and observe its no-owning-Type branch.

Handwritten native admission excludes specialized Connector/Expression contexts.
"""
import argparse,hashlib,json,os,subprocess,tempfile
from pathlib import Path
from export_pilot_feature_redefinitions import ROOT,PROFILE,PILOT,PIN,require
HELPER=ROOT/'tools/pilot-exporter/src/main/java/dev/mercurio/pilot/PilotNoOwningTypeFeaturing.java'
TREE=HELPER.with_name('PilotResultConstructionExporter.java')
SELECTOR=HELPER.with_name('PilotFeatureRedefinitionExporter.java')
OPERAND=HELPER.with_name('PilotOperandProbe.java')
OUTPUT=PROFILE/'no-owning-type-featuring.extract.json'
GENERATED=ROOT/'crates/mercurio-sysml/src/language_frontend/lowering/emit/no_owning_type_featuring_generated.rs'
DEFINITION_SHA='93abea9dbc3fd6e97dfbe3edb387cdd0ce62043afc42aaa0953c47b30eb22b5d'
def digest(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def definition_digest(doc):return hashlib.sha256(json.dumps(doc['definitions'],sort_keys=True,separators=(',',':')).encode()).hexdigest()
def render(doc):
 require(definition_digest(doc)==DEFINITION_SHA,'Changed resolved featuring producer')
 expected={r['kind'] for r in json.loads((PROFILE/'feature-redefinitions.extract.json').read_text(encoding='utf-8'))['bindings']}
 require(set(doc['bindings'])==expected,'Changed concrete Feature inventory')
 require(len(doc['controls'])==48*len(expected),'Incomplete no-owning-Type observations')
 required={(k,c,f,shape) for k in expected for c in ('detached','package','package_feature','library_package','namespace','namespace_feature') for f in (False,True) for shape in ('empty','explicit','duplicate','first_chain')}
 require({(r['kind'],r['context'],r['flag_value'],r['shape']) for r in doc['controls']}==required,'Incomplete no-owning-Type matrix')
 admitted=[]
 for kind,binding in doc['bindings'].items():
  require(binding['owning_type_provider']=='org.omg.sysml.adapter.FeatureAdapter#computeFeaturingType','Unknown owning-Type producer')
  require(binding['specialized_context']==bool({'Connector','Expression'} & (set(binding['ancestry'])|{kind})),'Specialized ancestry mismatch')
  if not binding['specialized_context']:admitted.append(kind)
 return '// Generated from resolved featuring producer bindings; separate native admission proves its no-owner branch.\n'+ 'pub(super) fn supports(kind: &str) -> bool { matches!(kind, '+ ' | '.join(json.dumps(k) for k in sorted(admitted)) +') }\n'
def main():
 parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--java-bin',type=Path,required=True);parser.add_argument('--check',action='store_true');args=parser.parse_args()
 def git(*commands):return subprocess.check_output(['git','-c','safe.directory='+str(PILOT),'-C',str(PILOT),*commands])
 require(git('rev-parse','HEAD').decode().strip()==PIN,'Pilot pin changed')
 jar=PILOT/'org.omg.sysml.interactive/target/org.omg.sysml.interactive-0.62.0-SNAPSHOT-all.jar'
 expected=json.loads((PROFILE/'xtext-prediction-nfa.experimental.json').read_text(encoding='utf-8'))['provenance']['runtime_sha256'];require(digest(jar)==expected,'Pilot runtime changed')
 source=PILOT/'org.omg.sysml.logic/src/main/java/org/omg/sysml/adapter/FeatureAdapter.java';relative=source.relative_to(PILOT).as_posix();data=source.read_bytes().replace(b'\r\n',b'\n');require(data==git('show',PIN+':'+relative).replace(b'\r\n',b'\n'),'Modified producer source')
 suffix='.exe' if os.name=='nt' else '';java,javac=[args.java_bin/(n+suffix) for n in ('java','javac')]
 with tempfile.TemporaryDirectory(prefix='no-owning-type-featuring-') as temp:
  raw=Path(temp)/'controls.json'
  subprocess.run([str(javac),'-encoding','UTF-8','-cp',str(jar),'-d',temp,*[str(p) for p in [HELPER,TREE,SELECTOR,OPERAND]]],check=True,timeout=60)
  subprocess.run([str(java),'-cp',temp+os.pathsep+str(jar),'dev.mercurio.pilot.PilotNoOwningTypeFeaturing',str(jar),str(raw),str(source)],check=True,timeout=90)
  doc=json.loads(raw.read_text(encoding='utf-8'))
 doc['schema']='dev.mercurio.no-owning-type-featuring.v1';doc['scope']='Resolved common owning-Type producer, invoked on every concrete Feature in detached/package/plain-Namespace contexts. Specialized Connector/Expression context producers excluded from native admission; no full transformation or resource qualification.'
 doc['provenance']={'pilot_revision':PIN,'runtime_sha256':expected,'source_sha256':{relative:hashlib.sha256(data).hexdigest()},'helper_sha256':digest(HELPER),'tree_helper_sha256':digest(TREE),'selector_helper_sha256':digest(SELECTOR),'driver_sha256':digest(Path(__file__)),'ecore_sha256':digest(PROFILE/'ecore-effective.extract.json'),'toolchain':{name:subprocess.check_output([str(exe),'-version'],stderr=subprocess.STDOUT,text=True).strip() for name,exe in [('java',java),('javac',javac)]}}
 generated=render(doc);serialized=json.dumps(doc,indent=2,sort_keys=True)+'\n'
 if args.check:require(OUTPUT.read_text(encoding='utf-8')==serialized and GENERATED.read_text(encoding='utf-8')==generated,'Stale no-owning-Type export')
 else:OUTPUT.write_text(serialized,encoding='utf-8');GENERATED.write_text(generated,encoding='utf-8')
 print(len(doc['bindings']),'resolved concrete Features;',len(doc['controls']),'common-producer observations')
if __name__=='__main__':main()
