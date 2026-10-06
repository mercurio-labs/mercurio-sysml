package dev.mercurio.pilot;

import com.google.gson.GsonBuilder;
import com.google.inject.Injector;
import java.io.ByteArrayInputStream;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.*;
import org.eclipse.emf.common.util.URI;
import org.eclipse.emf.ecore.*;
import org.eclipse.xtext.nodemodel.util.NodeModelUtils;
import org.eclipse.xtext.resource.XtextResource;
import org.eclipse.xtext.resource.XtextResourceSet;

/** Independent whole-document parse/construction/link observations, not validation. */
public final class PilotDefinitionDocumentProbe {
    static List<Object> usageFeaturingControls(Injector injector) throws Exception {
        var controls = new ArrayList<Object>();
        var compute = org.omg.sysml.adapter.FeatureAdapter.class.getDeclaredMethod("computeFeaturingType"); compute.setAccessible(true);
        String source = "standard library package Occurrences { class Occurrence { feature snapshots; } class HappensLink; } standard library package Links { class SelfLink; } package P { class Owner :> Occurrences::Occurrence { feature f; } }";
        for (var classifier : org.omg.sysml.lang.sysml.SysMLPackage.eINSTANCE.getEClassifiers()) {
            if (!(classifier instanceof EClass cls) || cls.isAbstract() || !org.omg.sysml.lang.sysml.SysMLPackage.Literals.USAGE.isSuperTypeOf(cls)) continue;
            for (boolean portion : List.of(false, true)) {
                var resources = injector.getInstance(XtextResourceSet.class);
                var resource = (XtextResource)resources.createResource(URI.createURI("memory:/usage-featuring.kerml"));
                resource.load(new ByteArrayInputStream(source.getBytes(StandardCharsets.UTF_8)), Map.of());
                org.omg.sysml.lang.sysml.Feature original = null;
                var iterator = resource.getAllContents();
                while (iterator.hasNext()) {
                    var object = iterator.next();
                    if (object instanceof org.omg.sysml.lang.sysml.Type type) type.setIsImpliedIncluded(true);
                    if (object instanceof org.omg.sysml.lang.sysml.Feature f && "f".equals(f.getDeclaredName())) original = f;
                }
                if (original == null) throw new IllegalStateException("Missing Usage fixture");
                var membership = (org.omg.sysml.lang.sysml.FeatureMembership)original.getOwningRelationship();
                var feature = (org.omg.sysml.lang.sysml.Usage)org.omg.sysml.lang.sysml.SysMLFactory.eINSTANCE.create(cls);
                feature.setDeclaredName("f"); feature.setIsComposite(false); feature.setIsPortion(portion); feature.setIsImpliedIncluded(true);
                membership.setOwnedMemberFeature(feature);
                boolean variable = feature.isVariable();
                var result = (org.omg.sysml.lang.sysml.Type)compute.invoke(org.omg.sysml.util.ElementUtil.getElementAdapter(feature));
                org.omg.sysml.util.FeatureUtil.insertImplicitTypeFeaturings(feature);
                var relation = feature.getOwnedTypeFeaturing().get(0);
                controls.add(Map.of("source", source, "kind", cls.getName(), "portion", portion, "variable", variable,
                    "target_name", result.getDeclaredName(), "target_is_owner", result == feature.getOwningType(),
                    "adopted", result.getOwningRelationship() == relation, "implied", relation.isImplied(),
                    "input_scope", "All input Type generalizations explicitly complete; no claim that native implicit Usage providers are complete"));
                if (!resource.getErrors().isEmpty()) throw new IllegalStateException("Invalid Usage control " + resource.getErrors());
                resource.unload();
            }
        }
        return controls;
    }
    static List<Object> featuringDispatchControls(Injector injector) throws Exception {
        var controls = new ArrayList<Object>();
        var compute = org.omg.sysml.adapter.FeatureAdapter.class.getDeclaredMethod("computeFeaturingType"); compute.setAccessible(true);
        String source = "standard library package Occurrences { class Occurrence { feature snapshots; } } package P { class C { feature f; } }";
        for (var classifier : org.omg.sysml.lang.sysml.SysMLPackage.eINSTANCE.getEClassifiers()) {
            if (!(classifier instanceof EClass cls) || cls.isAbstract() || !org.omg.sysml.lang.sysml.SysMLPackage.Literals.FEATURE.isSuperTypeOf(cls)) continue;
            var sample = (org.omg.sysml.lang.sysml.Feature)org.omg.sysml.lang.sysml.SysMLFactory.eINSTANCE.create(cls);
            if (!sample.getClass().getMethod("isVariable").getDeclaringClass().getName().equals("org.omg.sysml.lang.sysml.impl.FeatureImpl")) continue;
            for (boolean variable : List.of(false, true)) {
                var resources = injector.getInstance(XtextResourceSet.class);
                var resource = (XtextResource)resources.createResource(URI.createURI("memory:/dispatch.kerml"));
                resource.load(new ByteArrayInputStream(source.getBytes(StandardCharsets.UTF_8)), Map.of());
                org.omg.sysml.lang.sysml.Feature original = null;
                var iterator = resource.getAllContents();
                while (iterator.hasNext()) { var object = iterator.next(); if (object instanceof org.omg.sysml.lang.sysml.Feature f && "f".equals(f.getDeclaredName())) original = f; }
                if (original == null) throw new IllegalStateException("Missing dispatch input");
                var membership = (org.omg.sysml.lang.sysml.FeatureMembership)original.getOwningRelationship();
                var feature = (org.omg.sysml.lang.sysml.Feature)org.omg.sysml.lang.sysml.SysMLFactory.eINSTANCE.create(cls);
                feature.setDeclaredName("f"); feature.setIsVariable(variable); membership.setOwnedMemberFeature(feature);
                var owner = feature.getOwningType();
                var result = (org.omg.sysml.lang.sysml.Type)compute.invoke(org.omg.sysml.util.ElementUtil.getElementAdapter(feature));
                org.omg.sysml.util.FeatureUtil.insertImplicitTypeFeaturings(feature);
                var relation = feature.getOwnedTypeFeaturing().get(0);
                controls.add(Map.of("source", source, "kind", cls.getName(), "variable", variable,
                    "target_name", result.getDeclaredName(), "target_is_owner", result == owner,
                    "target_adopted", result.getOwningRelationship() == relation, "implied", relation.isImplied(),
                    "source_matches", relation.getFeatureOfType() == feature, "target_matches", relation.getFeaturingType() == result,
                    "owned_relationships", result.getOwnedRelationship().stream().map(r -> r.eClass().getName()).toList()));
                resource.unload();
            }
        }
        return controls;
    }
    static List<Object> variableFeaturingControls(Injector injector) throws Exception {
        var controls = new ArrayList<Object>();
        var compute = org.omg.sysml.adapter.FeatureAdapter.class.getDeclaredMethod("computeFeaturingType"); compute.setAccessible(true);
        String library = "standard library package Occurrences { class Occurrence { feature snapshots; } } ";
        for (String source : List.of(
            library + "package P { class C { var feature f; } }",
            library + "package P { class C { class D { var feature f; } } }",
            library + "package P { class 'C name' { var feature f; } }",
            library + "package P { class { var feature f; } }",
            "standard library package Occurrences { class Occurrence { feature snapshots; var feature f; } }")) {
            var resources = injector.getInstance(XtextResourceSet.class);
            var resource = (XtextResource)resources.createResource(URI.createURI("memory:/variable.kerml"));
            resource.load(new ByteArrayInputStream(source.getBytes(StandardCharsets.UTF_8)), Map.of());
            if (resource.getParseResult().hasSyntaxErrors()) throw new IllegalStateException("Variable source rejected: " + source);
            org.omg.sysml.lang.sysml.Feature feature = null;
            var iterator = resource.getAllContents();
            while (iterator.hasNext()) { var object = iterator.next(); if (object instanceof org.omg.sysml.lang.sysml.Feature f && "f".equals(f.getDeclaredName())) feature = f; }
            if (feature == null) throw new IllegalStateException("No variable Feature");
            var result = (org.omg.sysml.lang.sysml.Feature)compute.invoke(org.omg.sysml.util.ElementUtil.getElementAdapter(feature));
            org.omg.sysml.util.FeatureUtil.insertImplicitTypeFeaturings(feature);
            var outer = feature.getOwnedTypeFeaturing().get(0);
            var rows = new ArrayList<Object>();
            for (var relation : result.getOwnedRelationship()) {
                if (relation instanceof org.omg.sysml.lang.sysml.Redefinition redef) {
                    rows.add(Map.of("kind", "Redefinition", "source_is_result", redef.getRedefiningFeature() == result, "target", redef.getRedefinedFeature().getDeclaredName()));
                } else if (relation instanceof org.omg.sysml.lang.sysml.TypeFeaturing featuring) {
                    rows.add(Map.of("kind", "TypeFeaturing", "source_is_result", featuring.getFeatureOfType() == result, "target_is_owner", featuring.getFeaturingType() == feature.getOwningType()));
                }
            }
            controls.add(Map.of("source", source, "name", result.getDeclaredName(), "owned_by_outer", result.getOwningRelationship() == outer,
                "outer_implied", outer.isImplied(), "outer_source_is_feature", outer.getFeatureOfType() == feature,
                "outer_target_is_result", outer.getFeaturingType() == result, "relations", rows));
            resource.unload();
        }
        return controls;
    }
    static List<Object> owningFeaturingControls(Injector injector) throws Exception {
        var controls = new ArrayList<Object>();
        var compute = org.omg.sysml.adapter.FeatureAdapter.class.getDeclaredMethod("computeFeaturingType");
        compute.setAccessible(true);
        for (String source : List.of("package P { class C { feature f; } }", "package P { behavior C { out feature f; } }", "package P { feature outer { feature f; } }", "package P { feature f; }")) {
            for (boolean existing : List.of(false, true)) {
                var resources = injector.getInstance(XtextResourceSet.class);
                var resource = (XtextResource)resources.createResource(URI.createURI("memory:/featuring.kerml"));
                resource.load(new ByteArrayInputStream(source.getBytes(StandardCharsets.UTF_8)), Map.of());
                if (resource.getParseResult().hasSyntaxErrors()) throw new IllegalStateException("Featuring source rejected: " + source);
                org.omg.sysml.lang.sysml.Feature feature = null;
                var iterator = resource.getAllContents();
                while (iterator.hasNext()) { var object = iterator.next(); if (object instanceof org.omg.sysml.lang.sysml.Feature f && "f".equals(f.getDeclaredName())) feature = f; }
                if (feature == null) throw new IllegalStateException("No test Feature");
                var owner = feature.getOwningType();
                if (existing && owner != null) {
                    var relation = org.omg.sysml.util.FeatureUtil.addTypeFeaturingTo(feature);
                    relation.setFeaturingType(owner);
                }
                int before = feature.getOwnedTypeFeaturing().size();
                var adapter = org.omg.sysml.util.ElementUtil.getElementAdapter(feature);
                var result = compute.invoke(adapter);
                org.omg.sysml.util.FeatureUtil.insertImplicitTypeFeaturings(feature);
                var rows = new ArrayList<Object>();
                for (var relation : feature.getOwnedTypeFeaturing()) {
                    if (relation.getOwningRelatedElement() != feature || relation.getFeatureOfType() != feature) throw new IllegalStateException("Featuring ownership");
                    rows.add(Map.of("kind", relation.eClass().getName(), "target", relation.getFeaturingType().getDeclaredName(), "implied", relation.isImplied()));
                }
                compute.invoke(adapter); org.omg.sysml.util.FeatureUtil.insertImplicitTypeFeaturings(feature);
                if (feature.getOwnedTypeFeaturing().size() != rows.size()) throw new IllegalStateException("Featuring not idempotent");
                controls.add(Map.of("source", source, "existing", existing, "changed", rows.size() > before, "result", result == null ? "" : ((org.omg.sysml.lang.sysml.Type)result).getDeclaredName(), "relations", rows));
                resource.unload();
            }
        }
        return controls;
    }
    static List<Object> bindingControls() {
        var controls = new ArrayList<Object>();
        for (String ownership : List.of("detached", "owned", "orphan")) {
            for (var input : List.of(List.of("", ""), List.of("a", ""), List.of("", "b"), List.of("a", "b"), List.of("a", "a"))) {
                var factory = org.omg.sysml.lang.sysml.SysMLFactory.eINSTANCE;
                var features = new LinkedHashMap<String, org.omg.sysml.lang.sysml.Feature>();
                for (String id : List.of("a", "b")) {
                    var feature = factory.createFeature(); features.put(id, feature);
                    if (!ownership.equals("detached")) {
                        var membership = factory.createFeatureMembership(); membership.setOwnedMemberFeature(feature);
                        if (ownership.equals("owned")) factory.createType().getOwnedRelationship().add(membership);
                    }
                }
                var connector = org.omg.sysml.util.ConnectorUtil.createBindingConnector(features.get(input.get(0)), features.get(input.get(1)));
                var ends = new ArrayList<Object>();
                for (var relation : connector.getOwnedRelationship()) {
                    var membership = (org.omg.sysml.lang.sysml.FeatureMembership)relation;
                    var end = membership.getOwnedMemberFeature();
                    if (membership.getOwningRelatedElement() != connector || end.getOwningRelationship() != membership) throw new IllegalStateException("Binding ownership");
                    var row = new TreeMap<String,Object>();
                    row.put("membership_kind", membership.eClass().getName()); row.put("end_kind", end.eClass().getName()); row.put("is_end", end.isEnd());
                    row.put("reference_count", end.getOwnedRelationship().size());
                    if (!end.getOwnedRelationship().isEmpty()) {
                        var reference = (org.omg.sysml.lang.sysml.ReferenceSubsetting)end.getOwnedRelationship().get(0);
                        if (reference.getOwningRelatedElement() != end) throw new IllegalStateException("Reference ownership");
                        var referenced = reference.getReferencedFeature();
                        row.put("reference_kind", reference.eClass().getName());
                        row.put("referenced", features.entrySet().stream().filter(e -> e.getValue() == referenced).findFirst().orElseThrow().getKey());
                        row.put("adopted", referenced.getOwningRelationship() == reference);
                    }
                    ends.add(row);
                }
                controls.add(Map.of("ownership", ownership, "inputs", input, "kind", connector.eClass().getName(), "ends", ends));
            }
        }
        return controls;
    }
    static List<Object> chainControls() {
        var controls = new ArrayList<Object>();
        for (var inputs : List.of(List.<String>of(), List.of("a"), List.of("b", "a", "b"), List.of("a", "pair", "a"), List.of("nested", "pair"))) {
            var features = new LinkedHashMap<String, org.omg.sysml.lang.sysml.Feature>();
            for (String id : List.of("a", "b", "pair", "nested")) features.put(id, org.omg.sysml.lang.sysml.SysMLFactory.eINSTANCE.createFeature());
            org.omg.sysml.util.FeatureUtil.addChainingFeature(features.get("pair"), features.get("b"));
            org.omg.sysml.util.FeatureUtil.addChainingFeature(features.get("pair"), features.get("a"));
            org.omg.sysml.util.FeatureUtil.addChainingFeature(features.get("pair"), features.get("b"));
            org.omg.sysml.util.FeatureUtil.addChainingFeature(features.get("nested"), features.get("pair"));
            var output = org.omg.sysml.util.FeatureUtil.chainFeatures(inputs.stream().map(features::get).toArray(org.omg.sysml.lang.sysml.Feature[]::new));
            var targets = output.getOwnedRelationship().stream().map(r -> {
                var chain = (org.omg.sysml.lang.sysml.FeatureChaining)r;
                if (chain.getOwningRelatedElement() != output) throw new IllegalStateException("Chain ownership");
                return features.entrySet().stream().filter(e -> e.getValue() == chain.getChainingFeature()).findFirst().orElseThrow().getKey();
            }).toList();
            var derived = output.getChainingFeature().stream().map(f -> features.entrySet().stream().filter(e -> e.getValue() == f).findFirst().orElseThrow().getKey()).toList();
            var basic = output.getFeatureTarget();
            String basicId = basic == output ? "output" : features.entrySet().stream().filter(e -> e.getValue() == basic).findFirst().orElseThrow().getKey();
            controls.add(Map.of("inputs", inputs, "targets", targets, "chaining_feature", derived,
                "owned_feature_chaining_count", output.getOwnedFeatureChaining().size(), "feature_target", basicId));
        }
        return controls;
    }
    static String property(String name) {
        return name.replaceAll("([a-z])([A-Z])", "$1_$2").toLowerCase(Locale.ROOT);
    }
    static List<?> targets(EObject object, EReference feature, boolean resolve) {
        Object value = object.eGet(feature, resolve);
        return feature.isMany() ? (List<?>) value : value == null ? List.of() : List.of(value);
    }
    static Object scalar(Object value) {
        if (value instanceof org.eclipse.emf.common.util.Enumerator literal) return literal.getLiteral();
        if (value instanceof List<?> values) return values.stream().map(PilotDefinitionDocumentProbe::scalar).toList();
        return value;
    }
    static void stored(EObject object, String path, IdentityHashMap<EObject,String> paths,
            List<EObject> objects, List<Object> rows) {
        if (paths.put(object, path) != null) throw new IllegalStateException("Duplicate stored containment");
        objects.add(object);
        var attributes = new TreeMap<String,Object>();
        for (var feature : object.eClass().getEAllAttributes()) {
            if (feature.isDerived() || feature.isTransient() || feature.isVolatile()
                    || feature.getName().equals("elementId")) continue;
            Object value = object.eGet(feature, false);
            if (value != null) attributes.put(property(feature.getName()), scalar(value));
        }
        var children = new TreeMap<String,Object>();
        rows.add(Map.of("path", path, "kind", object.eClass().getName(),
                "attributes", attributes, "children", children));
        for (var feature : object.eClass().getEAllReferences()) {
            if (!feature.isContainment() || feature.isDerived() || feature.isTransient() || feature.isVolatile()) continue;
            var childPaths = new ArrayList<String>();
            String field = property(feature.getName());
            for (Object value : targets(object, feature, false)) {
                var child = (EObject)value;
                if (child.eContainer() != object || child.eContainmentFeature() != feature)
                    throw new IllegalStateException("Noncanonical stored containment");
                String childPath = path + "/" + field + "/" + childPaths.size();
                childPaths.add(childPath);
                stored(child, childPath, paths, objects, rows);
            }
            children.put(field, childPaths);
        }
    }
    static Map<String,Object> observe(Injector injector, String language, String source) throws Exception {
        return observe(injector, language, source, false);
    }
    static Map<String,Object> observe(Injector injector, String language, String source, boolean complete) throws Exception {
        var resources = injector.getInstance(XtextResourceSet.class);
        var resource = (XtextResource)resources.createResource(URI.createURI("memory:/definition-document." + language));
        resource.load(new ByteArrayInputStream(source.getBytes(StandardCharsets.UTF_8)), Map.of());
        boolean accepted = !resource.getParseResult().hasSyntaxErrors();
        var rows = new ArrayList<Object>();
        var links = new ArrayList<Object>();
        if (accepted) {
            if (complete) {
                var iterator = resource.getAllContents();
                while (iterator.hasNext()) {
                    var node = iterator.next();
                    if (node instanceof org.omg.sysml.lang.sysml.Type type) type.setIsImpliedIncluded(true);
                }
            }
            if (resource.getContents().size() != 1) throw new IllegalStateException("Unexpected root count");
            var paths = new IdentityHashMap<EObject,String>();
            var objects = new ArrayList<EObject>();
            stored(resource.getContents().get(0), "$", paths, objects, rows);
            for (var object : objects) {
                for (var feature : object.eClass().getEAllReferences()) {
                    if (feature.isContainment() || feature.isContainer() || feature.isDerived()
                            || feature.isTransient() || feature.isVolatile()) continue;
                    // Observe every stored reference, including parser-populated
                    // endpoints; extraction of source assignments alone misses them.
                    boolean assigned = !NodeModelUtils.findNodesForFeature(object, feature).isEmpty();
                    var targetPaths = new ArrayList<String>();
                    for (Object value : targets(object, feature, true)) {
                        var target = (EObject)value;
                        if (target.eIsProxy() || !paths.containsKey(target))
                            throw new IllegalStateException("Unresolved/external controlled link: " + source + " " + feature.getName());
                        targetPaths.add(paths.get(target));
                    }
                    if (targetPaths.isEmpty()) {
                        if (assigned) throw new IllegalStateException("Missing assigned link");
                        continue;
                    }
                    links.add(Map.of("owner_path", paths.get(object), "field", property(feature.getName()),
                            "target_paths", targetPaths));
                }
            }
            if (!resource.getErrors().isEmpty()) throw new IllegalStateException("Unexpected frontend errors: " + resource.getErrors());
            var afterPaths = new IdentityHashMap<EObject,String>();
            var afterObjects = new ArrayList<EObject>();
            var afterRows = new ArrayList<Object>();
            stored(resource.getContents().get(0), "$", afterPaths, afterObjects, afterRows);
            if (!rows.equals(afterRows)) throw new IllegalStateException("Linking changed stored construction");
        }
        resource.unload();
        return Map.of("language", language, "source", source, "accepted", accepted, "nodes", rows, "links", links);
    }
    static Map<String,Object> participant(Injector injector, String declaration, boolean end) throws Exception {
        String source = "standard library package Links { assoc Link { feature participant; } } package P { " +
            declaration + " Owner { " + (end ? "end " : "") + "feature candidate; } }";
        var resources = injector.getInstance(XtextResourceSet.class);
        var resource = (XtextResource)resources.createResource(URI.createURI("memory:/participant.kerml"));
        resource.load(new ByteArrayInputStream(source.getBytes(StandardCharsets.UTF_8)), Map.of());
        if (resource.getParseResult().hasSyntaxErrors()) throw new IllegalStateException("Invalid participant source " + source);
        org.omg.sysml.lang.sysml.Feature candidate = null;
        var iterator = resource.getAllContents();
        while (iterator.hasNext()) {
            var object = iterator.next();
            if (object instanceof org.omg.sysml.lang.sysml.Feature feature && "candidate".equals(feature.getDeclaredName())) candidate=feature;
        }
        var adapter=(org.omg.sysml.adapter.TypeAdapter)org.omg.sysml.util.ElementUtil.getElementAdapter(Objects.requireNonNull(candidate));
        var method=org.omg.sysml.adapter.FeatureAdapter.class.getDeclaredMethod("addParticipantSubsetting"); method.setAccessible(true); method.invoke(adapter);
        var contributions=adapter.getImplicitGeneralTypesOnly(org.omg.sysml.lang.sysml.SysMLPackage.Literals.SUBSETTING).stream().map(t->t.getDeclaredName()).toList();
        if (!resource.getErrors().isEmpty()) throw new IllegalStateException("Participant linking failed");
        resource.unload(); return Map.of("source",source,"contributions",contributions);
    }

