package dev.mercurio.pilot;
import com.google.gson.GsonBuilder;
import java.nio.file.*;
import java.io.StringReader;
import java.util.*;
import org.eclipse.xtext.*;
import org.eclipse.xtext.parser.IParser;
public final class PilotCommentProbe {
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
            for (String source : List.of("doc /**/", "doc D /* a\n b */", "doc D locale \"en\" /* a /* b */", "\"text\"", "language \"text\" /**/", "rep language \"text\" /* raw */", "rep R language \"text\" /* a\n b */", "rep <short> R language \"text\\tplain\" /* a /* b */", "rep <short> language \"text\" /* body */", "/**/", "locale \"en\" /* body */", "comment /**/", "comment C /* body */", "comment <s> C about P::A, 'B.C' locale \"en\" /* body */", "comment about A, A /* repeat */")) {
                boolean textual = source.startsWith("rep") || source.startsWith("language");
                boolean comment = source.startsWith("comment") || source.startsWith("locale") || source.startsWith("/*");
                var result = parser.parse((ParserRule)GrammarUtil.findRuleForName(grammar, comment ? "Comment" : textual ? "TextualRepresentation" : source.startsWith("doc") ? "Documentation" : "LiteralString"), new StringReader(source));
                if (result.hasSyntaxErrors()) throw new IllegalStateException("Probe parse failed: " + source);
                var object = result.getRootASTElement();
                var row = new TreeMap<String,Object>();
                row.put("language", language);
                row.put("source", source);
                row.put("kind", object.eClass().getName());
                for (String field : comment ? List.of("body", "locale", "declaredName", "declaredShortName") : textual ? List.of("body", "language", "declaredName", "declaredShortName") : source.startsWith("doc") ? List.of("body", "locale", "declaredName") : List.of("value"))
                    row.put(field.equals("language") ? "languageValue" : field, object.eGet(object.eClass().getEStructuralFeature(field)));
                if (comment) {
                    var references = new ArrayList<Object>();
                    var owned = (List<?>) object.eGet(object.eClass().getEStructuralFeature("ownedRelationship"));
                    for (Object entry : owned) {
                        var annotation = (org.eclipse.emf.ecore.EObject) entry;
                        var feature = annotation.eClass().getEStructuralFeature("annotatedElement");
                        var nodes = org.eclipse.xtext.nodemodel.util.NodeModelUtils.findNodesForFeature(annotation, feature);
                        if (nodes.size() != 1) throw new IllegalStateException("Missing target node");
                        var node = nodes.get(0);
                        references.add(new TreeMap<>(Map.of("kind", annotation.eClass().getName(), "targetType", feature.getEType().getName(),
                            "spelling", org.eclipse.xtext.nodemodel.util.NodeModelUtils.getTokenText(node))));
                    }
                    row.put("references", references);
                }
                cases.add(row);
            }
            for(String body:List.of("", "two\nlines", "quoted \"text\" and 'names'", "/* nested opener")) {
                String source="package P { doc /*"+body+"*/ package Q { doc /*inside*/ } }";
                var result=parser.parse((ParserRule)GrammarUtil.findRuleForName(grammar,"RootNamespace"),new StringReader(source));
                if(result.hasSyntaxErrors())throw new IllegalStateException("Document comment parse failed: "+source);
                var bodies=new ArrayList<String>();var contents=result.getRootASTElement().eAllContents();
                while(contents.hasNext()) {
                    var item=contents.next();if(item.eClass().getName().equals("Documentation"))bodies.add((String)item.eGet(item.eClass().getEStructuralFeature("body")));
                }
                cases.add(Map.of("language",language,"source",source,"kind","RootNamespace","bodies",bodies));
            }
        }
        Files.writeString(Path.of(args[0]), new GsonBuilder().serializeNulls().setPrettyPrinting().create().toJson(cases) + "\n");
    }
}
