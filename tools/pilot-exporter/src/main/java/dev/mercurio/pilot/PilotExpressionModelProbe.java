package dev.mercurio.pilot;
import com.google.gson.GsonBuilder;
import java.nio.file.*;
import java.io.StringReader;
import java.util.*;
import org.eclipse.xtext.*;
import org.eclipse.xtext.parser.IParser;
import org.omg.sysml.lang.sysml.*;
/** Raw grammar plus operand-delegate construction; no linking or library transforms. */
public final class PilotExpressionModelProbe {
    static Map<String,Object> tree(Element element) {
        var row = new TreeMap<String,Object>();
        var namespace = org.omg.sysml.util.NamespaceUtil.getParentNamespaceOf(element);
        if (namespace != null) row.put("parent_namespace_kind", namespace.eClass().getName());
        row.put("kind", element.eClass().getName());
        if (element instanceof LiteralInteger literal) row.put("value", literal.getValue());
        if (element instanceof LiteralBoolean literal) row.put("value", literal.isValue());
        if (element instanceof OperatorExpression operator) row.put("operator", operator.getOperator());
        if (element instanceof Membership member) row.put("visibility", member.getVisibility().getLiteral());
        if (element instanceof Feature feature && feature.getDirection() != null) row.put("direction", feature.getDirection().getLiteral());
        var references = new TreeMap<String,Object>();
        for (var feature : element.eClass().getEAllReferences()) {
            if (feature.isContainment() || feature.isContainer() || feature.isDerived()) continue;
            var nodes = org.eclipse.xtext.nodemodel.util.NodeModelUtils.findNodesForFeature(element, feature);
            if (!nodes.isEmpty()) references.put(feature.getName().replaceAll("([a-z])([A-Z])", "$1_$2").toLowerCase(Locale.ROOT),
                nodes.stream().map(n -> n.getText().strip()).toList());
        }
        if (!references.isEmpty()) row.put("references", references);
        row.put("relationships", element.getOwnedRelationship().stream().map(PilotExpressionModelProbe::tree).toList());
        if (element instanceof Relationship relationship) {
            row.put("elements", relationship.getOwnedRelatedElement().stream().map(PilotExpressionModelProbe::tree).toList());
            for (Element child : relationship.getOwnedRelatedElement())
                if (child.getOwningRelationship() != relationship) throw new IllegalStateException("Missing inverse");
        }
        for (Relationship child : element.getOwnedRelationship())
            if (child.getOwningRelatedElement() != element) throw new IllegalStateException("Missing inverse");
        return row;
    }
    static void indexTree(Element element, String path, Map<Element,String> paths) {
        paths.put(element, path);
        int i = 0;
        for (Relationship child : element.getOwnedRelationship()) indexTree(child, path + "/r" + i++, paths);
        if (element instanceof Relationship relationship) {
            i = 0;
            for (Element child : relationship.getOwnedRelatedElement()) indexTree(child, path + "/e" + i++, paths);
        }
    }
    static Map<String,Object> attachedScopes(Element root) {
        var factory = SysMLFactory.eINSTANCE;
        var namespace = factory.createPackage();
        var member = factory.createOwningMembership();
        var feature = factory.createFeature();
        var value = factory.createFeatureValue();
        namespace.getOwnedRelationship().add(member);
        member.getOwnedRelatedElement().add(feature);
        feature.getOwnedRelationship().add(value);
        value.getOwnedRelatedElement().add(root);
        var paths = new IdentityHashMap<Element,String>();
        indexTree(namespace, "$scope", paths);
        var rows = new TreeMap<String,Object>();
        for (var entry : paths.entrySet()) {
            if (!entry.getValue().startsWith("$scope/r0/e0/r0/e0")) continue;
            var selected = org.omg.sysml.util.NamespaceUtil.getNonExpressionNamespaceFor(entry.getKey());
            if (selected == null || !paths.containsKey(selected)) throw new IllegalStateException("Missing attached namespace");
            rows.put(entry.getValue(), paths.get(selected));
        }
        return rows;
    }
    static List<Object> ownedResults() {
        var rows = new ArrayList<Object>();
        var f = SysMLFactory.eINSTANCE;
        for (String ownerKind : List.of("Function", "Step", "Class", "Feature", "InvocationExpression")) {
            for (String membershipKind : List.of("FeatureMembership", "ParameterMembership", "ReturnParameterMembership")) {
                for (String direction : List.of("unset", "in", "out", "inout")) {
                    Type owner = (Type)f.create((org.eclipse.emf.ecore.EClass)SysMLPackage.eINSTANCE.getEClassifier(ownerKind));
                    var member = (FeatureMembership)f.create((org.eclipse.emf.ecore.EClass)SysMLPackage.eINSTANCE.getEClassifier(membershipKind));
                    var feature = f.createFeature();
                    if (!direction.equals("unset")) feature.setDirection(FeatureDirectionKind.get(direction));
                    owner.getOwnedRelationship().add(member);
                    member.getOwnedRelatedElement().add(feature);
                    var row = new TreeMap<String,Object>();
                    row.put("owner", ownerKind); row.put("membership", membershipKind); row.put("direction", direction);
                    row.put("selected", org.omg.sysml.util.TypeUtil.getOwnedResultParameterOf(owner) == feature);
                    rows.add(row);
                }
            }
        }
        return rows;
    }
    static List<Object> inheritedResults() {
        var rows = new ArrayList<Object>();
        // Ordered diamond with a back edge exercises shared ancestry and legal
        // cyclic generalization. Each mask supplies a distinct set of results.
        for (int mask = 0; mask < 16; mask++) {
            for (boolean reverse : List.of(false, true)) {
                var f = SysMLFactory.eINSTANCE;
                Behavior[] types = {f.createBehavior(), f.createBehavior(), f.createBehavior(), f.createBehavior()};
                Feature[] results = new Feature[4];
                int[][] edges = reverse ? new int[][] {{2,1},{3},{3},{0}} : new int[][] {{1,2},{3},{3},{0}};
                for (int i = 0; i < 4; i++) {
                    for (int general : edges[i]) {
                        var specialization = f.createSubclassification();
                        specialization.setSuperclassifier(types[general]);
                        types[i].getOwnedRelationship().add(specialization);
                    }
                    if ((mask & (1 << i)) != 0) {
                        var member = f.createReturnParameterMembership();
                        results[i] = f.createFeature();
                        results[i].setDirection(FeatureDirectionKind.OUT);
                        member.getOwnedRelatedElement().add(results[i]);
                        types[i].getOwnedRelationship().add(member);
                    }
                }
                var selected = org.omg.sysml.util.TypeUtil.getResultParameterOf(types[0]);
                int identity = -1;
                for (int i = 0; i < 4; i++) if (selected != null && selected == results[i]) identity = i;
                if (selected != null && identity == -1) throw new IllegalStateException("Unexpected synthesized result");
                // Record the actual prerequisite service output, not just fixture
                // edges: unaccounted implicit general types must fail this probe.
                var generalIds = new ArrayList<Object>();
                for (int i = 0; i < 4; i++) {
                    var ids = new ArrayList<Integer>();
                    for (Type general : org.omg.sysml.util.TypeUtil.getGeneralTypesOf(types[i])) {
                        int id = -1;
                        for (int j = 0; j < 4; j++) if (general == types[j]) id = j;
                        if (id == -1) throw new IllegalStateException("Unexpected implicit general type");
                        ids.add(id);
                    }
                    generalIds.add(ids);
                }
                var row = new TreeMap<String,Object>();
                row.put("result_mask", mask); row.put("generals", generalIds); row.put("selected", identity);
                rows.add(row);
            }
        }
        return rows;
    }
    static List<Object> aliasWrites() {
        var rows = new ArrayList<Object>();
        var f = SysMLFactory.eINSTANCE;
        for (String[] definition : List.of(
            new String[] {"Specialization", "Type", "Type", "general", "specific"},
            new String[] {"Subclassification", "Classifier", "Classifier", "superclassifier", "subclassifier"},
            new String[] {"FeatureTyping", "Type", "Feature", "type", "typedFeature"},
            new String[] {"Subsetting", "Feature", "Feature", "subsettedFeature", "subsettingFeature"},
            new String[] {"Redefinition", "Feature", "Feature", "redefinedFeature", "redefiningFeature"})) {
            for (int i = 0; i < 2; i++) {
                var owner = f.create((org.eclipse.emf.ecore.EClass)SysMLPackage.eINSTANCE.getEClassifier(definition[0]));
                var target = f.create((org.eclipse.emf.ecore.EClass)SysMLPackage.eINSTANCE.getEClassifier(definition[1+i]));
                String requested = i == 0 ? "general" : "specific";
                var base = owner.eClass().getEStructuralFeature(requested);
                var effective = owner.eClass().getEStructuralFeature(definition[3+i]);
                owner.eSet(base, target);
                if (owner.eGet(base) != target || owner.eGet(effective) != target)
                    throw new IllegalStateException("Redefined setter did not preserve target");
                var row = new TreeMap<String,Object>();
                row.put("owner", definition[0]); row.put("target", definition[1+i]);
                row.put("requested", requested);
                row.put("stored", definition[3+i].replaceAll("([a-z])([A-Z])", "$1_$2").toLowerCase(Locale.ROOT));
                rows.add(row);
            }
        }
        return rows;
    }
    public static void main(String[] args) throws Exception {
        org.omg.sysml.logic.SysMLLogicStandaloneSetup.doSetup();
        org.eclipse.emf.ecore.EPackage.Registry.INSTANCE.put("https://www.omg.org/spec/SysML/20250201", SysMLPackage.eINSTANCE);
        var cases = new ArrayList<Object>();
        var specializationCases = new ArrayList<Object>();
        var clauseCases = new ArrayList<Object>();
        var declarationCases = new ArrayList<Object>();
        var typeCases = new ArrayList<Object>();
        var definitionCases = new ArrayList<Object>();
        for (String language : List.of("kerml", "sysml")) {
            var injector = language.equals("kerml")
                ? new org.omg.kerml.xtext.KerMLStandaloneSetup().createInjectorAndDoEMFRegistration()
                : new org.omg.sysml.xtext.SysMLStandaloneSetup().createInjectorAndDoEMFRegistration();
            var rule = (ParserRule)GrammarUtil.findRuleForName(injector.getInstance(IGrammarAccess.class).getGrammar(), "OwnedExpression");
            var parser = injector.getInstance(IParser.class);
            if (language.equals("kerml")) for (String kind : List.of("Type", "Classifier")) {
                var entry = (ParserRule)GrammarUtil.findRuleForName(injector.getInstance(IGrammarAccess.class).getGrammar(), kind);
                for (String source : List.of("", "T", "T :> a", "<s> T specializes a,b", "all T :> a", "T [1..3] :> a",
                    "T ~ a", "T conjugates a", "T :> a disjoint from b", "T :> a unions b,c",
                    "T :> a intersects b,c", "T :> a differences b,c", "T :>", "T :> a,", "<s T :>a", "T [1..] :>a")) {
                    var parsed = parser.parse(entry, new StringReader(kind.toLowerCase(Locale.ROOT) + " " + source + ";"));
                    var row = new TreeMap<String,Object>(); row.put("kind", kind); row.put("source", source);
                    row.put("accepted", !parsed.hasSyntaxErrors());
                    if (!parsed.hasSyntaxErrors()) {
                        var root = (Type)parsed.getRootASTElement(); row.put("tree", tree(root));
                        var attrs = new TreeMap<String,Object>();
                        for (String name : List.of("declaredName", "declaredShortName", "isSufficient")) {
                            Object value = root.eGet(root.eClass().getEStructuralFeature(name), false);
                            if (value != null) attrs.put(name.replaceAll("([a-z])([A-Z])", "$1_$2").toLowerCase(Locale.ROOT), value);
                        }
                        row.put("attributes", attrs);
                    }
                    typeCases.add(row);
                }
            }
            if (language.equals("sysml")) {
                var entry = (ParserRule)GrammarUtil.findRuleForName(injector.getInstance(IGrammarAccess.class).getGrammar(), "PartDefinition");
                for (String fragment : List.of("DefinitionDeclaration", "OccurrenceDefinitionPrefix")) {
                    boolean prefix = fragment.equals("OccurrenceDefinitionPrefix");
                    var sources = prefix ? List.of("", "abstract", "variation", "individual", "abstract individual", "variation individual", "#a", "#a #b", "abstract individual #a", "abstract variation", "individual abstract", "#")
                        : List.of("", "P", "<s> P", "P :>a", "P specializes a,b", ":>a", "<s", "P :>", "P :>a,", "P P");
                    for (String source : sources) {
                        String wrapped = prefix ? source + " part def P;" : "part def " + source + ";";
                        var parsed = parser.parse(entry, new StringReader(wrapped));
                        var row = new TreeMap<String,Object>(); row.put("rule", fragment); row.put("source", source);
                        row.put("accepted", !parsed.hasSyntaxErrors());
                        if (!parsed.hasSyntaxErrors()) {
                            var root = (PartDefinition)parsed.getRootASTElement(); row.put("tree", tree(root));
                            var attrs = new TreeMap<String,Object>();
                            var names = prefix ? List.of("isAbstract", "isVariation", "isIndividual") : List.of("declaredName", "declaredShortName");
                            for (String name : names) {
                                Object value = root.eGet(root.eClass().getEStructuralFeature(name), false);
                                if (value != null) attrs.put(name.replaceAll("([a-z])([A-Z])", "$1_$2").toLowerCase(Locale.ROOT), value);
                            }
                            row.put("attributes", attrs);
                        }
                        definitionCases.add(row);
                    }
                }
            }
            var clauseRule = (ParserRule)GrammarUtil.findRuleForName(injector.getInstance(IGrammarAccess.class).getGrammar(), language.equals("kerml") ? "Feature" : "ReferenceUsage");
            for (String source : List.of(": a", ": a,b", ":> a,b", ":>> a,b", "::> a", "=> a",
                "typed by a subsets b redefines c", "[1..3] ordered nonunique", ":a [1..3] ordered nonunique :>b",
                ",a", ":", ":a,", "[1..]", "ordered ordered")) {
                String wrapped = (language.equals("kerml") ? "feature f " : "ref f ") + source + ";";
                var parsed = parser.parse(clauseRule, new StringReader(wrapped));
                var row = new TreeMap<String,Object>();
                row.put("language", language); row.put("source", source); row.put("accepted", !parsed.hasSyntaxErrors());
                if (!parsed.hasSyntaxErrors()) {
                    var root = (Feature)parsed.getRootASTElement();
                    row.put("tree", tree(root)); row.put("is_ordered", root.isOrdered()); row.put("is_unique", root.isUnique());
                }
                clauseCases.add(row);
            }
            for (String source : List.of("f", "<s> f", "all f : a", "f : a,b", "f :> a", "f :>> b",
                "f ~ a", "f [1..3] ordered nonunique", "f chains a.b", "f inverse of b", "f featured by a,b",
                "f disjoint from a", "f unions a,b", "f intersects a,b", "f differences a,b",
                "<s f", "f :", "f :a,", "f [1..]", "f ordered ordered")) {
                String wrapped = (language.equals("kerml") ? "feature " : "ref ") + source + ";";
                var parsed = parser.parse(clauseRule, new StringReader(wrapped));
                var row = new TreeMap<String,Object>();
                row.put("language", language); row.put("source", source); row.put("accepted", !parsed.hasSyntaxErrors());
                if (!parsed.hasSyntaxErrors()) {
                    var root = (Feature)parsed.getRootASTElement();
                    row.put("tree", tree(root));
                    var attrs = new TreeMap<String,Object>();
                    for (String name : List.of("declaredName", "declaredShortName", "isSufficient", "isOrdered", "isUnique")) {
                        Object value = root.eGet(root.eClass().getEStructuralFeature(name), false);
                        if (value != null) attrs.put(name.replaceAll("([a-z])([A-Z])", "$1_$2").toLowerCase(Locale.ROOT), value);
                    }
                    row.put("attributes", attrs);
                }
                declarationCases.add(row);
            }
            for (String ruleName : List.of("OwnedFeatureTyping", "OwnedSubsetting", "OwnedRedefinition", "OwnedReferenceSubsetting", "OwnedCrossSubsetting")) {
                var ownedRule = (ParserRule)GrammarUtil.findRuleForName(injector.getInstance(IGrammarAccess.class).getGrammar(), ruleName);
                for (String source : List.of("a", "q::a", "a.b", "q::a.b.c", "'a.b'", "", "a.", "a::", "a b")) {
                    var parsed = parser.parse(ownedRule, new StringReader(source));
                    var row = new TreeMap<String,Object>();
                    row.put("language", language); row.put("rule", ruleName); row.put("source", source);
                    row.put("accepted", !parsed.hasSyntaxErrors());
                    if (!parsed.hasSyntaxErrors()) row.put("tree", tree((Element)parsed.getRootASTElement()));
                    specializationCases.add(row);
                }
            }

            for (String source : List.of("1", "true", "(1)", "1 + 2", "1 + 2 * 3", "(1 + 2) * 3", "1 - 2 - 3", "2 ** 3 ** 4", "8 / 2", "-1", "not true", "1 < 2", "1 == 2", "1 +", "(1", "x", "P::x", "x + 2", "P::x * x", "true and false", "true or false", "true & false", "true | false", "true xor false", "true and then false", "true or else false", "true implies false", "null ?? 1", "if true ? 1 else 2", "x hastype T", "x as T", "all T", "hastype T", "x.thing", "x#(1)", "x[1]", "(1,2)", "T()", "T(1)", "T(x)", "T(x=1)", "new T()", "new T(1)", "x.metadata", "x.thing(1)", "T(x=U(x=x))")) {
                var result = parser.parse(rule, new StringReader(source));
                var row = new TreeMap<String,Object>();
                row.put("language", language); row.put("source", source); row.put("accepted", !result.hasSyntaxErrors());
                if (!result.hasSyntaxErrors()) {
                    var root = (Element)result.getRootASTElement();
                    row.put("tree", tree(root));
                    row.put("attached_non_expression_scopes", attachedScopes(root));
                }
                cases.add(row);
            }
        }
        Files.writeString(Path.of(args[0] + ".definitions.json"), new GsonBuilder().setPrettyPrinting().create().toJson(definitionCases) + "\n");
        Files.writeString(Path.of(args[0] + ".types.json"), new GsonBuilder().setPrettyPrinting().create().toJson(typeCases) + "\n");
        Files.writeString(Path.of(args[0] + ".declarations.json"), new GsonBuilder().setPrettyPrinting().create().toJson(declarationCases) + "\n");
        Files.writeString(Path.of(args[0] + ".clauses.json"), new GsonBuilder().setPrettyPrinting().create().toJson(clauseCases) + "\n");
        Files.writeString(Path.of(args[0] + ".specializations.json"), new GsonBuilder().setPrettyPrinting().create().toJson(specializationCases) + "\n");
        Files.writeString(Path.of(args[0] + ".aliases.json"), new GsonBuilder().setPrettyPrinting().create().toJson(aliasWrites()) + "\n");
        Files.writeString(Path.of(args[0] + ".inherited.json"), new GsonBuilder().setPrettyPrinting().create().toJson(inheritedResults()) + "\n");
        Files.writeString(Path.of(args[0] + ".results.json"), new GsonBuilder().setPrettyPrinting().create().toJson(ownedResults()) + "\n");
        Files.writeString(Path.of(args[0]), new GsonBuilder().setPrettyPrinting().create().toJson(cases) + "\n");
    }
}
