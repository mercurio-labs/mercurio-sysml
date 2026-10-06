"""Isolated Pilot controls for G05 documentation headers; no benchmark."""
import argparse
from concurrent.futures import ThreadPoolExecutor
import os
from pathlib import Path
import subprocess
from run_namespace_batch import ROOT, read, write, digest

def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--java-bin",type=Path,required=True)
    parser.add_argument("--suite", choices=["documentation", "anonymous-documentation", "annotations", "owned-annotations", "bare-comments", "annotation-legality", "membership-bodies", "namespace-visibility", "expose-bodies", "relationship-owned-elements", "qualified-visibility", "inherited-visibility", "inherited-types", "import-visibility", "import-feature-visibility", "library-visibility"], default="documentation")
    args=parser.parse_args()
    artifacts=ROOT.parent/"target"
    support=artifacts/"support-2026-08"/(args.suite+"-controls")
    fixtures=ROOT/"crates/mercurio-tools/corpus/release-2026-08"/args.suite
    paths=sorted(fixtures.glob("*"))
    if len(paths)!=8: raise ValueError("Expected eight documentation controls")
    cases=[{"relative_path":p.name,"input_files":[str(p)]} for p in paths]
    write(support/"spec.json",{"cases":cases})
    jar=artifacts/"upstream/kernel-0.62.0/sysml/jupyter-sysml-kernel-0.62.0-all.jar"
    helper=ROOT/"tools/pilot-exporter/src/main/java/dev/mercurio/pilot/PilotModelExporter.java"
    classes=support/"classes"; classes.mkdir(exist_ok=True)
    subprocess.run([str(args.java_bin/"javac.exe"),"-cp",str(jar),"-d",str(classes),str(helper)],check=True)
    def run(case):
        folder=support/case["relative_path"]
        write(folder/"spec.json",{"cases":[case]})
        positive=case["relative_path"].startswith("positive")
        if args.suite == "relationship-owned-elements":
            # These fixture names predate the structural grammar extraction.
            # Ordinary relationship children are legal only in the language
            # whose pinned RelationshipBody has the ownedRelatedElement slot.
            language = Path(case["relative_path"]).suffix.lstrip(".")
            contract = read(ROOT / "crates/mercurio-sysml/resources/metamodels/sysml-2.0-pilot-2026-08/namespace-grammar.extract.json")
            slots = contract["languages"][language]["relationship_body"]["slots"]
            positive = positive and any(slot["feature"] == "ownedRelatedElement" for slot in slots)
        graph=str(folder/"graph.json") if positive else "-"
        with (folder/"oracle.log").open("w",encoding="utf-8") as log:
            subprocess.run([str(args.java_bin/"java.exe"),"-Xmx3g","-cp",str(classes)+os.pathsep+str(jar),"dev.mercurio.pilot.PilotModelExporter","--assessment",str(artifacts/"upstream/SysML-v2-Pilot-Implementation/sysml.library"),str(folder/"spec.json"),graph,str(folder/"result.json")],stdout=log,stderr=subprocess.STDOUT,check=True,timeout=600)
        result=read(folder/"result.json")["cases"]
        if len(result)!=1 or result[0]["status"] != ("ok" if positive else "error"): raise ValueError(str(result))
        print("Verified Pilot: "+case["relative_path"],flush=True)
        return result[0]
    with ThreadPoolExecutor(max_workers=2) as pool: results=list(pool.map(run,cases))
    write(ROOT/"docs/conformance/2026-08-support"/(args.suite+"-pilot-controls.json"),{"scope":"Isolated CheckMode.ALL documentation controls; no benchmark", "source_sha256":{p.name:digest(p) for p in paths},"jar_sha256":digest(jar),"helper_sha256":digest(helper),"cases":results})
if __name__=="__main__": main()
