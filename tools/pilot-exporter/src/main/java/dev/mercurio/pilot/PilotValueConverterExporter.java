package dev.mercurio.pilot;

import com.google.gson.GsonBuilder;
import java.nio.file.*;
import java.util.*;
import org.eclipse.xtext.*;
import org.eclipse.xtext.conversion.IValueConverterService;
import org.eclipse.xtext.conversion.impl.AbstractDeclarativeValueConverterService;
import org.eclipse.emf.ecore.EDataType;
import org.eclipse.emf.ecore.util.EcoreUtil;

/** Build-time resolution of the actual injected converter service. */
public final class PilotValueConverterExporter {
    public static void main(String[] args) throws Exception {
        org.omg.sysml.logic.SysMLLogicStandaloneSetup.doSetup();
        org.eclipse.emf.ecore.EPackage.Registry.INSTANCE.put("https://www.omg.org/spec/SysML/20250201", org.omg.sysml.lang.sysml.SysMLPackage.eINSTANCE);
        var injectors = List.of(
            new org.omg.kerml.xtext.KerMLStandaloneSetup().createInjectorAndDoEMFRegistration(),
            new org.omg.sysml.xtext.SysMLStandaloneSetup().createInjectorAndDoEMFRegistration());
        var bindings = new ArrayList<Object>();
        var controls = new ArrayList<Object>();
        var numericModels = new ArrayList<Object>();
        for (var injector : injectors) {
            var grammar = injector.getInstance(IGrammarAccess.class).getGrammar();
            var service = (AbstractDeclarativeValueConverterService)injector.getInstance(IValueConverterService.class);
            var parser = injector.getInstance(org.eclipse.xtext.parser.IParser.class);
            var entry = (ParserRule)GrammarUtil.findRuleForName(grammar, "LiteralReal");
            for (String source : List.of("1.5", ".5", "2e3", " 1.5 ", "1.5 // trailing", "1.5e-2", "1 //*note*/ . 5", "1 . 5", ". 5", "1 . 5e2", "1 /*note*/ . 5", "1 // note\n . 5", "1 2 . 5", "1 . 2 3", "1 .", "1 . e2", "1 .. 5", "1 .5")) {
                var result = parser.parse(entry, new java.io.StringReader(source));
                var row = new TreeMap<String,Object>();
                row.put("context", grammar.getName()); row.put("source", source);
                row.put("accepted", !result.hasSyntaxErrors());
                var errors = new ArrayList<String>();
                for (var error : result.getSyntaxErrors()) errors.add(error.getSyntaxErrorMessage().getMessage());
                row.put("errors", errors);
                if (!result.hasSyntaxErrors()) {
                    var object = result.getRootASTElement();
                    row.put("value", object.eGet(object.eClass().getEStructuralFeature("value")));
                }
                numericModels.add(row);
            }
            for (var rule : GrammarUtil.allRules(grammar)) {
                if (rule.getType() == null || !(rule.getType().getClassifier() instanceof EDataType type)) continue;
                var converter = service.getConverter(rule.getName());
                var row = new TreeMap<String,Object>();
                row.put("context", grammar.getName());
                row.put("rule", ((Grammar)rule.eContainer()).getName() + "::" + rule.getName());
                row.put("datatype", EcoreUtil.getURI(type).toString());
                row.put("service", service.getClass().getName());
                row.put("converter", converter.getClass().getName());
                bindings.add(row);
                List<String> samples;
                if (type instanceof org.eclipse.emf.ecore.EEnum enumeration) {
                    samples = enumeration.getELiterals().stream().map(literal -> literal.getLiteral()).toList();
                } else samples = switch(type.getName()) {
                    case "EInt" -> List.of("0", "2147483647", "2147483648");
                    case "EDouble" -> List.of(".5", "1.25e2", "2E-3", "1e-999", "1e999");
                    case "EBoolean" -> rule.getName().equals("Nonunique") ? List.of("nonunique") : List.of("true", "false");
                    default -> List.of("plain", "\"a\\nb\"", "'ÃƒÆ’Ã‚Â©'", "/* body */");
                };
                for (var sample : samples) {
                    var control = new TreeMap<String,Object>();
                    control.put("context", grammar.getName());
                    control.put("rule", row.get("rule"));
                    control.put("input", sample);
                    try {
                        Object value = converter.toValue(sample, null);
                        if (value instanceof Double number && !Double.isFinite(number)) {
                            control.put("outcome", "nonfinite");
                            control.put("value", number.toString());
                        } else {
                            control.put("outcome", "value");
                            control.put("value", value instanceof org.eclipse.emf.common.util.Enumerator literal ? literal.getLiteral() : value);
                        }
                    } catch (org.eclipse.xtext.conversion.ValueConverterException ex) {
                        control.put("outcome", "rejected");
                    }
                    controls.add(control);
                }
            }
        }
        Files.writeString(Path.of(args[0]), new GsonBuilder().setPrettyPrinting().create().toJson(Map.of("bindings", bindings, "controls", controls, "numeric_models", numericModels)));
    }
}
