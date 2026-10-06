package dev.mercurio.pilot;

import com.google.gson.GsonBuilder;
import com.google.inject.Injector;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.security.MessageDigest;
import java.util.ArrayList;
import java.util.Comparator;
import java.util.HexFormat;
import java.util.HashSet;
import java.util.Set;
import java.util.IdentityHashMap;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import java.util.TreeMap;
import org.eclipse.emf.common.util.URI;
import org.eclipse.emf.ecore.EClassifier;
import org.eclipse.emf.ecore.EEnumLiteral;
import org.eclipse.emf.ecore.EObject;
import org.eclipse.emf.ecore.EPackage;
import org.eclipse.emf.ecore.EReference;
import org.eclipse.emf.ecore.EStructuralFeature;
import org.eclipse.emf.ecore.EcorePackage;
import org.eclipse.emf.ecore.resource.Resource;
import org.eclipse.emf.ecore.util.EcoreUtil;
import org.eclipse.emf.ecore.xmi.impl.EcoreResourceFactoryImpl;
import org.eclipse.xtext.AbstractRule;
import org.eclipse.xtext.Grammar;
import org.eclipse.xtext.GrammarUtil;
import org.eclipse.xtext.RuleCall;
import org.eclipse.xtext.XtextPackage;
import org.eclipse.xtext.XtextStandaloneSetup;
import org.eclipse.xtext.nodemodel.INode;
import org.eclipse.xtext.nodemodel.util.NodeModelUtils;
import org.eclipse.xtext.resource.XtextResource;
import org.eclipse.xtext.resource.XtextResourceSet;

/** Development-time export of raw Pilot grammars through Xtext's own parser and linker. */
public final class PilotGrammarExporter {
    private static final String[] SOURCES = {
        "org.omg.kerml.expressions.xtext/src/org/omg/kerml/expressions/xtext/KerMLExpressions.xtext",
        "org.omg.kerml.xtext/src/org/omg/kerml/xtext/KerML.xtext",
        "org.omg.sysml.xtext/src/org/omg/sysml/xtext/SysML.xtext"
    };
    private static final String METAMODEL = "org.omg.sysml/model/SysML.ecore";
    private final Map<EObject, String> identities = new IdentityHashMap<>();
    private final Map<String, Integer> counts = new TreeMap<>();
    private final Set<String> identityNames = new HashSet<>();
    private int referenceCount;
    private int inferredNodeCount;

    private PilotGrammarExporter() {}

    public static void main(String[] args) throws Exception {
        if (args.length != 2) throw new IllegalArgumentException("Usage: PilotGrammarExporter PILOT_ROOT OUTPUT_JSON");
        new PilotGrammarExporter().export(Path.of(args[0]).toAbsolutePath().normalize(), Path.of(args[1]));
    }

