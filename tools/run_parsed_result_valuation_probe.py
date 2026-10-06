from pathlib import Path
import argparse,json,hashlib,subprocess,tempfile,os
from export_library_reference_inventory import ROOT,PIN,read_bytes
PILOT=ROOT.parent/'target/support-2026-08/pilot-pinned-2026-08'
HELPER=ROOT/'tools/pilot-exporter/src/main/java/dev/mercurio/pilot/PilotParsedResultValuationProbe.java'
OUTPUT=ROOT/'docs/conformance/2026-08-support/parsed-result-valuation-pilot-controls.json'
def digest(data):return hashlib.sha256(data).hexdigest()
def main():
 p=argparse.ArgumentParser();p.add_argument('--java-bin',type=Path,required=True);p.add_argument('--check',action='store_true');a=p.parse_args()
 inventory=json.loads((ROOT/'docs/conformance/2026-08-support/library-reference-inventory.json').read_text(encoding='utf-8'));assert inventory['provenance']['pilot_commit']==PIN
 jar=PILOT/'org.omg.sysml.interactive/target/org.omg.sysml.interactive-0.62.0-SNAPSHOT-all.jar';assert digest(jar.read_bytes())==inventory['provenance']['jar_sha256']
 release=ROOT.parent/'target/upstream/SysML-v2-Release';paths=[]
 for name,h in inventory['provenance']['library_inputs'].items():
  path=release/name;assert digest(read_bytes(path))==h;paths.append(path)
 suffix='.exe' if os.name=='nt' else '';java=a.java_bin/('java'+suffix);javac=a.java_bin/('javac'+suffix)
 with tempfile.TemporaryDirectory(prefix='parsed-valuation-') as folder:
  folder=Path(folder);inputs=folder/'inputs.txt';inputs.write_text('\n'.join(str(p) for p in paths)+'\n',encoding='utf-8');raw=folder/'raw.json'
  subprocess.run([str(javac),'-encoding','UTF-8','-cp',str(jar),'-d',str(folder),str(HELPER)],check=True,timeout=60)
  subprocess.run([str(java),'-cp',str(folder)+os.pathsep+str(jar),'dev.mercurio.pilot.PilotParsedResultValuationProbe',str(inputs),str(raw)],check=True,timeout=240)
  cases=json.loads(raw.read_text(encoding='utf-8'));assert len(cases)==12 and {r['case'] for r in cases}==set(range(12))
 doc={'schema':'dev.mercurio.parsed-result-valuation-controls.v1','scope':'Parsed result-valuation component observations using genuine typed pinned library declarations. No validation or native qualification. Dependency failures remain explicit.','provenance':{'pilot_commit':PIN,'library_inputs':inventory['provenance']['library_inputs'],'runtime_sha256':digest(jar.read_bytes()),'helper_sha256':digest(HELPER.read_bytes()),'driver_sha256':digest(Path(__file__).read_bytes()),'java_sha256':digest(java.read_bytes()),'javac_sha256':digest(javac.read_bytes())},'controls':cases}
 text=json.dumps(doc,indent=2,sort_keys=True)+'\n'
 if a.check:assert OUTPUT.read_text(encoding='utf-8')==text,'Changed parsed valuation observations'
 else:OUTPUT.write_text(text,encoding='utf-8',newline='\n')
 print('Parsed valuation contexts:',len(cases),'; observed',sum(c['status']=='observed_component' for c in cases))
if __name__=='__main__':main()
