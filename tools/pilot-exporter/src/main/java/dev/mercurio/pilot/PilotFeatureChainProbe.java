package dev.mercurio.pilot;
import com.google.gson.GsonBuilder;
import java.nio.file.*;
import java.io.StringReader;
import java.util.*;
import org.eclipse.xtext.*;
import org.eclipse.xtext.parser.IParser;
public final class PilotFeatureChainProbe {
    public static void main(String[] args) throws Exception {
        org.omg.sysml.logic.SysMLLogicStandaloneSetup.doSetup();
        org.eclipse.emf.ecore.EPackage.Registry.INSTANCE.put("https://www.omg.org/spec/SysML/20250201", org.omg.sysml.lang.sysml.SysMLPackage.eINSTANCE);
        var cases = new ArrayList<Object>();
        for (String language : List.of("kerml", "sysml")) {
            var injector = language.equals("kerml")
                ? new org.omg.kerml.xtext.KerMLStandaloneSetup().createInjectorAndDoEMFRegistration()
                : new org.omg.sysml.xtext.SysMLStandaloneSetup().createInjectorAndDoEMFRegistration();
            var grammar = injector.getInstance(IGrammarAccess.class).getGrammar();
            var parser = injector.getInstance(IParser.class);
            for (String ruleName : List.of("FeatureChainMember", "InstantiatedTypeMember"))
            for (String source : List.of("a", "a::b", "$::a::b", "'a.b'", "a.b",
                    "a::b.c::d", "a.b.c", "'a.b'.'c::d'", "q::".repeat(40) + "a.b",
                    "a.", "a::", "a.(")) {
                var result = parser.parse((ParserRule)GrammarUtil.findRuleForName(grammar, ruleName), new StringReader(source));
                var row = new TreeMap<String,Object>();
                row.put("language", language);
                row.put("rule", ruleName);
                row.put("source", source);
                row.put("accepted", !result.hasSyntaxErrors());
                if (!result.hasSyntaxErrors()) {
                    var object = result.getRootASTElement();
                    row.put("kind", object.eClass().getName());
                    var children = new ArrayList<String>();
                    var iterator = object.eAllContents();
                    while (iterator.hasNext()) children.add(iterator.next().eClass().getName());
                    row.put("contained_kinds", children);
                }
                cases.add(row);
            }
        }
        Files.writeString(Path.of(args[0]), new GsonBuilder().setPrettyPrinting().create().toJson(cases) + "\n");
    }
}