    static Map<String,Object> variation(Injector injector, String source) throws Exception {
        var resources = injector.getInstance(XtextResourceSet.class);
        var resource = (XtextResource)resources.createResource(URI.createURI("memory:/variation.sysml"));
        resource.load(new ByteArrayInputStream(source.getBytes(StandardCharsets.UTF_8)), Map.of());
        if (resource.getParseResult().hasSyntaxErrors()) throw new IllegalStateException("Invalid variation control " + source);
        var contributions = new ArrayList<Object>();
        var method = org.omg.sysml.adapter.UsageAdapter.class.getDeclaredMethod("addVariationTyping"); method.setAccessible(true);
        var iterator = resource.getAllContents();
        while (iterator.hasNext()) {
            var object = iterator.next();
            if (!(object instanceof org.omg.sysml.lang.sysml.Usage usage)) continue;
            var adapter = (org.omg.sysml.adapter.TypeAdapter)org.omg.sysml.util.ElementUtil.getElementAdapter(usage);
            method.invoke(adapter);
            for (var relation : List.of(org.omg.sysml.lang.sysml.SysMLPackage.Literals.FEATURE_TYPING, org.omg.sysml.lang.sysml.SysMLPackage.Literals.SUBSETTING))
                for (var general : adapter.getImplicitGeneralTypesOnly(relation))
                    contributions.add(Map.of("specific",usage.getDeclaredName(),"general",general.getDeclaredName(),"relationship",relation.getName()));
        }
        if (!resource.getErrors().isEmpty()) throw new IllegalStateException("Invalid variation links");
        resource.unload(); return Map.of("source",source,"contributions",contributions);
    }

