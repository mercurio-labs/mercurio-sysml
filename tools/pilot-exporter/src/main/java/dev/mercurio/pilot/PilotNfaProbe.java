package dev.mercurio.pilot;
import java.nio.file.*;
import java.util.*;
import com.google.gson.GsonBuilder;
/** Independent branch controls for decisions excluded by eager analysis bounds. */
public final class PilotNfaProbe {
    static List<Object> hiddenControls(com.google.inject.Injector injector) {
        var grammar = injector.getInstance(org.eclipse.xtext.IGrammarAccess.class).getGrammar();
        var rule = (org.eclipse.xtext.ParserRule)org.eclipse.xtext.GrammarUtil.findRuleForName(grammar, "Package");
        var rows = new ArrayList<Object>();
        for(String source : List.of("package P {}", " \t\r\npackage P {\n}",
                "package //* note */ P { // line\r\n}", "package P { // line\r}",
                "package P { //* note */ part def A; }", "package P { //* open",
                "package\u00a0P {}", "package\u000bP {}")) {
            var parsed = injector.getInstance(org.eclipse.xtext.parser.IParser.class).parse(rule, new java.io.StringReader(source));
            var leaves = new ArrayList<Object>();
            for(var leaf : parsed.getRootNode().getLeafNodes()) {
                if(!leaf.isHidden()) continue;
                var element = leaf.getGrammarElement();
                String name = element instanceof org.eclipse.xtext.TerminalRule terminal ? terminal.getName() :
                    element instanceof org.eclipse.xtext.RuleCall call ? call.getRule().getName() : "unknown";
                leaves.add(Map.of("rule",name,"offset",leaf.getOffset(),"length",leaf.getLength(),"text",leaf.getText()));
            }
            rows.add(Map.of("source",source,"context",grammar.getName(),"accepted",!parsed.hasSyntaxErrors(),"hidden",leaves));
        }
        return rows;
    }
    static Object malformed(com.google.inject.Injector injector, String context, String entry,
            String decision, String path, String source, String input) {
        var grammar = injector.getInstance(org.eclipse.xtext.IGrammarAccess.class).getGrammar();
        var rule = (org.eclipse.xtext.ParserRule)org.eclipse.xtext.GrammarUtil.findRuleForName(grammar, decision.split("#",2)[0]);
        org.eclipse.emf.ecore.EObject node = rule.getAlternatives();
        var parts = path.isEmpty() ? new String[0] : path.split("/");
        for (int i=0;i<parts.length;i++) {
            Object value = node.eGet(node.eClass().getEStructuralFeature(parts[i]));
            node = value instanceof List<?> list ? (org.eclipse.emf.ecore.EObject)list.get(Integer.parseInt(parts[++i])) : (org.eclipse.emf.ecore.EObject)value;
        }
        var branches = ((org.eclipse.xtext.Alternatives)node).getElements();
        var result = injector.getInstance(org.eclipse.xtext.parser.IParser.class).parse(
            (org.eclipse.xtext.ParserRule)org.eclipse.xtext.GrammarUtil.findRuleForName(grammar, entry), new java.io.StringReader(source));
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
        var row = new TreeMap<String,Object>();
        row.put("context",context); row.put("decision",decision); row.put("source",source); row.put("input",input);
        row.put("accepted",!result.hasSyntaxErrors()); row.put("alternative",selected);
        return row;
    }
    static List<Object> conflictControls() throws Exception {
        var grammar = new org.antlr.tool.Grammar("parser grammar GuardConflict; tokens { RULE_ID; } a : (RULE_ID)=> RULE_ID | RULE_ID ;");
        grammar.buildNFA();
        org.antlr.analysis.DFA.MAX_TIME_PER_DFA_CREATION = 0;
        var dfa = grammar.createLL_1_LookaheadDFA(1);
        if (dfa == null) dfa = new org.antlr.analysis.DFA(1, grammar.getDecisionNFAStartState(1));
        var cases = new ArrayList<Object>();
        for (boolean guard : List.of(false, true)) {
            var state = dfa.startState;
            int tokens = 0, steps = 0;
            while (!state.isAcceptState()) {
                if (++steps > 20) throw new IllegalStateException("Unexpected reference conflict automaton");
                org.antlr.analysis.DFAState next = null;
                for (int i=0;i<state.getNumberOfTransitions();i++) {
                    var edge = state.transition(i);
                    boolean matches;
                    if (edge.label.isSemanticPredicate()) {
                        var condition = edge.label.getSemanticContext();
                        if (condition instanceof org.antlr.analysis.SemanticContext.TruePredicate) matches = true;
                        else if (condition.isSyntacticPredicate()) matches = guard;
                        else throw new IllegalStateException("Unexpected reference predicate");
                    } else matches = tokens == 0 && edge.label.matches(grammar.getTokenType("RULE_ID"));
                    if (matches) {
                        next = (org.antlr.analysis.DFAState)edge.target;
                        if (!edge.label.isSemanticPredicate()) tokens++;
                        break;
                    }
                }
                if (next == null) throw new IllegalStateException("Missing reference conflict prediction");
                state = next;
            }
            cases.add(Map.of("guard", guard, "alternative", state.getUniquelyPredictedAlt()-1,
                "scope", "Pinned ANTLR frontend ordered predicate-conflict control; not language qualification"));
        }
        return cases;
    }
    static Map<String,Object> storedModel(org.eclipse.emf.ecore.EObject object) {
        var row = new TreeMap<String,Object>();
        row.put("kind",object.eClass().getName());
        var attributes = new TreeMap<String,Object>();
        for (var feature : object.eClass().getEAllAttributes()) {
            // Generated element identities are an external identity service.
            if (feature.isDerived() || feature.isTransient() || feature.isVolatile()
                    || feature.getName().equals("elementId") || !object.eIsSet(feature)) continue;
            Object value = object.eGet(feature, false);
            if(value instanceof org.eclipse.emf.common.util.Enumerator literal) value = literal.getLiteral();
            attributes.put(feature.getName().replaceAll("([a-z])([A-Z])", "$1_$2").toLowerCase(Locale.ROOT),value);
        }
        row.put("attributes",attributes);
        var children = new TreeMap<String,Object>();
        var references = new TreeMap<String,Object>();
        for (var feature : object.eClass().getEAllReferences()) {
            if(feature.isDerived() || feature.isContainer()) continue;
            var name = feature.getName().replaceAll("([a-z])([A-Z])", "$1_$2").toLowerCase(Locale.ROOT);
            if(feature.isContainment()) {
                var values = new ArrayList<Object>();
                Object stored = object.eGet(feature,false);
                var targets = feature.isMany() ? (List<?>)stored : stored == null ? List.of() : List.of(stored);
                for(Object child : targets) {
                    var contained = (org.eclipse.emf.ecore.EObject)child;
                    if(contained.eContainer() != object) throw new IllegalStateException("Missing reciprocal container");
                    values.add(storedModel(contained));
                }
                children.put(name,values);
            } else {
                var nodes = org.eclipse.xtext.nodemodel.util.NodeModelUtils.findNodesForFeature(object,feature);
                if(!nodes.isEmpty()) references.put(name,nodes.stream().map(n -> n.getText().strip()).toList());
            }
        }
        row.put("children",children); row.put("references",references);
        return row;
    }
    static List<Object> modelControls(com.google.inject.Injector injector, String context, String entry, List<String> sources) {
        var grammar = injector.getInstance(org.eclipse.xtext.IGrammarAccess.class).getGrammar();
        var rule = (org.eclipse.xtext.ParserRule)org.eclipse.xtext.GrammarUtil.findRuleForName(grammar,entry);
        var cases = new ArrayList<Object>();
        for (String source : sources) {
            var result = injector.getInstance(org.eclipse.xtext.parser.IParser.class).parse(rule,new java.io.StringReader(source));
            var row = new TreeMap<String,Object>();
            row.put("source",source); row.put("accepted",!result.hasSyntaxErrors());
            row.put("context",context); row.put("entry",entry);
            var tree = new ArrayList<Object>();
            if (!result.hasSyntaxErrors()) {
                var root = result.getRootASTElement();
                row.put("stored_model",storedModel(root));
                var objects = new ArrayList<org.eclipse.emf.ecore.EObject>(); objects.add(root);
                var children = root.eAllContents(); while(children.hasNext()) objects.add(children.next());
                for(var object:objects) {
                    var fields = new TreeMap<String,Object>(); fields.put("kind",object.eClass().getName());
                    var feature = object.eClass().getEStructuralFeature("declaredName");
                    var name = feature == null ? null : object.eGet(feature);
                    if(name != null) fields.put("name",name);
                    tree.add(fields);
                }
            }
            row.put("tree",tree); cases.add(row);
        }
        return cases;
    }
    static List<Object> linkControls(com.google.inject.Injector injector) throws Exception {
        var controls = new ArrayList<Object>();
        for (String source : List.of(
                "package P { part p : C; alias C for B; alias B for A; part def A; }",
                "package P { part def A; alias B for A; alias C for B; part p : C; }",
                "package P { package Q { part def A; } part p : Q::A; }",
                "package P { part def A; package Q { part p : A; } }",
                "package P { part def A; package Q { part def A; part p : A; } }",
                "package P { part def 'A::B'; part p : 'A::B'; }",
                "package P { part def <Short> Long; part p : Short; }",
                "package P { part def A; package Q { alias A for A; part p : A; } }",
                "package P { package Q { part def A; } package R { private import Q::*; part p : A; } }",
                "package P { package R { part p : A; private import Q::*; } package Q { part def A; } }",
                "package P { package Q { part def A; } package R { private import Q::*; part def A; part p : A; } }",
                "package P { package Q { part def A; } package R { public import Q::*; } package S { private import R::*; part p : A; } }",
                "package P { package Q { private part def A; part def B; } package R { private import Q::*; part p : B; } }",
                "package P { package Q { part def <Short> Long; } package R { private import Q::*; part p : Short; } }",
                "package P { package Q { private part def A; } package R { private import all Q::*; part p : A; } }",
                "package P { package Q { package N { part def A; } } package R { private import Q::*::**; part p : A; } }",
                "package P { package Q { package N { package M { part def A; } } } package R { private import Q::*::**; part p : A; } }",
                "package P { package Q { public import R::*; part def A; } package R { public import Q::*; } package S { private import R::*; part p : A; } }",
                "package P { package Q { part def A; } package R { private import Q::A; part p : A; } }",
                "package P { package Q { part def A; alias B for A; } package R { part p : B; private import Q::B; } }",
                "package P { package Q { part def <Short> Long; } package R { private import Q::Short; part p : Short; } }",
                "package P { package Q { part def A; } package R { public import Q::A; } package S { private import R::*; part p : A; } }",
                "package P { package Q { part def A; } private import Q::**; part p : A; }",
                "package P { package Q { private part def A; } private import all Q::A; part p : A; }",
                "package P { package Q { part def A; } alias B for Q; private import B::**; part p : A; }",
                "package P { package Q { part def A; } package T { part def B; } package R { private import Q::*; private import T::*; part a : A; part b : B; } }",
                "package P { package Q { part def A; } private import Q::*; private import Q::*; part p : A; }",
                "package P { package Q { part def A; } package R { public import Q::*; } package S { public import Q::*; } package T { private import R::*; private import S::*; part p : A; } }",
                "package P { part def <Short>; part p : Short; }",
                "package P { package Q { part def <Short>; } private import Q::*; part p : Short; }",
                "package P { package Q { part def <Short>; } private import Q::Short; part p : Short; }",
                "package P { package Q { part def A; } alias <Short> for Q::A; part p : Short; }",
                "package P { part def; part def A; part p : A; }",
                "package P { package Q { part def; part def A; } private import Q::*; part p : A; }")) {
            var resources = injector.getInstance(org.eclipse.xtext.resource.XtextResourceSet.class);
            var resource = resources.createResource(org.eclipse.emf.common.util.URI.createURI("memory:/links.sysml"));
            resource.load(new java.io.ByteArrayInputStream(source.getBytes(java.nio.charset.StandardCharsets.UTF_8)), Map.of());
            var links = new ArrayList<Object>();
            var objects = resource.getAllContents();
            while(objects.hasNext()) {
                var object = objects.next();
                org.omg.sysml.lang.sysml.Element target;
                String field;
                if(object instanceof org.omg.sysml.lang.sysml.FeatureTyping typing) { target=typing.getType(); field="type"; }
                else if(object instanceof org.omg.sysml.lang.sysml.NamespaceImport imp) { target=imp.getImportedNamespace(); field="imported_namespace"; }
                else if(object instanceof org.omg.sysml.lang.sysml.MembershipImport imp) { target=imp.getImportedMembership(); field="imported_membership"; }
                else if(object.eClass().getName().equals("Membership")) { target=((org.omg.sysml.lang.sysml.Membership)object).getMemberElement(); field="member_element"; }
                else continue;
                if(target == null || target.eIsProxy()) throw new IllegalStateException("Unresolved controlled Pilot link: " + source);
                String targetName = target instanceof org.omg.sysml.lang.sysml.Membership mem ? mem.getMemberName() : target.getDeclaredName();
                if(targetName == null) targetName = target instanceof org.omg.sysml.lang.sysml.Membership mem ? mem.getMemberShortName() : target.getDeclaredShortName();
                var path = new ArrayList<String>();
                if(target instanceof org.omg.sysml.lang.sysml.Membership) path.add(org.omg.sysml.util.ElementUtil.unescapeString(targetName));
                for(org.omg.sysml.lang.sysml.Element cursor=target; cursor!=null; cursor=org.omg.sysml.util.NamespaceUtil.getParentNamespaceOf(cursor)) {
                    if(cursor.getDeclaredName()!=null) path.add(org.omg.sysml.util.ElementUtil.unescapeString(cursor.getDeclaredName()));
                }
                Collections.reverse(path);
                links.add(Map.of("owner_kind",object.eClass().getName(),"field",field,
                    "target_kind",target.eClass().getName(),"target_label",org.omg.sysml.util.ElementUtil.unescapeString(targetName),
                    "target_path",path));
            }
            if(links.isEmpty() || !resource.getErrors().isEmpty()) throw new IllegalStateException("Changed Pilot link control: " + resource.getErrors());
            controls.add(Map.of("source",source,"links",links));
            resource.unload();
        }
        return controls;
    }
    static List<Object> scopeDisagreements(com.google.inject.Injector injector) throws Exception {
        String source = "package P { package Q { private package N { part def A; } } package R { private import all Q::*::**; part p : A; } }";
        var resources = injector.getInstance(org.eclipse.xtext.resource.XtextResourceSet.class);
        var resource = resources.createResource(org.eclipse.emf.common.util.URI.createURI("memory:/scope-disagreement.sysml"));
        resource.load(new java.io.ByteArrayInputStream(source.getBytes(java.nio.charset.StandardCharsets.UTF_8)), Map.of());
        boolean unresolved = false;
        var derived = new ArrayList<Object>();
        var objects = resource.getAllContents();
        while(objects.hasNext()) {
            var object = objects.next();
            if(object instanceof org.omg.sysml.lang.sysml.FeatureTyping typing) {
                var target = typing.getType(); unresolved = target == null || target.eIsProxy();
            }
            if(object instanceof org.omg.sysml.lang.sysml.NamespaceImport imp) {
                for(var mem : imp.importedMemberships(new org.eclipse.emf.common.util.BasicEList<>())) {
                    if(!"A".equals(mem.getMemberName())) continue;
                    var target = mem.getMemberElement();
                    var path = new ArrayList<String>();
                    for(org.omg.sysml.lang.sysml.Element cursor=target; cursor!=null; cursor=org.omg.sysml.util.NamespaceUtil.getParentNamespaceOf(cursor)) {
                        if(cursor.getDeclaredName()!=null) path.add(cursor.getDeclaredName());
                    }
                    Collections.reverse(path);
                    derived.add(Map.of("target_kind",target.eClass().getName(),"target_path",path));
                }
            }
        }
        if(!unresolved || derived.size()!=1) throw new IllegalStateException("Changed recursive import-all disagreement");
        resource.unload();
        return List.of(Map.of("source",source,"pilot_link_unresolved",unresolved,"derived_members",derived));
    }
    static List<Object> membershipConflicts(com.google.inject.Injector injector) throws Exception {
        var controls = new ArrayList<Object>();
        for(String source : List.of(
            "package P { part def A; package Q { part def A; } package T { part def A; } package R { private import Q::*; private import T::*; part p : A; } }",
            "package P { part def A; package Q { part def <Shared> A; } package T { part def <Shared> B; } package R { private import Q::*; private import T::*; part p : A; } }",
            "package P { part def A; package Q { part def <Shared> A; } package R { part def <Shared> X; private import Q::*; part p : A; } }",
            "package P { package Q { package A; } package T { part def A; } package R { private import Q::*; private import T::*; part p : A; } }")) {
            var resources = injector.getInstance(org.eclipse.xtext.resource.XtextResourceSet.class);
            var resource = resources.createResource(org.eclipse.emf.common.util.URI.createURI("memory:/membership-conflicts.sysml"));
            resource.load(new java.io.ByteArrayInputStream(source.getBytes(java.nio.charset.StandardCharsets.UTF_8)), Map.of());
            var targetPath = new ArrayList<String>();
            int collisions = 0;
            int metaclassCollisions = 0;
            boolean unresolved = false;
            var objects = resource.getAllContents();
            while(objects.hasNext()) {
                var object = objects.next();
                if(object instanceof org.omg.sysml.lang.sysml.FeatureTyping typing) {
                    var target = typing.getType();
                    if(target == null || target.eIsProxy()) { unresolved = true; continue; }
                    for(org.omg.sysml.lang.sysml.Element cursor=target; cursor!=null; cursor=org.omg.sysml.util.NamespaceUtil.getParentNamespaceOf(cursor)) {
                        if(cursor.getDeclaredName()!=null) targetPath.add(cursor.getDeclaredName());
                    }
                    Collections.reverse(targetPath);
                }
                if(object instanceof org.omg.sysml.lang.sysml.Package ns && "R".equals(ns.getDeclaredName())) {
                    var memberships = new ArrayList<>(ns.getMembership());
                    for(int i=0;i<memberships.size();i++) for(int j=i+1;j<memberships.size();j++) {
                        if(!memberships.get(i).isDistinguishableFrom(memberships.get(j))) {
                            collisions++;
                            var left = memberships.get(i).getMemberElement().eClass();
                            var right = memberships.get(j).getMemberElement().eClass();
                            if(left.isSuperTypeOf(right) || right.isSuperTypeOf(left)) metaclassCollisions++;
                        }
                    }
                }
            }
            if(targetPath.isEmpty() && !unresolved) throw new IllegalStateException("Changed membership conflict control");
            controls.add(Map.of("source",source,"pilot_target_path",targetPath,"collision_count",collisions,"metaclass_collision_count",metaclassCollisions,"pilot_link_unresolved",unresolved));
            resource.unload();
        }
        return controls;
    }
    public static void main(String[] args) throws Exception {
        org.omg.sysml.logic.SysMLLogicStandaloneSetup.doSetup();
        org.eclipse.emf.ecore.EPackage.Registry.INSTANCE.put("https://www.omg.org/spec/SysML/20250201", org.omg.sysml.lang.sysml.SysMLPackage.eINSTANCE);
        var kerml = new org.omg.kerml.xtext.KerMLStandaloneSetup().createInjectorAndDoEMFRegistration();
        var sysml = new org.omg.sysml.xtext.SysMLStandaloneSetup().createInjectorAndDoEMFRegistration();
        Files.writeString(Path.of(args[0]+".membership-conflicts.json"), new GsonBuilder().setPrettyPrinting().create().toJson(membershipConflicts(sysml))+"\n");
        Files.writeString(Path.of(args[0]+".scope-disagreements.json"), new GsonBuilder().setPrettyPrinting().create().toJson(scopeDisagreements(sysml))+"\n");
        var hidden = new ArrayList<Object>();
        hidden.addAll(hiddenControls(kerml)); hidden.addAll(hiddenControls(sysml));
        Files.writeString(Path.of(args[0]+".hidden.json"), new GsonBuilder().setPrettyPrinting().create().toJson(hidden)+"\n");
        Files.writeString(Path.of(args[0]+".links.json"), new GsonBuilder().setPrettyPrinting().create().toJson(linkControls(sysml))+"\n");
        var cases = new ArrayList<Object>();
        for (String body : List.of("feature f;", "step s;", "connector c from a to b;"))
            PilotExpandedDecisionProbe.probe(cases,kerml,"org.omg.kerml.xtext.KerML","Function","FeatureElement","","function F { " + body + " }",body + " }");
        for (String body : List.of("part p;", "attribute a;", "part def Q;", "alias a for b;", "public import a::*;"))
            PilotExpandedDecisionProbe.probe(cases,sysml,"org.omg.sysml.xtext.SysML","PartDefinition","DefinitionBodyItem","","part def P { " + body + " }",body + " }");
        Files.writeString(Path.of(args[0]+".conflicts.json"), new GsonBuilder().setPrettyPrinting().create().toJson(conflictControls())+"\n");
        var models = new ArrayList<Object>();
        models.addAll(modelControls(sysml,"org.omg.sysml.xtext.SysML","PartDefinition", List.of("part def P { part p; }", "part def P { attribute a; }",
                "part def P { part def Q; }", "part def P { alias a for b; }", "part def P { public import a::*; }",
                "part def P { part p; attribute a; part def Q; }", "part def P { part def Q { part q; } }",
                "part def P { part p : ; }", "part def P { part p;", "part def P { part p; unexpected @ }")));
        models.addAll(modelControls(kerml,"org.omg.kerml.xtext.KerML","Function", List.of(
            "function F { feature f; }", "function F { in feature f; out feature g; }",
            "function F { feature f : ; }")));
        models.addAll(modelControls(sysml,"org.omg.sysml.xtext.SysML","AssignmentNode", List.of(
            "assign x := 1;", "assign a.b := 2;", "assign x := ;")));
        for (var control : List.of(
            List.of("Package", "package P { part def A; part a : A; }"),
            List.of("ActionDefinition", "action def A { action a; }"),
            List.of("CalculationDefinition", "calc def C { 1 }"),
            List.of("StateDefinition", "state def S { state s; }"),
            List.of("ConstraintDefinition", "constraint def C { true }"),
            List.of("RequirementDefinition", "requirement def R { subject s; }"),
            List.of("PortDefinition", "port def P { in item i; }"),
            List.of("ItemDefinition", "item def I;"),
            List.of("ConnectionDefinition", "connection def C;"),
            List.of("InterfaceDefinition", "interface def I;"),
            List.of("EnumerationDefinition", "enum def E { enum a; enum b; }"))) {
            models.addAll(modelControls(sysml,"org.omg.sysml.xtext.SysML",control.get(0),List.of(control.get(1))));
        }
        Files.writeString(Path.of(args[0]+".models.json"), new GsonBuilder().setPrettyPrinting().create().toJson(models)+"\n");
        var negatives = new ArrayList<Object>();
        negatives.add(malformed(kerml,"org.omg.kerml.xtext.KerML","Connector","ConnectorDeclaration","","connector c from a to ;","c from a to ;"));
        negatives.add(malformed(kerml,"org.omg.kerml.xtext.KerML","Function","FeatureElement","","function F { feature f : ; }","feature f : ; }"));
        negatives.add(malformed(sysml,"org.omg.sysml.xtext.SysML","PartDefinition","DefinitionBodyItem","","part def P { part p : ; }","part p : ; }"));
        negatives.add(malformed(sysml,"org.omg.sysml.xtext.SysML","SendNode","SendNode#/elements/3","elements/3","send x to ;","x to ;"));
        negatives.add(malformed(sysml,"org.omg.sysml.xtext.SysML","TerminateNode","TerminateNode","elements/3","terminate x + ;","x + ;"));
        negatives.add(malformed(sysml,"org.omg.sysml.xtext.SysML","IfNode","IfNode","elements/4/elements/1/terminal","if true {} else if ;","if ;"));
        negatives.add(malformed(sysml,"org.omg.sysml.xtext.SysML","AcceptNode","PayloadParameter","","accept after ;","after ;"));
        Files.writeString(Path.of(args[0]+".negative.json"), new GsonBuilder().setPrettyPrinting().create().toJson(negatives)+"\n");
        Files.writeString(Path.of(args[0]), new GsonBuilder().setPrettyPrinting().create().toJson(cases) + "\n");
    }
}
