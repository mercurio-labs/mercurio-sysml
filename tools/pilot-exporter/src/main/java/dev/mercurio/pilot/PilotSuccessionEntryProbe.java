package dev.mercurio.pilot;
import com.google.gson.GsonBuilder;
import java.nio.file.*;
import java.io.StringReader;
import java.util.*;
import org.eclipse.xtext.*;
import org.eclipse.xtext.parser.IParser;
public final class PilotSuccessionEntryProbe {
    public static void main(String[] args) throws Exception {
        org.omg.sysml.logic.SysMLLogicStandaloneSetup.doSetup();
        org.eclipse.emf.ecore.EPackage.Registry.INSTANCE.put("https://www.omg.org/spec/SysML/20250201", org.omg.sysml.lang.sysml.SysMLPackage.eINSTANCE);
        var injector = new org.omg.sysml.xtext.SysMLStandaloneSetup().createInjectorAndDoEMFRegistration();
        var grammar = injector.getInstance(IGrammarAccess.class).getGrammar();
        var parser = injector.getInstance(IParser.class);
        var cases = new ArrayList<Object>();
        for (String source : List.of("then", "then [1]", "then [0]", "then [*]", "then [1..3]",
                "then [0..*]", "then [1,2]", "then []", "then [1..]", "then [", "then 2")) {
            var result = parser.parse((ParserRule)GrammarUtil.findRuleForName(grammar, "EmptySuccessionMember"), new StringReader(source));
            var row = new TreeMap<String,Object>();
            row.put("source", source);
            row.put("accepted", !result.hasSyntaxErrors());
            if (!result.hasSyntaxErrors()) {
                var object = result.getRootASTElement();
                row.put("kind", object.eClass().getName());
                var children = new ArrayList<String>();
                var values = new ArrayList<Object>();
                var iterator = object.eAllContents();
                while (iterator.hasNext()) {
                    var child = iterator.next();
                    children.add(child.eClass().getName());
                    if (child.eClass().getName().equals("LiteralInteger"))
                        values.add(child.eGet(child.eClass().getEStructuralFeature("value"), false));
                }
                row.put("contained_kinds", children);
                row.put("integer_values", values);
            }
            cases.add(row);
        }
        Files.writeString(Path.of(args[0]), new GsonBuilder().setPrettyPrinting().create().toJson(cases) + "\n");
    }
}