    static Map<String,Object> variability(Injector injector, boolean occurrenceOwner, String general, boolean composite, boolean portion) throws Exception {
        String source = "standard library package Items { item def Item; } standard library package Occurrences { item def Occurrence; item def HappensLink; } " +
            "standard library package Links { item def SelfLink; } standard library package Actions { action def Action; } " +
            "package P { item def Unrelated; item def Owner " + (occurrenceOwner ? ":> Occurrences::Occurrence " : "") +
            "{ item x : " + general + "; } }";
        var resources = injector.getInstance(XtextResourceSet.class);
        var resource = (XtextResource)resources.createResource(URI.createURI("memory:/variability.sysml"));
        resource.load(new ByteArrayInputStream(source.getBytes(StandardCharsets.UTF_8)), Map.of());
        if (resource.getParseResult().hasSyntaxErrors()) throw new IllegalStateException("Invalid variability control");
        org.omg.sysml.lang.sysml.Usage target = null;
        var iterator = resource.getAllContents();
        while (iterator.hasNext()) {
            var object = iterator.next();
            if (object instanceof org.omg.sysml.lang.sysml.Type type) type.setIsImpliedIncluded(true);
            if (object instanceof org.omg.sysml.lang.sysml.Usage usage && "x".equals(usage.getDeclaredName())) target = usage;
        }
        Objects.requireNonNull(target).setIsComposite(composite); target.setIsPortion(portion);
        boolean answer = target.isMayTimeVary();
        if (!resource.getErrors().isEmpty()) throw new IllegalStateException("Invalid variability links " + resource.getErrors());
        resource.unload();
        return Map.of("source",source,"composite",composite,"portion",portion,"may_time_vary",answer,
            "input_scope","All Type generalizations explicitly marked complete in both runtimes; implicit Usage providers not assessed");
    }

