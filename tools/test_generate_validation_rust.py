"""Focused source-backend controls, including compiled generated Rust execution."""
from copy import deepcopy
import json
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest

from generate_validation_rust import BOOL, STRING, ROOT, INPUT, GenerationError, generate, main, rust_string

A, B, OBJECT = "model.A", "model.B", "org.eclipse.emf.ecore.EObject"
FLAG, CHILD = "model.A.getFlag()", "model.A.getChild()"


def local(name="subject", type_=A):
    return {"kind": "local", "type": type_, "name": name}


def string(value):
    return {"kind": "string", "type": STRING, "value": value}


def getter(symbol, type_):
    return {"kind": "get", "type": type_, "symbol": symbol, "receiver": local()}


def fixture():
    null = {"kind": "null", "type": "null"}
    diagnostic = {"op": "diagnostic", "severity": "warning", "message": string('quote " slash \\ newline\n\t\0\b\f é🛰'), "subject": local(), "feature": null, "code": string("fixtureCode"), "index": None, "data": [string("detail") ]}
    return {
        "schema_version": 1, "source_format": "typed-validation-rules", "source": {},
        "types": {A: {"kind": "JvmGenericType", "supertypes": [OBJECT]}, B: {"kind": "JvmGenericType", "supertypes": [OBJECT]}, OBJECT: {"kind": "JvmGenericType", "supertypes": []}},
        "dependencies": [{"symbol": FLAG, "receiver_type": A, "result_type": BOOL}, {"symbol": CHILD, "receiver_type": A, "result_type": A}],
        "rules": [{"id": "example.Validator#checkA", "name": "checkA", "declaring_type": "example.Validator", "source": {}, "parameter": {"name": "subject", "type": A}, "dependencies": [FLAG, CHILD], "body": [
            {"op": "block", "body": [{"op": "let", "name": "shared", "type": STRING, "value": string("first")}]},
            {"op": "block", "body": [{"op": "let", "name": "shared", "type": STRING, "value": string("second")}]},
            {"op": "let", "name": "shared", "type": STRING, "value": string("outer")},
            {"op": "if", "condition": {"kind": "and", "type": BOOL, "left": {"kind": "not", "type": BOOL, "operand": getter(FLAG, BOOL)}, "right": {"kind": "identity_ne", "type": BOOL, "left": getter(CHILD, A), "right": null}}, "then": [diagnostic], "else": []},
        ]}],
    }


def emit(document):
    return generate(document, "a" * 64, "b" * 64)


