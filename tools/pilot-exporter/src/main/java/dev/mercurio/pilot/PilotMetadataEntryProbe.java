package dev.mercurio.pilot;
import com.google.gson.GsonBuilder;
import java.nio.file.*;
import java.io.StringReader;
import java.util.*;
import org.eclipse.xtext.*;
import org.eclipse.xtext.parser.IParser;
import org.eclipse.xtext.nodemodel.util.NodeModelUtils;
public final class PilotMetadataEntryProbe {
    public static void main(String[] args) throws Exception {
        org.omg.sysml.logic.SysMLLogicStandaloneSetup.doSetup();
        org.eclipse.emf.ecore.EPackage.Registry.INSTANCE.put("https://www.omg.org/spec/SysML/20250201", org.omg.sysml.lang.sysml.SysMLPackage.eINSTANCE);
        var injector = new org.omg.kerml.xtext.KerMLStandaloneSetup().createInjectorAndDoEMFRegistration();
        var grammar = injector.getInstance(IGrammarAccess.class).getGrammar();
        var parser = injector.getInstance(IParser.class);
        var cases = new ArrayList<Object>();
        for (String source : List.of("T", "q::T", "$::q::T", "'T'", ": T", "typed by T",
                "m : T", "m typed by q::T", "<s> m : q::T", "<s> : T", "'m' : 'T'",
                "m :", "m typed T", "m typed by", "m T", "q::", "<s : T")) {
            var result = parser.parse((ParserRule)GrammarUtil.findRuleForName(grammar, "MetadataFeature"), new StringReader("@ " + source + ";"));
            var row = new TreeMap<String,Object>();
            row.put("source", source);
            row.put("accepted", !result.hasSyntaxErrors());
            if (!result.hasSyntaxErrors()) {
                var object = result.getRootASTElement();
                row.put("kind", object.eClass().getName());
                var fields = new TreeMap<String,Object>();
                for (String name : List.of("declaredName", "declaredShortName")) {
                    var value = object.eGet(object.eClass().getEStructuralFeature(name), false);
                    if (value != null) fields.put(name, value);
                }
                row.put("fields", fields);
                var children = new ArrayList<String>();
                var iterator = object.eAllContents();
                while (iterator.hasNext()) {
                    var child = iterator.next();
                    children.add(child.eClass().getName());
                    if (child.eClass().getName().equals("FeatureTyping")) {
                        var nodes = NodeModelUtils.findNodesForFeature(child, child.eClass().getEStructuralFeature("type"));
                        if (nodes.size() != 1) throw new IllegalStateException("Unexpected type reference nodes");
                        row.put("type_text", nodes.get(0).getText().trim());
                    }
                }
                row.put("contained_kinds", children);
            }
            cases.add(row);
        }
        Files.writeString(Path.of(args[0]), new GsonBuilder().setPrettyPrinting().create().toJson(cases) + "\n");
    }
}