    private void export(Path root, Path output) throws Exception {
        Injector injector = new XtextStandaloneSetup().createInjectorAndDoEMFRegistration();
        List<Object> exported = new ArrayList<>();
        Map<String, Object> contexts = new TreeMap<>();
        for (String path : SOURCES) {
            // Xtext rewrites inherited calls for the loaded language. Sharing a
            // ResourceSet lets the last sibling language contaminate the others.
            XtextResourceSet resources = injector.getInstance(XtextResourceSet.class);
            resources.setClasspathURIContext(PilotGrammarExporter.class);
            resources.getResourceFactoryRegistry().getExtensionToFactoryMap().put("ecore", new EcoreResourceFactoryImpl());
            resources.getPackageRegistry().put(EcorePackage.eNS_URI, EcorePackage.eINSTANCE);
            Resource modelResource = resources.getResource(URI.createFileURI(root.resolve(METAMODEL).toString()), true);
            if (!modelResource.getErrors().isEmpty()) throw new IllegalStateException(modelResource.getErrors().toString());
            for (EObject object : modelResource.getContents()) {
                if (!(object instanceof EPackage model)) throw new IllegalStateException("Non-package metamodel root");
                registerPackage(resources, model);
            }
            for (String sourcePath : SOURCES) {
                String classpath = sourcePath.substring(sourcePath.indexOf("/src/") + 5);
                resources.getURIConverter().getURIMap().put(URI.createURI("classpath:/" + classpath), URI.createFileURI(root.resolve(sourcePath).toString()));
            }
            Resource resource = resources.getResource(URI.createFileURI(root.resolve(path).toString()), true);
            if (!(resource instanceof XtextResource xtext) || xtext.getParseResult() == null || xtext.getParseResult().hasSyntaxErrors()) {
                throw new IllegalStateException("Xtext parse failed: " + path + " " + resource.getErrors());
            }
            if (resource.getContents().size() != 1 || !(resource.getContents().get(0) instanceof Grammar grammar)) {
                throw new IllegalStateException("Expected exactly one grammar in " + path);
            }
            EcoreUtil.resolveAll(resources);
            for (Resource linked : resources.getResources()) {
                if (!linked.getErrors().isEmpty()) throw new IllegalStateException("Grammar link failed: " + linked.getErrors());
            }
            identities.clear();
            identityNames.clear();
            index(grammar, grammar.getName());
            for (Grammar inherited : GrammarUtil.allUsedGrammars(grammar)) {
                if (!identities.containsKey(inherited)) index(inherited, inherited.getName());
            }
            Map<String, String> effective = new TreeMap<>();
            Map<String, String> calls = new TreeMap<>();
            Map<String, Object> firstSets = new TreeMap<>();
            Map<String, String> hoisted = new TreeMap<>();
            var grammarAccess = new org.eclipse.xtext.xtext.generator.grammarAccess.GrammarAccessExtensions();
            for (AbstractRule rule : GrammarUtil.allRules(grammar)) {
                String identity = identities.get(rule);
                if (identity == null || effective.put(rule.getName(), identity) != null) {
                    throw new IllegalStateException("Ambiguous effective rule " + rule.getName());
                }
                var children = rule.eAllContents();
                while (children.hasNext()) {
                    EObject child = children.next();
                    if (child instanceof org.eclipse.xtext.AbstractElement element && element.isFirstSetPredicated()) {
                        var symbols = new ArrayList<String>();
                        for (var first : org.eclipse.xtext.xtext.generator.parser.antlr.FirstSetComputer.getFirstSet(element)) {
                            if (first instanceof org.eclipse.xtext.Keyword keyword) symbols.add("keyword:" + keyword.getValue());
                            else if (first instanceof RuleCall call && call.getRule() instanceof org.eclipse.xtext.TerminalRule)
                                symbols.add("call:" + call.getRule().getName());
                            else throw new IllegalStateException("Unsupported resolved first-set element " + first);
                        }
                        if (symbols.isEmpty()) throw new IllegalStateException("Empty first-set predicate");
                        firstSets.put(identities.get(element), symbols);
                    }
                    if (child instanceof RuleCall call) {
                        if (!call.isPredicated() && !call.isFirstSetPredicated() && grammarAccess.predicated(call)) {
                            if (!call.getArguments().isEmpty()) throw new IllegalStateException("Parameterized hoisted predicate requires substitution export");
                            var guard = grammarAccess.predicatedElement(call);
                            String guarded = identities.get(guard);
                            if (guarded == null) throw new IllegalStateException("Unresolved hoisted predicate target");
                            hoisted.put(identities.get(call), guarded);
                        }
                        String target = identities.get(call.getRule());
                        if (target == null) throw new IllegalStateException("Unexported contextual rule call");
                        calls.put(identities.get(call), target);
                    }
                }
            }
            Map<String, Object> context = new TreeMap<>();
            context.put("effective_rules", effective);
            context.put("rule_calls", calls);
            context.put("first_set_predicates", firstSets);
            context.put("hoisted_predicates", hoisted);
            contexts.put(grammar.getName(), context);
            int before = counts.values().stream().mapToInt(Integer::intValue).sum();
            Map<String, Object> item = node(grammar);
            int localNodes = 1;
            var localContents = grammar.eAllContents();
            while (localContents.hasNext()) { localContents.next(); localNodes++; }
            if (counts.values().stream().mapToInt(Integer::intValue).sum() - before != localNodes) {
                throw new IllegalStateException("Incomplete containment export");
            }
            @SuppressWarnings("unchecked") Map<String, Object> fields = (Map<String, Object>) item.get("fields");
            item.put("name", grammar.getName());
            item.put("source", source(root, path));
            item.put("rules", fields.remove("rules"));
            exported.add(item);
        }
        Map<String, Object> document = new LinkedHashMap<>();
        document.put("schema_version", 1);
        document.put("parser", "org.eclipse.xtext.XtextStandaloneSetup");
        document.put("source_format", "raw-xtext");
        document.put("span_encoding", "utf-16-code-units");
        document.put("metamodel", source(root, METAMODEL));
        document.put("grammars", exported);
        document.put("resolution_contexts", contexts);
        Map<String, Object> coverage = new LinkedHashMap<>();
        coverage.put("node_kinds", counts);
        coverage.put("nodes", counts.values().stream().mapToInt(Integer::intValue).sum());
        coverage.put("references", referenceCount);
        coverage.put("inferred_nodes_without_source_span", inferredNodeCount);
        coverage.put("unresolved_references", 0);
        coverage.put("unsupported_nodes", 0);
        document.put("coverage", coverage);
        String json = new GsonBuilder().serializeNulls().disableHtmlEscaping().setPrettyPrinting().create().toJson(document) + "\n";
        Files.createDirectories(output.toAbsolutePath().getParent());
        Files.writeString(output, json, StandardCharsets.UTF_8);
        System.out.println("Exported " + exported.size() + " raw Xtext grammars, " + coverage.get("nodes") + " nodes and " + referenceCount + " resolved references");
    }

