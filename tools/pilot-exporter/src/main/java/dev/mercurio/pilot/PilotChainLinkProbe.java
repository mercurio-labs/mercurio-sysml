package dev.mercurio.pilot;
import com.google.gson.GsonBuilder;
import java.nio.file.*;
import java.util.*;
import org.omg.sysml.lang.sysml.*;
public final class PilotChainLinkProbe {
    static List<String> declaredPath(Element element) {
        var path = new ArrayList<String>();
        for (Element cursor = element; cursor != null; cursor = org.omg.sysml.util.NamespaceUtil.getParentNamespaceOf(cursor)) {
            if (cursor.getDeclaredName() != null) path.add(org.omg.sysml.util.ElementUtil.unescapeString(cursor.getDeclaredName()));
        }
        Collections.reverse(path);
        return path;
    }
    public static void main(String[] args) throws Exception {
        org.omg.sysml.logic.SysMLLogicStandaloneSetup.doSetup();
        org.eclipse.emf.ecore.EPackage.Registry.INSTANCE.put("https://www.omg.org/spec/SysML/20250201", SysMLPackage.eINSTANCE);
        var injector = new org.omg.kerml.xtext.KerMLStandaloneSetup().createInjectorAndDoEMFRegistration();
        var cases = new ArrayList<Object>();
        String base = "package ChainScope { class A { feature x : B; } class B { feature y : C; } class C { feature z : A; } feature y : A; feature missing : A; inverting Invert inverse %s of C::z; }";
        var sources = new ArrayList<String>();
        for (String operand : List.of("A::x.y.z", "A::x.y::z", "A::x.missing", "A::x.y::missing")) sources.add(String.format(base, operand));
        sources.add(String.format(base, "A::x.y::z").replace("class B { feature y : C; }", "class Base { feature y : C; } class B :> Base;"));
        sources.add(String.format(base, "A::x.y::z").replace("feature y : C;", "feature y : C { feature z : A; }"));
        sources.add(String.format(base, "A::x.Inner::z").replace("class B { feature y : C; }", "class B { class Inner { feature z : A; } }"));
        sources.add(String.format(base, "A::x.Inner::missing").replace("class B { feature y : C; }", "class B { class Inner { feature z : A; } }"));
        sources.add(String.format(base, "A::x.Inner").replace("class B { feature y : C; }", "class B { class Inner { feature z : A; } }"));
        sources.add(String.format(base, "A.z").replace("class A { feature x : B; }", "class A { feature x : B; feature z : A; }"));
        sources.add(String.format(base, "A::x.Inner::z").replace("class B { feature y : C; }", "class Base { feature Inner : C; } class B :> Base { class Inner { feature z : A; } }"));
        sources.add(String.format(base, "A::x.Inner::z").replace("class B { feature y : C; }", "class Base { class Inner { feature z : A; } } class B :> Base;"));
        sources.add(String.format(base, "A::x.Inner::z").replace("class B { feature y : C; }", "class Base { class Inner { feature z : A; } } class Mid :> Base; class B :> Mid;"));
        sources.add(String.format(base, "A::x.Inner::z").replace("class B { feature y : C; }", "class Base { class Inner { feature z : A; } } class Mid :> Base; class B :> Mid { class Inner { feature z : C; } }"));
        sources.add(String.format(base, "A::x.Inner::z").replace("class B { feature y : C; }", "class B { private class Inner { feature z : A; } }"));
        sources.add(String.format(base, "A::x.Inner::z").replace("class B { feature y : C; }", "class B { class Inner { private feature z : A; } }"));
        sources.add("package ChainScope { class A { feature x : B; } class B { private class Inner { feature z : A; } inverting Invert inverse A::x.Inner::z of C::z; } class C { feature z : A; } }");
        sources.add(String.format(base, "A::x.Shortcut::z").replace("class B { feature y : C; }", "class B { class Inner { feature z : A; } alias Shortcut for Inner; }"));
        sources.add(String.format(base, "A::x.shortcut::z").replace("class B { feature y : C; }", "class B { feature y : C; alias shortcut for y; }"));
        sources.add(String.format(base, "A::x.Shortcut::z").replace("class B { feature y : C; }", "class B { class Inner { feature z : A; } private alias Shortcut for Inner; }"));
        sources.add(String.format(base, "A::x.Shortcut::z").replace("class B { feature y : C; }", "class Base { class Inner { feature z : A; } alias Shortcut for Inner; } class B :> Base;"));
        sources.add(String.format(base, "A::x.shortcut::z").replace("class B { feature y : C; }", "class B { feature y : C; private alias shortcut for y; }"));
        sources.add(String.format(base, "A::x.shortcut::z").replace("class B { feature y : C; }", "class B { private feature y : C; alias shortcut for y; }"));
        sources.add(String.format(base, "A::x.Shortcut::z").replace("class B { feature y : C; }", "class B { class Inner { feature z : A; } alias Shortcut for Other; alias Other for Inner; }"));
        sources.add(String.format(base, "A::x.shortcut").replace("class B { feature y : C; }", "class B { comment Note /* text */ alias shortcut for Note; }"));
        sources.add(String.format(base, "A::x.shortcut").replace("class B { feature y : C; }", "class B { disjoining Link disjoint C from A; alias shortcut for Link; }"));
        sources.add(String.format(base, "A::x.shortcut").replace("class B { feature y : C; }", "class B { feature y : C; alias shortcut for y; }"));
        sources.add(String.format(base, "A::x.Shortcut").replace("class B { feature y : C; }", "class B { class Inner { feature z : A; } alias Shortcut for Inner; }"));
        sources.add(String.format(base, "A::x.shortcut").replace("class B { feature y : C; }", "class B { multiplicity y [1..1]; alias shortcut for y; }"));
        sources.addAll(List.of(
            "package P { feature x { feature y; } feature f chains x.y; }",
            "package P { feature f chains x.y; feature x { feature y; } }",
            "package P { feature y; feature x { feature y; } feature f chains x.y; }",
            "package P { feature y; feature x; feature f chains x.y; }",
            "package P { feature x { private feature y; } feature f chains x.y; }",
            "package P { feature x { feature y; } feature f subsets x.y; }",
            "package P { feature x { feature y; } feature f crosses x.y; }",
            "package P { class A { feature x { feature y; } feature f chains x.y; } }",
            "package P { feature x { feature 'y::z'; } feature f chains x.'y::z'; }",
            "standard library package Base { classifier Anything; } package P { classifier A { feature x { feature y; } feature other; } classifier B :> A { feature f chains x.y; feature renamed redefines other; } }",
            "standard library package Base { classifier Anything; } package P { classifier B :> A { feature f chains x.y; feature renamed redefines other; } classifier A { feature x { feature y; } feature other; } }"
        ));
        sources.add("standard library package Base { classifier Anything; feature things; } package P { classifier A { feature a { feature y; } feature b; } classifier B :> A { feature f chains a2.y; feature a2 redefines a; feature b2 redefines b; } }");
        sources.add("standard library package Base { classifier Anything; feature things; } package P { classifier B :> A { feature f chains a2.y; feature b2 redefines b; feature a2 redefines a; } classifier A { feature a { feature y; } feature b; } }");
        String library = "standard library package Base { classifier Anything; feature things; } standard library package Occurrences { feature occurrences; class Occurrence; } ";
        String targetSource = "classifier T { feature m; } ";
        for (String last : List.of("m", "absent")) {
            for (boolean referenceFirst : List.of(false, true)) {
                String ends = "end cross feature e : T; end feature other; ";
                String chain = "feature f chains e.'cross'." + last + "; ";
                String context = "class C { " + (referenceFirst ? chain + ends : ends + chain) + "} ";
                sources.add(library + context + targetSource);
                sources.add(library + targetSource + context);
            }
        }
        // Reciprocal subsets exercise scoped prerequisite cycles independently of library names.
        for (String last : List.of("self", "absent")) {
            for (boolean reverseEnds : List.of(false, true)) {
                String first = "end feature thisThing : Base::Anything subsets sameThing crosses sameThing." + last + "; ";
                String second = "end self2 feature sameThing : Base::Anything subsets thisThing; ";
                String context = "class C { " + (reverseEnds ? second + first : first + second) + "} ";
                String baseLibrary = "standard library package Base { classifier Anything { feature self : Anything; } feature things : Anything; } standard library package Occurrences { feature occurrences; class Occurrence; } ";
                sources.add(baseLibrary + context);
                sources.add(context + baseLibrary);
            }
        }
        for (String source : sources) {
            var resources = injector.getInstance(org.eclipse.xtext.resource.XtextResourceSet.class);
            var resource = resources.createResource(org.eclipse.emf.common.util.URI.createURI("memory:/chain.kerml"));
            resource.load(new java.io.ByteArrayInputStream(source.getBytes(java.nio.charset.StandardCharsets.UTF_8)), Map.of());
            if (!resource.getErrors().isEmpty()) throw new IllegalStateException("Invalid control syntax: " + resource.getErrors());
            var chains = new ArrayList<FeatureChaining>();
            var iterator = resource.getAllContents();
            while (iterator.hasNext()) { var object = iterator.next(); if (object instanceof FeatureChaining chain) chains.add(chain); }
            var links = new ArrayList<Object>();
            for (var chain : chains) {
                var row = new TreeMap<String,Object>();
                var feature = chain.eClass().getEStructuralFeature("chainingFeature");
                var nodes = org.eclipse.xtext.nodemodel.util.NodeModelUtils.findNodesForFeature(chain, feature);
                row.put("spelling", nodes.stream().map(n -> n.getText()).reduce("", String::concat).trim());
                var target = chain.getChainingFeature();
                boolean resolved = target != null && !target.eIsProxy();
                row.put("resolved", resolved);
                if (resolved) {
                    var path = new ArrayList<String>();
                    for (Element cursor = target; cursor != null; cursor = org.omg.sysml.util.NamespaceUtil.getParentNamespaceOf(cursor)) {
                        if (cursor.getDeclaredName() != null) path.add(org.omg.sysml.util.ElementUtil.unescapeString(cursor.getDeclaredName()));
                    }
                    Collections.reverse(path);
                    row.put("target_path", path);
                    row.put("target_kind", target.eClass().getName());
                }
                links.add(row);
            }
            var row = new TreeMap<String,Object>();
            row.put("source", source); row.put("links", links);
            var redefinitions = new ArrayList<Object>();
            var modelObjects = resource.getAllContents();
            while (modelObjects.hasNext()) {
                var item = modelObjects.next();
                if (item instanceof Redefinition redefinition) {
                    var target = redefinition.getRedefinedFeature();
                    boolean resolved = target != null && !target.eIsProxy();
                    var observation = new TreeMap<String,Object>();
                    observation.put("resolved", resolved);
                    observation.put("owner_path", declaredPath(redefinition.getOwningFeature()));
                    if (resolved) observation.put("target_path", declaredPath(target));
                    redefinitions.add(observation);
                }
            }
            row.put("redefinitions", redefinitions);
            cases.add(row);
            resource.unload();
        }
        var names = new ArrayList<Object>();
        for (String identification : List.of("Invert", "inv", "'inv'", "<Short> Invert", "<inv> Invert", "<'inv'> Invert")) {
            String source = String.format(base, "A::x.y::z").replace("inverting Invert inverse", "inverting " + identification + " inverse");
            var resources = injector.getInstance(org.eclipse.xtext.resource.XtextResourceSet.class);
            var resource = resources.createResource(org.eclipse.emf.common.util.URI.createURI("memory:/identification.kerml"));
            resource.load(new java.io.ByteArrayInputStream(source.getBytes(java.nio.charset.StandardCharsets.UTF_8)), Map.of());
            var row = new TreeMap<String,Object>();
            row.put("source", source); row.put("accepted", resource.getErrors().isEmpty());
            names.add(row);
            resource.unload();
        }
        Files.writeString(Path.of(args[0] + ".names.json"), new GsonBuilder().setPrettyPrinting().create().toJson(names) + "\n");
        Files.writeString(Path.of(args[0]), new GsonBuilder().setPrettyPrinting().create().toJson(cases) + "\n");
    }
}
