"""Extract top-level usage default types from a pinned Pilot fixture export."""
import argparse
import json
import subprocess
from pathlib import Path
from generate_release_defaults import digest

p = argparse.ArgumentParser(description=__doc__)
for name in ("export", "fixture", "pilot-root", "jar", "out"):
    p.add_argument("--" + name, type=Path, required=True)
p.add_argument("--namespace", action="append", default=["ReleaseOccurrenceDefaults"])
p.add_argument("--check", action="store_true")
p.add_argument("--include-subsets", action="store_true", help="Extract external owned-subsetting targets for exact selected elements")
p.add_argument("--element", action="append", default=[], help="Exact qualified name=lowering construct; otherwise select namespace usage members")
p.add_argument("--member-element", action="append", default=[], help="Qualified name=construct:member; observed type applies when this body member exists")
a = p.parse_args()
def git(*args):
    return subprocess.check_output(["git", "-C", str(a.pilot_root), *args], text=True).strip()
if git("status", "--porcelain"):
    raise ValueError("Pilot checkout must be clean")
d = json.loads(a.export.read_text(encoding="utf-8"))
selection = dict(item.rsplit("=", 1) for item in a.element)
if len(selection) != len(a.element):
    raise ValueError("Duplicate element selection")
member_selection = dict(item.rsplit("=", 1) for item in a.member_element)
if len(member_selection) != len(a.member_element) or set(selection) & set(member_selection):
    raise ValueError("Duplicate member selection")
observed = set()
defaults = {}
subsets = {}
for e in d["elements"]:
    name = e["qualified_name"]
    if selection or member_selection:
        if name not in selection and name not in member_selection:
            continue
        observed.add(name)
    elif name.rsplit("::", 1)[0] not in a.namespace or not e["kind"].endswith("Usage"):
        continue
    types = sorted({r["target"] for r in d["relationships"] if r["source"] == name and r["relation"] == "type"})
    if len(types) != 1 or any(types[0].startswith(namespace + "::") for namespace in a.namespace):
        raise ValueError(f"Expected one external implicit type for {name}: {types}")
    if a.include_subsets and name in selection:
        owned = {r["target"] for r in d["relationships"] if r["source"] == name and r["relation"] == "owned_subsetting"}
        targets = sorted({r["target"] for r in d["relationships"] if r["source"] in owned and r["relation"] == "subsetted_feature"})
        if not targets or any(target.startswith(name.split("::")[0] + "::") for target in targets):
            raise ValueError(f"Expected external implicit subsettings for {name}: {targets}")
        subsets[selection[name]] = {"subsetted_feature_refs": targets}
    if name in member_selection:
        construct, member = member_selection[name].rsplit(":", 1)
        values = defaults.setdefault(construct, {}).setdefault("member_type_refs", {})
        if member in values and values[member] != types[0]:
            raise ValueError("Inconsistent member defaults")
        values[member] = types[0]
    else:
        construct = selection.get(name, e["kind"])
        values = defaults.setdefault(construct, {})
        if "type_ref" in values and values["type_ref"] != types[0]:
            raise ValueError("Inconsistent usage defaults")
        values["type_ref"] = types[0]
if set(selection) | set(member_selection) != observed:
    raise ValueError("Missing selected elements: " + str((set(selection) | set(member_selection)) - observed))
if not defaults:
    raise ValueError("Empty observation set")
output = {"schema_version": 1, "source": {
    "pilot_commit": git("rev-parse", "HEAD"), "pilot_dirty": False,
    "fixture_sha256": digest(a.fixture), "export_sha256": digest(a.export),
    "jar_sha256": digest(a.jar), "extractor": "generate_release_usage_defaults.py", "extractor_version": 1,
    "export_metadata": d.get("metadata", {})}, "usage_type_defaults": defaults}
if a.include_subsets:
    output["usage_subset_defaults"] = subsets
    output["source"]["include_subsets"] = True
if selection:
    output["source"]["selected_elements"] = selection
if member_selection:
    output["source"]["selected_member_elements"] = member_selection
if a.check:
    assert output == json.loads(a.out.read_text(encoding="utf-8")), "Usage default artifact drift"
else:
    a.out.write_text(json.dumps(output, indent=2) + "\n", encoding="utf-8")
print(f"{len(defaults)} usage defaults verified")
