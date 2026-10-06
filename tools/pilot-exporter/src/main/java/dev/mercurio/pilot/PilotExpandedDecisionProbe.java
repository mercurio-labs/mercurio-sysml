package dev.mercurio.pilot;
import com.google.gson.GsonBuilder;
import java.nio.file.*;
import java.io.StringReader;
import java.util.*;
import org.eclipse.xtext.*;
import org.eclipse.xtext.parser.IParser;
import org.eclipse.emf.ecore.EObject;
/** Independent generated-parser node evidence; does not read exported DFA tables. */
public final class PilotExpandedDecisionProbe {
    static void probe(List<Object> cases, com.google.inject.Injector injector, String context,
            String entry, String ruleName, String path, String source, String input) {
        var grammar = injector.getInstance(IGrammarAccess.class).getGrammar();
        var rule = (ParserRule)GrammarUtil.findRuleForName(grammar, ruleName.split("#", 2)[0]);
        EObject node = rule.getAlternatives();
        String[] parts = path.isEmpty() ? new String[0] : path.split("/");
        for (int i = 0; i < parts.length; i++) {
            Object value = node.eGet(node.eClass().getEStructuralFeature(parts[i]));
            node = value instanceof List<?> list ? (EObject)list.get(Integer.parseInt(parts[++i])) : (EObject)value;
        }
        var branches = ((Alternatives)node).getElements();
        var result = injector.getInstance(IParser.class).parse(
            (ParserRule)GrammarUtil.findRuleForName(grammar, entry), new StringReader(source));
        if (result.hasSyntaxErrors()) throw new IllegalStateException("Rejected control: " + source);
        int selected = -1;
        for (var concrete : result.getRootNode().getAsTreeIterable()) {
            var element = concrete.getGrammarElement();
            while (element != null && element != rule) {
                int index = branches.indexOf(element);
                if (index >= 0) { selected = index; break; }
                element = element.eContainer();
            }
            if (selected >= 0) break;
        }
        if (selected < 0) throw new IllegalStateException("Missing branch evidence: " + source);
        var row = new TreeMap<String,Object>();
        row.put("context", context); row.put("decision", ruleName);
        row.put("source", source); row.put("input", input); row.put("alternative", selected);
        cases.add(row);
    }
    public static void main(String[] args) throws Exception {
        org.omg.sysml.logic.SysMLLogicStandaloneSetup.doSetup();
        org.eclipse.emf.ecore.EPackage.Registry.INSTANCE.put("https://www.omg.org/spec/SysML/20250201", org.omg.sysml.lang.sysml.SysMLPackage.eINSTANCE);
        var kerml = new org.omg.kerml.xtext.KerMLStandaloneSetup().createInjectorAndDoEMFRegistration();
        var sysml = new org.omg.sysml.xtext.SysMLStandaloneSetup().createInjectorAndDoEMFRegistration();
        var cases = new ArrayList<Object>();
        for (String body : List.of("a to b", "(a,b)", "c from a to b", "c(a,b,c)"))
            probe(cases,kerml,"org.omg.kerml.xtext.KerML","Connector","ConnectorDeclaration","","connector " + body + ";",body + ";");
        for (String body : List.of("class C;", "feature f;", "alias a for b;", "public import a::*;", "return feature r;"))
            probe(cases,kerml,"org.omg.kerml.xtext.KerML","Function","FunctionBodyPart","elements/0","function F { " + body + " }",body + " }");
        for (String body : List.of("part def P;", "part p;", "attribute def A;", "attribute a;"))
            probe(cases,sysml,"org.omg.sysml.xtext.SysML","PackageMember","PackageMember","elements/1",body,body);
        for (String body : List.of("{}", "if false {}"))
            probe(cases,sysml,"org.omg.sysml.xtext.SysML","IfNode","IfNode","elements/4/elements/1/terminal","if true {} else " + body,body);
        for (String body : List.of("x", "after 1", "when true"))
            probe(cases,sysml,"org.omg.sysml.xtext.SysML","PayloadParameter","PayloadParameter","",body,body);
        for (String body : List.of("a", "accept x", "send x", "assign x := 1"))
            probe(cases,sysml,"org.omg.sysml.xtext.SysML","PerformedActionUsage","PerformedActionUsage","",body,body);
        for (String body : List.of(";", "{}", "x;", "x to y;", "via x;", "to y;"))
            probe(cases,sysml,"org.omg.sysml.xtext.SysML","SendNode","SendNode#/elements/3","elements/3","send " + body,body);
        for (String body : List.of(";", "{}", "x;", "1;", "x + y;", "'end';"))
            probe(cases,sysml,"org.omg.sysml.xtext.SysML","TerminateNode","TerminateNode","elements/3","terminate " + body,body);
        for (String body : List.of(";", "{}", "{ return r; }"))
            probe(cases,sysml,"org.omg.sysml.xtext.SysML","CalculationDefinition","CalculationBody","","calc def C " + body,body);
        var names = new ArrayList<Object>();
        for (var injector : List.of(kerml, sysml)) {
            boolean kernel = injector == kerml;
            var grammar = injector.getInstance(IGrammarAccess.class).getGrammar();
            for (String spelling : List.of("plain", "'action'", "'a b'", "'é'", "''",
                    "'a\\bb'", "'a\\tb'", "'a\\nb'", "'a\\fb'", "'a\\rb'",
                    "'a\\\"b'", "'a\\'b'", "'a\\\\b'")) {
                String source = (kernel ? "feature " : "ref ") + spelling + ";";
                var result = injector.getInstance(IParser.class).parse(
                    (ParserRule)GrammarUtil.findRuleForName(grammar, kernel ? "Feature" : "ReferenceUsage"), new StringReader(source));
                if (result.hasSyntaxErrors()) throw new IllegalStateException("Rejected name: " + source);
                var element = (org.omg.sysml.lang.sysml.Element)result.getRootASTElement();
                var row = new TreeMap<String,Object>();
                row.put("kerml", kernel); row.put("spelling", spelling);
                row.put("raw_declared_name", element.getDeclaredName());
                row.put("represented_name", org.omg.sysml.util.ElementUtil.unescapeString(element.getDeclaredName()));
                names.add(row);
            }
        }
        Files.writeString(Path.of(args[0] + ".names.json"), new GsonBuilder().setPrettyPrinting().create().toJson(names) + "\n");
        Files.writeString(Path.of(args[0]), new GsonBuilder().setPrettyPrinting().create().toJson(cases) + "\n");
    }
}