class RustBackendTests(unittest.TestCase):
    def setUp(self):
        self.document = fixture()

    def reject(self, pattern):
        with self.assertRaisesRegex(GenerationError, pattern):
            emit(self.document)

    def test_real_rules_emit_direct_stable_code_and_provenance(self):
        document = json.loads((ROOT / INPUT).read_text(encoding="utf-8"))
        source, manifest = emit(document)
        self.assertEqual((source, manifest), emit(document))
        text = source.decode("utf-8")
        self.assertIn("if ", text)
        self.assertIn(" && ", text)
        self.assertIn(" || ", text)
        self.assertNotIn("Expression::", text)
        self.assertNotIn("serde_json", text)
        self.assertNotIn("BTreeMap", text)
        metadata = json.loads(manifest)
        self.assertEqual(metadata["backend"], "rust-source")
        self.assertEqual(len(metadata["rules"]), 11)
        for rule in metadata["rules"]:
            self.assertEqual(text.count(rule["entrypoint"]), 1)

    def test_predicate_mutation_changes_actual_rust(self):
        first, _ = emit(self.document)
        self.document["rules"][0]["body"][-1]["condition"]["kind"] = "or"
        second, _ = emit(self.document)
        self.assertNotEqual(first, second)
        self.assertIn(b" || ", second)

    def test_identifiers_are_numbered_not_interpolated(self):
        injection = 'x; panic!("injected"); //'
        self.document["rules"][0]["body"][0]["body"][0]["name"] = injection
        source, _ = emit(self.document)
        self.assertNotIn(injection.encode(), source)
        self.assertIn(b"let local_0 =", source)

    def test_rust_string_escaping_uses_rust_unicode_syntax(self):
        self.assertEqual(rust_string('"\\\n\t\0\b\f é🛰'), '"\\"\\\\\\n\\t\\0\\u{8}\\u{c} \\u{e9}\\u{1f6f0}"')
        with self.assertRaisesRegex(GenerationError, "surrogate"):
            rust_string("\ud800")

    def test_unknown_op_fails(self):
        self.document["rules"][0]["body"][0] = {"op": "evaluate_java", "body": []}
        self.reject("Unsupported statement operation")

    def test_unknown_expression_fails(self):
        self.document["rules"][0]["body"][-1]["condition"]["kind"] = "opaque_predicate"
        self.reject("Unsupported expression kind")

    def test_unknown_type_fails(self):
        self.document["rules"][0]["body"][-1]["condition"]["left"]["operand"]["type"] = "model.Unknown"
        self.reject("Unknown expression type")

    def test_incompatible_known_receiver_fails(self):
        self.document["dependencies"][0]["receiver_type"] = B
        self.reject("Incompatible getter receiver")

    def test_inherited_receiver_is_accepted(self):
        self.document["types"][A]["supertypes"] = [B]
        self.document["dependencies"][0]["receiver_type"] = B
        emit(self.document)

    def test_unknown_getter_fails(self):
        self.document["rules"][0]["body"][-1]["condition"]["left"]["operand"]["symbol"] = "model.A.unknown()"
        self.reject("Undeclared getter dependency")

    def test_getter_result_type_must_match_contract(self):
        self.document["dependencies"][0]["result_type"] = STRING
        self.reject("Type mismatch")

    def test_unused_declared_getter_fails(self):
        self.document["dependencies"].append({"symbol": "model.A.unused()", "receiver_type": A, "result_type": BOOL})
        self.document["rules"][0]["dependencies"].append("model.A.unused()")
        self.reject("Unused declared getter")

    def test_duplicate_local_and_escaped_local_fail(self):
        self.document["rules"][0]["body"].insert(3, deepcopy(self.document["rules"][0]["body"][2]))
        self.reject("Duplicate or shadowing immutable local")
        self.document = fixture()
        del self.document["rules"][0]["body"][2]
        self.document["rules"][0]["body"][-1]["then"][0]["message"] = local("shared", STRING)
        self.reject("Unbound local")

    def test_primitive_identity_fails(self):
        self.document["rules"][0]["body"][-1]["condition"]["right"]["left"] = getter(FLAG, BOOL)
        self.reject("Identity comparison requires")

    def test_unknown_rule_field_fails(self):
        self.document["rules"][0]["guard"] = True
        self.reject("Malformed rule")

    def test_unknown_expression_field_fails(self):
        self.document["rules"][0]["body"][-1]["condition"]["left"]["operand"]["null_safe"] = True
        self.reject("Malformed getter expression")

    def test_ambiguous_short_rule_names_fail(self):
        second = deepcopy(self.document["rules"][0])
        second.update(id="other.Validator#checkA", declaring_type="other.Validator")
        self.document["rules"].append(second)
        self.reject("Duplicate short rule name")

    def test_null_diagnostic_strings_fail(self):
        self.document["rules"][0]["body"][-1]["then"][0]["message"] = {"kind": "null", "type": "null"}
        self.reject("Type mismatch")

    def test_diagnostic_expressions_follow_xtend_argument_order(self):
        diagnostic = self.document["rules"][0]["body"][-1]["then"][0]
        for key, type_ in [("message", STRING), ("subject", A), ("feature", A), ("code", STRING)]:
            symbol = "model.A.get" + key.title() + "()"
            self.document["dependencies"].append({"symbol": symbol, "receiver_type": A, "result_type": type_})
            self.document["rules"][0]["dependencies"].append(symbol)
            diagnostic[key] = getter(symbol, type_)
        source, _ = emit(self.document)
        body = source.decode("utf-8").split("let mut diagnostics", 1)[1]
        positions = [body.index("read_getter(subject_value.clone(), \"model.A.get" + key.title() + "()") for key in ["message", "subject", "feature", "code"]]
        self.assertEqual(positions, sorted(positions))

    def test_check_rejects_modified_generated_source_and_manifest(self):
        with tempfile.TemporaryDirectory(prefix="rust generation controls ") as directory:
            root = Path(directory)
            (root / "rules.json").write_text(json.dumps(self.document), encoding="utf-8")
            args = ["--root", str(root), "--input", "rules.json", "--out", "generated.rs", "--manifest", "generation.json"]
            main(args)
            main([*args, "--check"])
            source = root / "generated.rs"
            source.write_bytes(source.read_bytes() + b"// changed\n")
            with self.assertRaisesRegex(GenerationError, "Stale generated Rust"):
                main([*args, "--check"])
            main(args)
            provenance = root / "generation.json"
            provenance.write_bytes(provenance.read_bytes() + b" ")
            with self.assertRaisesRegex(GenerationError, "Stale generated Rust"):
                main([*args, "--check"])

    @unittest.skipUnless(shutil.which("rustc"), "Rust compiler is required for generated-code execution control")
    def test_generated_rust_compiles_and_executes_scope_short_circuit_and_error_controls(self):
        source, _ = emit(self.document)
        harness = r'''
#![allow(dead_code, unused_variables, unused_imports)]
use std::cell::RefCell;
#[derive(Clone, Debug, PartialEq)]
enum Value { Null, Bool(bool), String(String), Enum { type_name:String, name:String }, Object { id:String, type_name:String } }
impl Value {
    fn object(id:impl Into<String>, ty:impl Into<String>) -> Self { Self::Object{id:id.into(),type_name:ty.into()} }
    fn boolean(self)->Result<bool,String> { if let Self::Bool(v)=self {Ok(v)}else{Err("not bool".into())} }
    fn string(self)->Result<String,String> { if let Self::String(v)=self {Ok(v)}else{Err("not string".into())} }
    fn optional_identity(self)->Result<Option<String>,String> { match self {Self::Null=>Ok(None),Self::Object{id,..}=>Ok(Some(id)),_=>Err("not object".into())} }
}
#[derive(Debug)]
struct RuleDiagnostic { severity:String,message:String,code:String,subject:Option<String>,feature:Option<String>,index:Option<i64>,data:Vec<String> }
trait SemanticModel { fn supports_getter(&self,s:&str)->bool; fn get(&self,id:&str,s:&str)->Result<Value,String>; }
fn require_getters(_: &str,deps:&[&str],m:&impl SemanticModel)->Result<(),String> {for d in deps {if !m.supports_getter(d){return Err("missing".into())}}Ok(())}
fn read_getter(receiver:Value,s:&str,m:&impl SemanticModel)->Result<Value,String> {if let Value::Object{id,..}=receiver {m.get(&id,s)}else{Err("null receiver".into())}}
fn identity_equal(a:Value,b:Value)->Result<bool,String> {Ok(match(a,b){(Value::Null,Value::Null)=>true,(Value::Object{id:a,..},Value::Object{id:b,..})=>a==b,_=>false})}
struct Model { flag:bool, fail_child:bool, support_child:bool, calls:RefCell<Vec<String>> }
impl SemanticModel for Model {
 fn supports_getter(&self,s:&str)->bool {s=="model.A.getFlag()" || (self.support_child && s=="model.A.getChild()")}
 fn get(&self,_:&str,s:&str)->Result<Value,String> {
  self.calls.borrow_mut().push(s.into());
  match s {"model.A.getFlag()"=>Ok(Value::Bool(self.flag)),"model.A.getChild()"=>if self.fail_child{Err("child failure".into())}else{Ok(Value::object("child","model.A"))},_=>panic!("unexpected getter")}
 }
}
fn model(flag:bool, fail_child:bool, support_child:bool)->Model {Model{flag,fail_child,support_child,calls:RefCell::new(vec![])}}
mod generated { include!("generated.rs"); }
fn main() {
 let m=model(false,false,true);
 let diagnostics=generated::evaluate("checkA","root",&m).unwrap();
 assert_eq!(diagnostics.len(),1);
 let d=&diagnostics[0];
 assert_eq!(d.message,concat!("quote \" slash \\"," newline\n\t\0\u{8}\u{c} é🛰"));
 assert_eq!(d.subject.as_deref(),Some("root")); assert_eq!(d.severity,"warning"); assert_eq!(d.code,"fixtureCode"); assert_eq!(d.data,vec!["detail"]);
 assert!(d.feature.is_none() && d.index.is_none());
 let guarded=model(true,true,true); assert!(generated::evaluate("checkA","root",&guarded).unwrap().is_empty()); assert_eq!(guarded.calls.borrow().as_slice(),["model.A.getFlag()"]);
 let missing=model(true,true,false); assert_eq!(generated::evaluate("checkA","root",&missing).unwrap_err(),"missing"); assert!(missing.calls.borrow().is_empty());
 let failed=model(false,true,true); assert_eq!(generated::evaluate("checkA","root",&failed).unwrap_err(),"child failure");
 assert!(generated::evaluate("unknown","root",&m).is_err());
 println!("generated Rust controls passed");
}
'''
        with tempfile.TemporaryDirectory(prefix="compiled validation controls ") as directory:
            root = Path(directory)
            (root / "generated.rs").write_bytes(source)
            (root / "main.rs").write_text(harness, encoding="utf-8")
            binary = root / "generated-controls.exe"
            result = subprocess.run([shutil.which("rustc"), "--edition=2024", str(root / "main.rs"), "-o", str(binary)], capture_output=True, text=True, timeout=60)
            self.assertEqual(result.returncode, 0, result.stderr)
            run = subprocess.run([str(binary)], capture_output=True, text=True, timeout=20)
            self.assertEqual(run.returncode, 0, run.stderr)
            self.assertIn("generated Rust controls passed", run.stdout)


if __name__ == "__main__":
    unittest.main()
