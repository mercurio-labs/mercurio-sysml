"""Resolve pinned Connector context/lifecycle programs; extraction is not support."""
import argparse,hashlib,json,os,subprocess,tempfile
from pathlib import Path
from export_pilot_feature_redefinitions import ROOT,PROFILE,PILOT,PIN,require
HELPER=ROOT/'tools/pilot-exporter/src/main/java/dev/mercurio/pilot/PilotConnectorContextExporter.java'
TREE=HELPER.with_name('PilotResultConstructionExporter.java');SELECTOR=HELPER.with_name('PilotFeatureRedefinitionExporter.java');OPERAND=HELPER.with_name('PilotOperandProbe.java')
OUTPUT=PROFILE/'connector-contexts.extract.json'
GENERATED=ROOT/'crates/mercurio-sysml/src/language_frontend/lowering/emit/connector_contexts_generated.rs'
UTILITY_SHA='476c67057bce6e28f6a39f4faf1c5096fe066bbc047328fe042f89127a60338b'
SOURCE_NAMES=['adapter/ConnectorAdapter.java','adapter/SuccessionAsUsageAdapter.java','adapter/ConnectionUsageAdapter.java','adapter/FlowAdapter.java','adapter/FlowUsageAdapter.java','adapter/FeatureAdapter.java','adapter/TypeAdapter.java','adapter/UsageAdapter.java','util/ConnectorUtil.java','delegate/setting/Connector_defaultFeaturingType_SettingDelegate.java']
def digest(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def validate(doc):
 expected={kind for kind,row in json.loads((PROFILE/'no-owning-type-featuring.extract.json').read_text(encoding='utf-8'))['bindings'].items() if 'Connector' in row['ancestry'] or kind=='Connector'}
 require(set(doc['bindings'])==expected,'Changed concrete Connector inventory')
 require('org.omg.sysml.delegate.setting.Connector_defaultFeaturingType_SettingDelegate#basicGet' in doc['definitions'],'Missing resolved default featuring delegate')
 require('org.omg.sysml.adapter.SuccessionAsUsageAdapter#addContextFeaturingType' in doc['definitions'],'Missing resolved succession context')
 for kind,row in doc['bindings'].items():
  require(row['adapter'].startswith('org.omg.sysml.adapter.') and row['transform'].startswith('org.omg.sysml.adapter.'),'Unresolved runtime dispatch '+kind)
  require(row['transform'] in doc['definitions'],'Missing resolved transform body '+kind)
  if row['context'] is not None:require(row['context'] in doc['definitions'],'Missing resolved context body '+kind)
 return expected
def render(doc):
 expected=validate(doc)
 require(hashlib.sha256(json.dumps(doc['definitions']['org.omg.sysml.util.ConnectorUtil#getContextTypeFor'],sort_keys=True,separators=(',',':')).encode()).hexdigest()==UTILITY_SHA,'Changed resolved context utility')
 shapes={'empty','single_empty','single','shared_two','shared_three','disjoint_two','disjoint_three','ordered_two','direct_two','direct_all_three','direct_partial_three'}
 require(len(doc['controls'])==len(expected)*len(shapes),'Incomplete context controls')
 require({(r['kind'],r['shape']) for r in doc['controls']}=={(kind,shape) for kind in expected for shape in shapes},'Incomplete context matrix')
 return '// Generated concrete utility inventory; lifecycle producers are separately implemented.\n'+'pub(super) const UTILITY_SHA: &str = '+json.dumps(UTILITY_SHA)+';\n'+'pub(super) fn supports(kind: &str) -> bool { matches!(kind, '+' | '.join(json.dumps(k) for k in sorted(expected))+') }\n'
def main():
 parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--java-bin',type=Path,required=True);parser.add_argument('--check',action='store_true');args=parser.parse_args()
 def git(*commands):return subprocess.check_output(['git','-c','safe.directory='+str(PILOT),'-C',str(PILOT),*commands])
 require(git('rev-parse','HEAD').decode().strip()==PIN,'Pilot pin changed')
 jar=PILOT/'org.omg.sysml.interactive/target/org.omg.sysml.interactive-0.62.0-SNAPSHOT-all.jar';runtime=json.loads((PROFILE/'xtext-prediction-nfa.experimental.json').read_text(encoding='utf-8'))['provenance']['runtime_sha256'];require(digest(jar)==runtime,'Pilot runtime changed')
 sources=[PILOT/'org.omg.sysml.logic/src/main/java/org/omg/sysml'/name for name in SOURCE_NAMES];hashes={}
 for source in sources:
  relative=source.relative_to(PILOT).as_posix();data=source.read_bytes().replace(b'\r\n',b'\n');require(data==git('show',PIN+':'+relative).replace(b'\r\n',b'\n'),'Modified context source');hashes[relative]=hashlib.sha256(data).hexdigest()
 suffix='.exe' if os.name=='nt' else '';java,javac=[args.java_bin/(name+suffix) for name in ('java','javac')]
 with tempfile.TemporaryDirectory(prefix='connector-contexts-') as temp:
  raw=Path(temp)/'resolved.json'
  subprocess.run([str(javac),'-encoding','UTF-8','-cp',str(jar),'-d',temp,*[str(p) for p in [HELPER,TREE,SELECTOR,OPERAND]]],check=True,timeout=60)
  subprocess.run([str(java),'-cp',temp+os.pathsep+str(jar),'dev.mercurio.pilot.PilotConnectorContextExporter',str(jar),str(raw),*[str(p) for p in sources]],check=True,timeout=90)
  doc=json.loads(raw.read_text(encoding='utf-8'))
 validate(doc);doc['schema']='dev.mercurio.connector-contexts.v1';doc['scope']='Resolved typed programs and runtime dispatch only; native consumption and semantic verification must be recorded separately. Parent transformation, common Connector context utility and other subclass transformations remain dependencies.'
 doc['provenance']={'pilot_revision':PIN,'runtime_sha256':runtime,'source_sha256':hashes,'helper_sha256':digest(HELPER),'tree_helper_sha256':digest(TREE),'selector_helper_sha256':digest(SELECTOR),'driver_sha256':digest(Path(__file__)),'ecore_sha256':digest(PROFILE/'ecore-effective.extract.json'),'concrete_inventory_sha256':digest(PROFILE/'no-owning-type-featuring.extract.json'),'toolchain':{name:subprocess.check_output([str(exe),'-version'],stderr=subprocess.STDOUT,text=True).strip() for name,exe in [('java',java),('javac',javac)]}}
 generated=render(doc);text=json.dumps(doc,indent=2,sort_keys=True)+'\n'
 if args.check:require(OUTPUT.read_text(encoding='utf-8')==text and GENERATED.read_text(encoding='utf-8')==generated,'Stale Connector context export')
 else:OUTPUT.write_text(text,encoding='utf-8');GENERATED.write_text(generated,encoding='utf-8')
 print(len(doc['bindings']),'resolved Connector bindings;',len(doc['definitions']),'typed programs; no native qualification claimed')
if __name__=='__main__':main()
