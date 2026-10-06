package dev.mercurio.pilot;
import com.google.gson.GsonBuilder;
import java.nio.file.*;
import java.io.StringReader;
import java.util.*;
import org.eclipse.xtext.*;
import org.eclipse.xtext.parser.IParser;
import org.antlr.runtime.*;
public final class PilotExpressionDecisionProbe {
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
            for (String ruleName : List.of("BaseExpression", "ClassificationExpression")) {
            var rule = (ParserRule)GrammarUtil.findRuleForName(grammar, ruleName);
            var branches = ((Alternatives)rule.getAlternatives()).getElements();
            for (String source : ruleName.equals("BaseExpression") ? List.of("a", "a.b", "a::b", "'a.b'", "f()", "f(1)", "f(x=1)",
                    "a.b()", "a.metadata", "new T()", "{}", "(1)", "()", "null",
                    "12", "1e3", "1.25e-2", "\"text\"", "true", "*") : List.of("1", "a hastype T", "a istype T",
                    "a @ T", "hastype T", "istype T", "@ T", "a as T", "as T",
                    "T @@ U", "T meta U", "a + b hastype T", "a.b(1) as T")) {
                var result = parser.parse(rule, new StringReader(source));
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
                if (selected < 0) throw new IllegalStateException("Missing parser decision evidence");
                Lexer lexer = language.equals("kerml")
                    ? new org.omg.kerml.xtext.parser.antlr.internal.InternalKerMLLexer(new ANTLRStringStream(source))
                    : new org.omg.sysml.xtext.parser.antlr.internal.InternalSysMLLexer(new ANTLRStringStream(source));
                String[] names = language.equals("kerml")
                    ? org.omg.kerml.xtext.parser.antlr.internal.InternalKerMLParser.tokenNames
                    : org.omg.sysml.xtext.parser.antlr.internal.InternalSysMLParser.tokenNames;
                var tokens = new ArrayList<String>();
                for (Token token = lexer.nextToken(); token.getType() != Token.EOF; token = lexer.nextToken()) {
                    String name = names[token.getType()];
                    if (List.of("RULE_WS", "RULE_ML_NOTE", "RULE_SL_NOTE").contains(name)) continue;
                    tokens.add(name.startsWith("RULE_") ? "terminal:" + name.substring(5)
                        : "keyword:" + token.getText());
                }
                tokens.add("eof");
                var row = new TreeMap<String,Object>();
                row.put("rule", ruleName); row.put("accepted", !result.hasSyntaxErrors()); row.put("language", language); row.put("source", source); row.put("alternative", selected);
                row.put("kind", result.getRootASTElement().eClass().getName()); row.put("tokens", tokens);
                cases.add(row);
            }
        }
        }
        Files.writeString(Path.of(args[0]), new GsonBuilder().setPrettyPrinting().create().toJson(cases) + "\n");
    }
}
