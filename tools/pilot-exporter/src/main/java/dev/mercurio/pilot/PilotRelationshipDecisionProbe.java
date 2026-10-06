package dev.mercurio.pilot;
import com.google.gson.GsonBuilder;
import java.nio.file.*;
import java.io.StringReader;
import java.util.*;
import org.eclipse.xtext.*;
import org.eclipse.xtext.parser.IParser;
public final class PilotRelationshipDecisionProbe {
    public static void main(String[] args) throws Exception {
        org.omg.sysml.logic.SysMLLogicStandaloneSetup.doSetup();
        org.eclipse.emf.ecore.EPackage.Registry.INSTANCE.put("https://www.omg.org/spec/SysML/20250201", org.omg.sysml.lang.sysml.SysMLPackage.eINSTANCE);
        var injector = new org.omg.kerml.xtext.KerMLStandaloneSetup().createInjectorAndDoEMFRegistration();
        var grammar = injector.getInstance(IGrammarAccess.class).getGrammar();
        var parser = injector.getInstance(IParser.class);
        var cases = new ArrayList<Object>();
        String[][] rules = {{"Specialization","subtype",":>"}, {"Conjugation","conjugate","~"},
            {"Disjoining","disjoint","from"}, {"FeatureInverting","inverse","of"},
            {"Subsetting","subset",":>"}, {"Redefinition","redefinition",":>>"}};
        String[][] endpoints = {{"a","b"}, {"a.b","c"}, {"a","b.c"}, {"a.b","c.d"},
            {"q::a","r::b.c"}, {"'a.b'","'c.d'"}};
        for (String[] entry : rules) for (String[] pair : endpoints) {
            var rule = (ParserRule)GrammarUtil.findRuleForName(grammar, entry[0]);
            String source = entry[1] + " " + pair[0] + " " + entry[2] + " " + pair[1] + ";";
            var result = parser.parse(rule, new StringReader(source));
            if (result.hasSyntaxErrors()) throw new IllegalStateException("Rejected control: " + source);
            for (int side = 0; side < 2; side++) {
                int position = side == 0 ? 2 : 4;
                var branches = ((Alternatives)((Group)rule.getAlternatives()).getElements().get(position)).getElements();
                int selected = -1;
                for (var node : result.getRootNode().getAsTreeIterable()) {
                    var element = node.getGrammarElement();
                    while (element != null && element != rule) {
                        int index = branches.indexOf(element);
                        if (index >= 0) { selected = index; break; }
                        element = element.eContainer();
                    }
                    if (selected >= 0) break;
                }
                if (selected < 0) throw new IllegalStateException("Missing endpoint evidence: " + source);
                var row = new TreeMap<String,Object>();
                row.put("source", source);
                row.put("decision", entry[0] + "#/elements/" + position);
                row.put("input", side == 0 ? pair[0] + " " + entry[2] + " " + pair[1] + ";" : pair[1] + ";");
                row.put("alternative", selected);
                cases.add(row);
            }
        }
        Files.writeString(Path.of(args[0]), new GsonBuilder().setPrettyPrinting().create().toJson(cases) + "\n");
    }
}
