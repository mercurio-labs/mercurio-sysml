"""Resolve pinned Connector context/lifecycle programs; extraction is not support."""
import argparse,hashlib,json,os,subprocess,tempfile
from pathlib import Path
from export_pilot_feature_redefinitions import ROOT,PROFILE,PILOT,PIN,require
HELPER=ROOT/'tools/pilot-exporter/src/main/java/dev/mercurio/pilot/PilotTransitionContextStage.java'
TREE=HELPER.with_name('PilotResultConstructionExporter.java');SELECTOR=HELPER.with_name('PilotFeatureRedefinitionExporter.java');OPERAND=HELPER.with_name('PilotOperandProbe.java')
OUTPUT=PROFILE/'transition-context-stage.extract.json'
GENERATED=ROOT/'crates/mercurio-sysml/src/language_frontend/lowering/emit/transition_context_stage_generated.rs'
PROGRAM_SHA='3d4e719d72167a3e359e044379285ea4b5ff3c9adbd54d7490536995b6a3c72f'
SOURCE_NAMES=['adapter/ElementAdapter.java','adapter/FeatureAdapter.java','adapter/TypeAdapter.java','adapter/SuccessionAsUsageAdapter.java','util/FeatureUtil.java','util/ElementUtil.java']

def digest(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def program_digest(doc):return hashlib.sha256(json.dumps(doc['definitions'],sort_keys=True,separators=(',',':')).encode()).hexdigest()
def render(doc):
 require(program_digest(doc)==PROGRAM_SHA,'Changed resolved context stage program')
 required={(p,s,e) for p in ('TransitionUsage','StateUsage','Namespace') for s in ('empty','one','two','duplicate') for e in (False,True)}
 require(len(doc['controls'])==24 and {(r['parent'],r['shape'],r['existing']) for r in doc['controls']}==required,'Incomplete context stage controls')
 require(all(r['stage_guard_supplied'] is True and r['completion_flags'] is False for r in doc['controls']),'Unproved stage/completion claim')
 for key in ('org.omg.sysml.adapter.ElementAdapter#transform','org.omg.sysml.adapter.FeatureAdapter#doTransform','org.omg.sysml.adapter.TypeAdapter#doTransform','org.omg.sysml.adapter.SuccessionAsUsageAdapter#addContextFeaturingType','org.omg.sysml.util.FeatureUtil#insertImplicitTypeFeaturings'):
  require(key in doc['definitions'],'Missing resolved stage program')
 return '// Generated guard for prepared featuring stage; full transformation is separate.\n'+'pub(super) const PROGRAM_SHA: &str = '+json.dumps(program_digest(doc))+';\n'

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
  subprocess.run([str(java),'-cp',temp+os.pathsep+str(jar),'dev.mercurio.pilot.PilotTransitionContextStage',str(jar),str(raw),*[str(p) for p in sources]],check=True,timeout=90)
  doc=json.loads(raw.read_text(encoding='utf-8'))
 render(doc);doc['schema']='dev.mercurio.transition-context-stage.v1';doc['scope']='Resolved prepared parent-featuring/context stage programs and independent supplied-stage observations. No complete parent or Connector lifecycle claim.'
 doc['provenance']={'pilot_revision':PIN,'runtime_sha256':runtime,'source_sha256':hashes,'helper_sha256':digest(HELPER),'tree_helper_sha256':digest(TREE),'selector_helper_sha256':digest(SELECTOR),'driver_sha256':digest(Path(__file__)),'ecore_sha256':digest(PROFILE/'ecore-effective.extract.json'),'concrete_inventory_sha256':digest(PROFILE/'ecore-effective.extract.json'),'toolchain':{name:subprocess.check_output([str(exe),'-version'],stderr=subprocess.STDOUT,text=True).strip() for name,exe in [('java',java),('javac',javac)]}}
 generated=render(doc);text=json.dumps(doc,indent=2,sort_keys=True)+'\n'
 if args.check:require(OUTPUT.read_text(encoding='utf-8')==text and GENERATED.read_text(encoding='utf-8')==generated,'Stale Transition context-stage export')
 else:OUTPUT.write_text(text,encoding='utf-8');GENERATED.write_text(generated,encoding='utf-8')
 print(len(doc['controls']),'prepared-stage controls;',len(doc['definitions']),'typed programs; full parent lifecycle not qualified')
if __name__=='__main__':main()