    static Map<String,Object> conformance(Injector injector, String source) throws Exception {
        var resources = injector.getInstance(XtextResourceSet.class);
        var resource = (XtextResource)resources.createResource(URI.createURI("memory:/conformance.kerml"));
        resource.load(new ByteArrayInputStream(source.getBytes(StandardCharsets.UTF_8)), Map.of());
        if (resource.getParseResult().hasSyntaxErrors()) throw new IllegalStateException("Invalid conformance control");
        var types = new ArrayList<org.omg.sysml.lang.sysml.Type>();
        var iterator = resource.getAllContents();
        while (iterator.hasNext()) {
            var object = iterator.next();
            if (object instanceof org.omg.sysml.lang.sysml.Type type) types.add(type);
        }
        var pairs = new ArrayList<Object>();
        for (var sub : types) for (var sup : types)
            pairs.add(Map.of("subtype", sub.getDeclaredName(), "supertype", sup.getDeclaredName(),
                "specializes", org.omg.sysml.util.TypeUtil.specializes(sub, sup)));
        if (!resource.getErrors().isEmpty()) throw new IllegalStateException("Invalid conformance links");
        resource.unload();
        return Map.of("source", source, "pairs", pairs);
    }

    // Keep provider disagreements separate from successful model comparisons.
    static Map<String,Object> unresolvedLibraryControl(Injector injector, String source) throws Exception {
        var resources = injector.getInstance(XtextResourceSet.class);
        var resource = (XtextResource)resources.createResource(URI.createURI("memory:/library-provider.kerml"));
        resource.load(new ByteArrayInputStream(source.getBytes(StandardCharsets.UTF_8)), Map.of());
        if (resource.getParseResult().hasSyntaxErrors()) throw new IllegalStateException("Invalid library control syntax");
        var unresolved = new ArrayList<Object>();
        var objects = new ArrayList<EObject>(resource.getContents());
        resource.getAllContents().forEachRemaining(object -> { if (!objects.contains(object)) objects.add(object); });
        for (var object : objects) {
            for (var feature : object.eClass().getEAllReferences()) {
                if (feature.isContainment() || feature.isContainer() || feature.isDerived()
                        || feature.isTransient() || feature.isVolatile()) continue;
                for (Object value : targets(object, feature, true)) {
                    if (((EObject)value).eIsProxy()) unresolved.add(Map.of("owner_kind", object.eClass().getName(), "field", property(feature.getName())));
                }
            }
        }
        if (unresolved.isEmpty() || resource.getErrors().isEmpty()) throw new IllegalStateException("Expected independent provider failure");
        var result = Map.<String,Object>of("source", source, "language", "kerml", "syntax_accepted", true,
                "unresolved_references", unresolved, "error_count", resource.getErrors().size());
        resource.unload();
        return result;
    }
    public static void main(String[] args) throws Exception {
        org.omg.sysml.logic.SysMLLogicStandaloneSetup.doSetup();
        EPackage.Registry.INSTANCE.put("https://www.omg.org/spec/SysML/20250201", org.omg.sysml.lang.sysml.SysMLPackage.eINSTANCE);
        var kerml = new org.omg.kerml.xtext.KerMLStandaloneSetup().createInjectorAndDoEMFRegistration();
        var sysml = new org.omg.sysml.xtext.SysMLStandaloneSetup().createInjectorAndDoEMFRegistration();
        var cases = new ArrayList<Object>();
        for (String source : List.of(
                "package P { class A; feature x : A; }",
                "package P { package Q { class A; } feature x : Q::A; }",
                "package P { feature x : Alias; alias Alias for A; class A; }",
                "package A { class T; } package B { private import A::*; feature x : T; }",
                "package P {", "package P; }")) cases.add(observe(kerml, "kerml", source));
        for (String source : List.of(
                "package P { part def A; part x : A; }",
                "package P { package Q { part def A; } part x : Q::A; }",
                "package P { part x : Alias; alias Alias for A; part def A; }",
                "package A { part def T; } package B { private import A::*; part x : T; }",
                "package P {", "package A { part def T; } package B { import A::*; part x : T; }")) cases.add(observe(sysml, "sysml", source));
        for (String source : List.of(
                "package P { class Base; class Derived :> Base; }",
                "package P { class First; class Second; class Derived :> First, Second; }",
                "package P { class Derived :> Alias; alias Alias for Base; class Base; }",
                "package P { class Outer { class Local; feature x : Local; } }",
                "package P { class Outer { class Local; } feature x : Outer::Local; }",
                "package P { feature base; feature sub :> base; }",
                "package P { class Base; class Other conjugates Base; }",
                "package P { class A; class Outer { class A; feature x : A; } }",
                "package P { class Bad :>; }",
                "package P { class Base; class Bad :> Base,; }")) cases.add(observe(kerml, "kerml", source));
        for (String source : List.of(
                "package P { part def Base; part def Derived :> Base; }",
                "package P { part def First; part def Second; part def Derived :> First, Second; }",
                "package P { part def Derived :> Alias; alias Alias for Base; part def Base; }",
                "package P { part def Outer { part def Local; part x : Local; } }",
                "package P { part def Outer { part def Local; } part x : Outer::Local; }",
                "package P { part base; part sub :> base; }",
                "package P { port def Base; port x : ~Base; }",
                "package P { part def A; part def Outer { part def A; part x : A; } }",
                "package P { part def Bad :>; }",
                "package P { part def Base; part def Bad :> Base,; }")) cases.add(observe(sysml, "sysml", source));
        for (String source : List.of(
                "package P { part def Context { ref part refValue; in part inputValue; end part endValue; attribute attr; port boundary; part nested; } }",
                "package P { part def Context { variation part choice { variant part option; } } }",
                "package P { package Q { port def 'odd name'; } port x : ~Q::'odd name'; }")) cases.add(observe(sysml, "sysml", source));
        cases.add(observe(kerml, "kerml", "package P { class Context { const feature x; } }"));
        String occurrences = "standard library package Occurrences { class Occurrence; } ";
        for (String source : List.of(
                "package P { class Base { class Local; } class Derived :> Base { feature x : Local; } }",
                "package P { class Derived :> Base { feature x : Local; } class Base { class Local; } }",
                "package P { class Base { protected class Local; } class Left :> Base; class Right :> Base; class Derived :> Left, Right { feature x : Local; } }",
                "package P { class Base { class Local; } class Derived :> Base; feature x : Derived::Local; }",
                "package P { class Base { class Local; } class Other conjugates Base { feature x : Local; } }",
                "package P { class Outer; class Context { feature x : Outer; } }",
                "package Lib { class Local; } package P { class Context { private import Lib::*; feature x : Local; } }"
                )) cases.add(observe(kerml, "kerml", occurrences + source));
        cases.add(observe(kerml, "kerml", "standard library package Occurrences { class Occurrence { class FromDefault; } } package P { class Context { feature x : FromDefault; } }"));
        cases.add(observe(kerml, "kerml", "standard library package Base { classifier Anything; datatype DataValue; } standard library package Objects { struct Object; } package P { type T :> Base::Anything { class Local; } type U :> T { feature x : Local; } classifier C { class Local; } classifier D :> C { feature x : Local; } datatype V { class Local; } datatype W :> V { feature x : Local; } struct S { class Local; } struct R :> S { feature x : Local; } }"));
        cases.add(observe(kerml, "kerml", occurrences + "package P { class Local; class Base { class Local; } class Derived :> Base { feature x : Local; } }"));
        String parts = "standard library package Parts { part def Part; } ";
        for (String source : List.of(
                "package P { part def Base { part def Local; } part def Derived :> Base { part x : Local; } }",
                "package P { part def Derived :> Base { part x : Local; } part def Base { part def Local; } }",
                "package P { part def Base { protected part def Local; } part def Left :> Base; part def Right :> Base; part def Derived :> Left, Right { part x : Local; } }",
                "package P { part def Base { part def Local; } part def Derived :> Base; part x : Derived::Local; }"
                )) cases.add(observe(sysml, "sysml", parts + source));
        var unsupported = List.of(observe(sysml, "sysml", parts + "standard library package Occurrences { occurrence def Occurrence; occurrence def Life { part def FromLife; } } package P { individual part def Context { part x : FromLife; } }"));
        String connections = "standard library package Connections { connection def Connection { part def FromBase; } connection def BinaryConnection { part def FromBinary; } } ";
        for (String source : List.of(
                "package P { connection def Context { end part a; part x : FromBase; } }",
                "package P { connection def Context { end part a; end part b; part x : FromBinary; } }",
                "package P { connection def Context { end part a; end part b; end part c; part x : FromBase; } }"
                )) cases.add(observe(sysml, "sysml", connections + source));
        cases.add(observe(sysml, "sysml", "standard library package Interfaces { interface def Interface; interface def BinaryInterface { part def FromBinary; } } package P { interface def Context { end port a; end port b; part x : FromBinary; } }"));
        cases.add(observe(sysml, "sysml", "standard library package Flows { flow def MessageAction; flow def Message { item def FromBinary; } } package P { flow def Context { end item a; end item b; item x : FromBinary; } }"));
        cases.add(observe(sysml, "sysml", "standard library package Ports { port def Port; } package P { port def Base { part def Local; } port def Derived :> Base { part x : Local; } }"));
        cases.add(observe(sysml, "sysml", "standard library package Base { attribute def DataValue; } package P { attribute def Base { attribute def Local; } attribute def Derived :> Base { attribute x : Local; } }"));
        for (String source : List.of(
                "package P { class Base { feature unrelated; class Local; } class Derived :> Base { feature x : Local; } }",
                "package P { class Derived :> Base { feature g :> f; } class Base { feature f; } }",
                "package P { class Base { feature f; } class Left :> Base; class Right :> Base; class Derived :> Left, Right { feature g :> f; } }",
                "package P { class Base { feature f; } class Derived :> Base; alias selected for Derived::f; }",
                "package P { class Base { feature <s> long; } class Derived :> Base { feature g :> s; } }",
                "package P { class Base { feature; class Local; } class Derived :> Base { feature x : Local; } }"
                )) cases.add(observe(kerml, "kerml", occurrences + source));
        for (String source : List.of(
                "package P { part def Base { part unrelated; part def Local; } part def Derived :> Base { part x : Local; } }",
                "package P { part def Base { protected part f; } part def Derived :> Base { part g :> f; } }",
                "package P { part def Derived :> Base { part g :> f; } part def Base { part f; } }"
                )) cases.add(observe(sysml, "sysml", parts + source));
        for (String source : List.of(
                "package P { class Base { end feature <a> ea; end feature <b> eb; } class Derived :> Base { end feature; end feature; } }",
                "package P { class Base { end feature ea; end feature eb; } class Middle :> Base; class Derived :> Middle { end feature; end feature; end feature extra; } }",
                "package P { class Base { end feature ea; end feature eb; } class Left :> Base; class Right :> Base; class Derived :> Left, Right { end feature; end feature; } }",
                "package P { class Base { protected end feature ea; private end feature hidden; } class Middle :> Base; class Derived :> Middle { end feature; } }"
                )) cases.add(observe(kerml, "kerml", occurrences + source));
        for (String source : List.of(
                "package P { connection def Base { end part ea; end part eb; } connection def Derived :> Base { end part; end part; } }",
                "package P { connection def Base { end part ea; end part eb; } connection def Middle :> Base; connection def Derived :> Middle { end part; end part; } }"
                )) cases.add(observe(sysml, "sysml", "standard library package Connections { connection def Connection; connection def BinaryConnection; } " + source));
        for (String source : List.of(
                "package P { action def Base { in item a; out item b; } action def Derived :> Base { in item; out item; } }",
                "package P { action def Base { in item a; out item b; } action def Middle :> Base; action def Derived :> Middle { in item; out item; inout item extra; } }",
                "package P { action def Base { in item a; } action def Left :> Base; action def Right :> Base; action def Derived :> Left, Right { in item; } }",
                "package P { action def Base { protected in item a; private out item hidden; } action def Middle :> Base; action def Derived :> Middle { in item; } }"
                )) cases.add(observe(sysml, "sysml", "standard library package Actions { action def Action; } " + source));
        for (String source : List.of(
                "package P { calc def Base { in item a; return item r; } calc def Derived :> Base { in item; return item; } }",
                "package P { calc def Base { return item r; in item a; out item b; } calc def Derived :> Base { in item; out item; return item; } }",
                "package P { calc def Base { in item a; return item r; } calc def Middle :> Base; calc def Derived :> Middle { in item; return item; } }",
                "package P { calc def Base { in item a; return item r; } calc def Left :> Base; calc def Right :> Base; calc def Derived :> Left, Right { in item; return item; } }"
                )) cases.add(observe(sysml, "sysml", "standard library package Calculations { calc def Calculation; } " + source));
        for (String source : List.of(
                "package P { class Base { feature x; } class Derived :> Base { feature x redefines x; } }",
                "package P { class Base { protected feature x; } class Middle :> Base; class Derived :> Middle { feature y redefines x; } }",
                "package P { class Base { feature x; } class Derived :> Base { feature y redefines Base::x; } }",
                "package P { feature outer; class Base; class Derived :> Base { feature y redefines outer; } }"
                )) cases.add(observe(kerml, "kerml", occurrences + source));
        var participantControls = new ArrayList<Object>();
        for (String owner : List.of("assoc", "assoc struct", "class")) for (boolean end : List.of(false,true))
            participantControls.add(participant(kerml,owner,end));
        for (var control : participantControls) cases.add(observe(kerml,"kerml",(String)((Map<?,?>)control).get("source")));
        var variationControls = new ArrayList<Object>();
        for (String source : List.of(
            "package P { variation part def Choice { variant part option; } }",
            "package P { variation part choice { variant part option; } }",
            "package P { variation item def Choice { variant item option; } }",
            "package P { variation item choice { variant item option; } }",
            "package P { variation attribute def Choice { variant attribute option; } }",
            "package P { variation attribute choice { variant attribute option; } }",
            "package P { variation action def Choice { variant action option; } }",
            "package P { variation action choice { variant action option; } }"
        )) variationControls.add(variation(sysml,source));
        for (var control : variationControls) cases.add(observe(sysml, "sysml", (String)((Map<?,?>)control).get("source")));
        var variabilityControls = new ArrayList<Object>();
        for (boolean owner : List.of(false,true)) for (String general : List.of("Unrelated","Links::SelfLink","Occurrences::HappensLink","Actions::Action"))
            for (boolean composite : List.of(false,true)) for (boolean portion : List.of(false,true))
                variabilityControls.add(variability(sysml,owner,general,composite,portion));
        var conformanceControls = new ArrayList<Object>();
        for (String source : List.of(
            "class A; class B :> A; class C :> B; class Unrelated;",
            "class A; class Left :> A; class Right :> A; class Diamond :> Left, Right;",
            "class A :> B; class B :> A; class Unrelated;",
            "class A; class B conjugates A; class C :> B;"
        )) conformanceControls.add(conformance(kerml, occurrences + "package P { " + source + " }"));
        var disagreements = List.of(
                unresolvedLibraryControl(kerml, "package P { class Context { feature x : FromDefault; } } standard library package Occurrences { alias Occurrence for Root; class Root { class FromDefault; } }"),
                unresolvedLibraryControl(kerml, "package P { class Context { feature x : FromDefault; } } standard library package Occurrences { public import Defaults::*; } package Defaults { class Occurrence { class FromDefault; } }"));
        var output = new LinkedHashMap<String,Object>(Map.of("controls", cases, "library_lookup_disagreements", disagreements, "unsupported_native_dependencies", unsupported, "conformance_controls", conformanceControls, "variability_controls", variabilityControls, "variation_controls", variationControls, "participant_controls", participantControls, "chain_construction_controls", chainControls(), "binding_construction_controls", bindingControls(), "owning_type_featuring_controls", owningFeaturingControls(kerml)));
        output.put("variable_featuring_controls", variableFeaturingControls(kerml));
        output.put("featuring_dispatch_controls", featuringDispatchControls(kerml));
        output.put("usage_featuring_controls", usageFeaturingControls(kerml));
        var ordinarySources = new ArrayList<Object>();
        for (String body : List.of("occurrence x;", "connection x;", "interface x;", "allocation x;",
                "constraint x;", "assert constraint x;", "view x;", "event occurrence x;",
                "flow x;", "succession flow x;"))
            ordinarySources.add(observe(sysml, "sysml", "package P { " + body + " }"));
        for (String body : List.of("connector x;", "binding x;", "succession x;", "step x;", "flow x;", "succession flow x;"))
            ordinarySources.add(observe(kerml, "kerml", "package P { " + body + " }"));
        for (String body : List.of("part a; part b; bind a = b;", "part a; part b; first a then b;",
                "enum x;", "metadata def T; metadata x : T;", "part x [1];"))
            ordinarySources.add(observe(sysml, "sysml", "package P { " + body + " }"));
        for (String body : List.of("metaclass T; metadata x : T;", "feature x [1];", "feature a; feature b; binding x of a = b;", "feature a; feature b; succession x first a then b;"))
            ordinarySources.add(observe(kerml, "kerml", "package P { " + body + " }"));
        output.put("ordinary_strategy_source_controls", ordinarySources);
        var connectorProjections = new ArrayList<Object>();
        for (String body : List.of("feature a; feature b; binding x of a = b;", "feature a; binding x of a = a;", "binding x;", "feature a; feature b; succession x first a then b;", "feature a; feature b; binding base of a = b; binding x :> base;", "connector x { end feature e; }", "feature a; feature b; connector x (a, b, b);"))
            connectorProjections.add(observe(kerml, "kerml", "package P { " + body + " }", true));
        output.put("connector_projection_controls", connectorProjections);
        Files.writeString(Path.of(args[0]), new GsonBuilder().setPrettyPrinting().create().toJson(output) + "\n");
    }
}
