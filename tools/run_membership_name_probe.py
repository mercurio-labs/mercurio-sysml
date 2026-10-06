"""Export six independent pinned Pilot stored Membership name observations."""
from pathlib import Path
import hashlib,json,subprocess,os,tempfile
root=Path(__file__).resolve().parents[1];pilot=root.parent/'target/support-2026-08/pilot-pinned-2026-08';runtime=pilot/'org.omg.sysml.interactive/target/org.omg.sysml.interactive-0.62.0-SNAPSHOT-all.jar';profile=root/'crates/mercurio-sysml/resources/metamodels/sysml-2.0-pilot-2026-08';reference=json.loads((profile/'xtext-prediction-nfa.experimental.json').read_text(encoding='utf-8'));sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
assert sha(runtime)==reference['provenance']['runtime_sha256']
helper=root/'tools/pilot-exporter/src/main/java/dev/mercurio/pilot/PilotMembershipNameProbe.java';jdk=Path('D:/dev/jdks/jdk21/bin');out=root/'docs/conformance/2026-08-support/membership-name-pilot-controls.json'
with tempfile.TemporaryDirectory(prefix='membership-names-') as tmp:
 subprocess.run([str(jdk/'javac.exe'),'-encoding','UTF-8','-cp',str(runtime),'-d',tmp,str(helper)],check=True,timeout=60)
 raw=Path(tmp)/'raw.json';subprocess.run([str(jdk/'java.exe'),'-cp',tmp+os.pathsep+str(runtime),'dev.mercurio.pilot.PilotMembershipNameProbe',str(raw)],check=True,timeout=60);cases=json.loads(raw.read_text())
paths=['org.omg.sysml.model/src/main/java/org/omg/sysml/lang/sysml/impl/MembershipImpl.java','org.omg.sysml/model/SysML.ecore']
with tempfile.TemporaryDirectory(prefix='membership-git-') as tmp:
 cfg=Path(tmp)/'gitconfig';cfg.write_text('[safe]\n\tdirectory = '+pilot.as_posix()+'\n');env=dict(os.environ,GIT_CONFIG_GLOBAL=str(cfg));pin=subprocess.check_output(['git','-C',str(pilot),'rev-parse','HEAD'],env=env,text=True).strip();assert pin=='692170b71867353b8f90341e61556f49a5beb0e5';hashes={}
 for p in paths:
  data=(pilot/p).read_bytes().replace(b'\r\n',b'\n');assert data==subprocess.check_output(['git','-C',str(pilot),'show',pin+':'+p],env=env).replace(b'\r\n',b'\n');hashes[p]=hashlib.sha256(data).hexdigest()
report={'schema':'dev.mercurio.membership-name-pilot-controls.v1','scope':'Stored non-owning Membership attributes only. Owning membership derivation, endpoint resolution, publication and family qualification are separate.','cases':cases,'provenance':{'pilot_revision':pin,'runtime_sha256':sha(runtime),'source_sha256':hashes,'helper_sha256':sha(helper),'driver_sha256':sha(Path(__file__)),'ecore_effective_sha256':sha(profile/'ecore-effective.extract.json'),'toolchain':subprocess.check_output([str(jdk/'java.exe'),'-version'],stderr=subprocess.STDOUT,text=True).strip()}}
out.write_text(json.dumps(report,indent=2)+'\n',encoding='utf-8');print('Six independent pinned Membership name controls exported')
