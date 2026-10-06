"""Audit the complete fixed valuation bundle; imported structure is not semantic support."""
from pathlib import Path
import argparse,collections,json
from run_value_result_library_dependency_bundle import ROOT,EV,read,sha,save


def produce():
    base=EV.parent;progress_path=EV/"value-result-value-binding-library-progress.json";progress=read(progress_path)
    assert progress["status"]=="bounded_library_dependency_stage_verified" and progress["library_ports_matched"]==5 and progress["provider_getters_matched"]==14
    assert all(sha(Path(p))==h for p,h in progress["input_sha256"].items())
    assert progress["graph_unchanged_on_replay"] and progress["strict_families_qualified"]==0
    bundle_path=EV/"value-result-value-binding-bundle-spec.json";bundle=read(bundle_path)
    record_path=Path(progress["current_record_path"]);queries_path=Path(progress["current_query_path"])
    assert sha(record_path)==progress["current_record_sha256"] and sha(queries_path)==progress["current_query_sha256"]
    record=read(record_path);index={node["id"]:node for node in record["inspection"]["constructed_elements"]}
    query_index={(row["owner_id"],row["field"]):row for row in read(queries_path)["queries"]}
    expressions={row["owner_id"]:row for row in bundle["expressions"]}
    meta=ROOT/"crates/mercurio-sysml/resources/metamodels/sysml-2.0-pilot-2026-08/ecore-effective.extract.json"
    defaults={row["name"]:row["default_value"] for row in read(meta)["features"] if row["owner"].endswith("#//FeatureValue")}
    assert defaults["isDefault"] is False and defaults["isInitial"] is False
    valuations=[]
    for original in bundle["valuations"]:
        owner=index[original["owner_id"]];value=index[original["valuation_id"]];expression=index[original["expression_id"]]
        assert owner["kind"]=="SysML::Feature" and value["kind"]=="SysML::FeatureValue"
        assert value["properties"].get("is_default",defaults["isDefault"])==original["is_default"]
        assert value["properties"].get("is_initial",defaults["isInitial"])==original["is_initial"]
        relationships=[index[id] for id in owner["properties"]["owned_relationship"]]
        result=query_index[(expression["id"],"result")];negative=expressions[expression["id"]]["link_negative"]
        valuations.append(dict(original,owner_kind=owner["kind"],owner_owned_relationship_kinds=[node["kind"] for node in relationships],
          original_declared_direction=owner["properties"].get("declared_direction"),original_completion_flag=owner["properties"].get("is_implied_included"),
          result_read_status=result["status"],result_targets=result.get("targets",[]),result_dependency=result.get("dependency"),
          required_link_negative=negative,eligible_for_binding_stage=not negative,
          semantic_dependencies=["Effective direction through its delegate/parameter context; declared direction is insufficient",
            "Actual Expression result and result typing, plus complete superclass/Expression lifecycle",
            "Owned valuation selection and imported FeatureValue.value/featureWithValue contracts",
            "Bound-value specialization when its normative predicate applies",
            "Binary default/effective ends, canonical result chain and featuring",
            "Initial that/startShot context when applicable; default values produce no value binding"]))
    assert len(valuations)==45 and sum(row["eligible_for_binding_stage"] for row in valuations)==41
    assert len({row["context"] for row in valuations if row["required_link_negative"]})==2
    pilot=ROOT.parent/"target/support-2026-08/pilot-pinned-2026-08/org.omg.sysml.logic/src/main/java/org/omg/sysml"
    sources=[pilot/"adapter/FeatureAdapter.java",pilot/"util/FeatureUtil.java",pilot/"delegate/setting/FeatureValue_value_SettingDelegate.java"]
    paths=[progress_path,bundle_path,record_path,queries_path,meta,base/"value-result-type-warning-normative-disposition.json",Path(__file__)]+sources
    return dict(schema="dev.mercurio.general-value-provider-dependency-bundle.v1",qualification_certificate=False,status="dependency_bundle_audited_implementation_required",
      fixed_contexts=26,fixed_resources=119,fixed_libraries=94,required_valuations=45,eligible_valuation_stages=41,required_link_negative_valuations=4,required_link_negative_contexts=2,
      required_binding_categories=dict(collections.Counter(row["required_binding"] for row in valuations)),
      eligible_binding_categories=dict(collections.Counter(row["required_binding"] for row in valuations if row["eligible_for_binding_stage"])),
      result_read_outcomes=dict(collections.Counter(row["result_read_status"] for row in valuations)),
      feature_value_imported_defaults=dict(is_default=defaults["isDefault"],is_initial=defaults["isInitial"]),valuations=valuations,input_sha256={str(path):sha(path) for path in paths},
      shared_strategies=[dict(strategy="valuation_selection_and_non_typing_contribution_assessment",required="Imported ownership/metaclass/default contracts and explicit semantic predicates; no literal-kind or completed-flag shortcut."),
        dict(strategy="general_bound_value_chain_and_binary_binding",required="All eligible nondefault Expression kinds share one typed plan and transaction. Reuse result/default/end/chain/binding consumers; enumerate unresolved library reads."),
        dict(strategy="default_and_initial_dispatch",required="Default values produce no value binding. Initial nondefault binding uses actual Base::things::that and Occurrences::Occurrence::startShot; fallback and validation stay explicit."),
        dict(strategy="complete_owner_and_expression_lifecycle",required="Bound specialization, superclass contributions, direction, bounds and validation are dependencies; a completed binding does not complete a receiver.")],
      acceptance=["Retain all 45 valuations, including nested argument/chain valuations; all 26 contexts and both original Ecore mutations",
        "Written specification is normative; Java adapter/util bodies are independent implementation evidence",
        "Batch ordinary/default/initial/bound/directed controls; four valuations in the two required link-negative contexts remain unqualified",
        "Unknown applicability, generic failures and extracted signatures do not establish support",
        "Same-stage independent ordered effects, atomic failure, canonical publication and fresh replay; full context certificates remain separate"],
      strict_contexts_qualified=0,strict_obligations_qualified=0,strict_families_qualified=0,candidate_promoted=False)


def main():
    parser=argparse.ArgumentParser();parser.add_argument("--check",action="store_true");args=parser.parse_args()
    value=produce();path=EV/"value-result-general-value-provider-dependency-bundle.json"
    if args.check:assert read(path)==value
    else:assert not path.exists();save(path,value)
    print("General provider bundle: 45 valuations; 41 eligible stages; four valuations in two required link-negative contexts;",value["result_read_outcomes"],flush=True)

if __name__=="__main__":main()