    private static void registerPackage(XtextResourceSet resources, EPackage model) {
        resources.getPackageRegistry().put(model.getNsURI(), model);
        for (EPackage child : model.getESubpackages()) registerPackage(resources, child);
    }

    private static Map<String, Object> source(Path root, String path) throws Exception {
        Map<String, Object> source = new LinkedHashMap<>();
        source.put("path", path);
        source.put("sha256", HexFormat.of().formatHex(MessageDigest.getInstance("SHA-256").digest(Files.readAllBytes(root.resolve(path)))));
        return source;
    }

    private void index(EObject object, String fallback) {
        if (object.eIsProxy()) throw new IllegalStateException("Unresolved contained object " + fallback);
        String identity = fallback;
        if (object instanceof AbstractRule rule) identity = ((Grammar) rule.eContainer()).getName() + "::" + rule.getName();
        if (!identityNames.add(identity)) throw new IllegalStateException("Duplicate grammar identity: " + identity);
        if (identities.put(object, identity) != null) throw new IllegalStateException("Repeated containment: " + identity);
        for (EStructuralFeature feature : object.eClass().getEAllStructuralFeatures()) {
            if (feature instanceof EReference ref && ref.isContainment()) {
                Object value = object.eGet(feature, true);
                if (feature.isMany()) {
                    int i = 0;
                    for (Object child : (List<?>) value) index((EObject) child, identity + "/" + feature.getName() + "/" + i++);
                } else if (value != null) index((EObject) value, identity + "/" + feature.getName());
            }
        }
    }

    private Map<String, Object> node(EObject object) {
        if (!XtextPackage.eNS_URI.equals(object.eClass().getEPackage().getNsURI())) throw new IllegalStateException("Unsupported grammar node: " + object.eClass().getName());
        Map<String, Object> result = new LinkedHashMap<>();
        String kind = object.eClass().getName();
        counts.merge(kind, 1, Integer::sum);
        result.put("kind", kind);
        result.put("id", identities.get(object));
        INode location = NodeModelUtils.getNode(object);
        result.put("origin", location == null ? "xtext-inferred" : "source");
        if (location == null) {
            inferredNodeCount++;
            result.put("span", null);
        } else {
            Map<String, Object> span = new LinkedHashMap<>();
            span.put("offset", location.getOffset());
            span.put("length", location.getLength());
            span.put("start_line", location.getStartLine());
            span.put("end_line", location.getEndLine());
            result.put("span", span);
        }
        Map<String, Object> fields = new TreeMap<>();
        List<EStructuralFeature> features = new ArrayList<>(object.eClass().getEAllStructuralFeatures());
        features.sort(Comparator.comparing(EStructuralFeature::getName));
        for (EStructuralFeature feature : features) {
            Object value = object.eGet(feature, true);
            if (feature.isMany()) {
                List<Object> list = new ArrayList<>();
                for (Object child : (List<?>) value) list.add(value(feature, child));
                fields.put(feature.getName(), list);
            } else fields.put(feature.getName(), value(feature, value));
        }
        result.put("fields", fields);
        return result;
    }

    private Object value(EStructuralFeature feature, Object value) {
        if (value == null) return null;
        if (feature instanceof EReference ref) {
            EObject target = (EObject) value;
            if (target.eIsProxy()) throw new IllegalStateException("Unresolved " + feature.getName() + ": " + EcoreUtil.getURI(target));
            if (ref.isContainment()) return node(target);
            String identity = identities.get(target);
            if (identity == null && target instanceof EPackage pkg) identity = pkg.getNsURI();
            if (identity == null && target instanceof EClassifier classifier) identity = classifier.getEPackage().getNsURI() + "#//" + classifier.getName();
            if (identity == null && target instanceof EEnumLiteral literal) identity = literal.getEEnum().getEPackage().getNsURI() + "#//" + literal.getEEnum().getName() + "/" + literal.getName();
            if (identity == null) throw new IllegalStateException("Unexported reference " + feature.getName() + ": " + EcoreUtil.getURI(target));
            referenceCount++;
            return Map.of("$ref", identity);
        }
        if (value instanceof String || value instanceof Boolean || value instanceof Number) return value;
        if (value instanceof org.eclipse.emf.common.util.Enumerator enumerator) return enumerator.getLiteral();
        throw new IllegalStateException("Unsupported attribute " + feature.getName() + ": " + value.getClass());
    }
}
