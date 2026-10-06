package dev.mercurio.pilot;

import com.google.gson.GsonBuilder;
import java.nio.file.*;
import java.nio.charset.StandardCharsets;
import java.io.ByteArrayInputStream;
import java.util.*;
import org.eclipse.emf.common.util.URI;
import org.eclipse.xtext.resource.*;
import org.eclipse.xtext.resource.impl.*;
import org.omg.sysml.lang.sysml.*;

/** Bounded source-resource linking observations only; never invokes validation. */
public final class PilotResourceLinkProbe {
    record Unit(String uri, String language, String source) {}
    record Control(String name, List<Unit> units) {}
    static Unit unit(String name, String language, String source) {
        return new Unit("memory:/" + name + "." + language, language, source);
    }
    static List<String> path(Element element) {
        var result = new ArrayList<String>();
        for (Element cursor = element; cursor != null; cursor = org.omg.sysml.util.NamespaceUtil.getParentNamespaceOf(cursor)) {
            if (cursor.getDeclaredName() != null) result.add(org.omg.sysml.util.ElementUtil.unescapeString(cursor.getDeclaredName()));
        }
        Collections.reverse(result);
        return result;
    }
    public static void main(String[] args) throws Exception {
        org.omg.sysml.logic.SysMLLogicStandaloneSetup.doSetup();
        org.eclipse.emf.ecore.EPackage.Registry.INSTANCE.put("https://www.omg.org/spec/SysML/20250201", SysMLPackage.eINSTANCE);
        var kerml = new org.omg.kerml.xtext.KerMLStandaloneSetup().createInjectorAndDoEMFRegistration();
        var sysml = new org.omg.sysml.xtext.SysMLStandaloneSetup().createInjectorAndDoEMFRegistration();
        // Use Xtext's live ResourceSet descriptions, retaining Pilot's global scope,
        // resource-description managers, qualified-name providers and lazy linker.
        for (var injector : List.of(kerml, sysml)) {
            var provider = injector.getInstance(ResourceDescriptionsProvider.class);
            provider.setLiveScopeResourceDescriptions(() -> injector.getInstance(ResourceSetBasedResourceDescriptions.class));
        }
        var controls = new ArrayList<Control>();
        for (String language : List.of("kerml", "sysml")) {
            String type = language.equals("kerml") ? "class" : "part def";
            String usage = language.equals("kerml") ? "feature" : "part";
            var provider = unit("provider", language, "package A { " + type + " T; }");
            var qualified = unit("consumer", language, "package B { " + usage + " x : A::T; }");
            controls.add(new Control(language + "_qualified_forward", List.of(qualified, provider)));
            controls.add(new Control(language + "_qualified_reverse", List.of(provider, qualified)));
            controls.add(new Control(language + "_import_forward", List.of(unit("consumer", language, "package B { private import A::*; " + usage + " x : T; }"), provider)));
        }
        controls.add(new Control("kerml_alias_forward", List.of(unit("consumer", "kerml", "package B { feature x : A::Alias; }"), unit("provider", "kerml", "package A { alias Alias for T; class T; }"))));
        controls.add(new Control("kerml_private_export", List.of(unit("consumer", "kerml", "package B { feature x : A::T; }"), unit("provider", "kerml", "package A { private class T; }"))));
        controls.add(new Control("kerml_missing_endpoint", List.of(unit("consumer", "kerml", "package B { feature x : A::Missing; }"), unit("provider", "kerml", "package A { class T; }"))));
        var duplicateConsumer = unit("consumer", "kerml", "package B { feature x : A::T; }");
        var first = unit("first", "kerml", "package A { class T; }");
        var second = unit("second", "kerml", "package A { class T; }");
        controls.add(new Control("duplicate_export_first", List.of(duplicateConsumer, first, second)));
        controls.add(new Control("duplicate_export_reverse", List.of(duplicateConsumer, second, first)));
        controls.add(new Control("mixed_provider", List.of(unit("consumer", "sysml", "package B { attribute x : A::T; }"), unit("provider", "kerml", "package A { datatype T; }"))));
        var results = new ArrayList<Object>();
        for (var control : controls) {
            var set = kerml.getInstance(XtextResourceSet.class);
            set.getLoadOptions().put(ResourceDescriptionsProvider.LIVE_SCOPE, Boolean.TRUE);
            var resources = new ArrayList<XtextResource>();
            for (var unit : control.units()) {
                var resource = (XtextResource) set.createResource(URI.createURI(unit.uri()));
                resource.load(new ByteArrayInputStream(unit.source().getBytes(StandardCharsets.UTF_8)), Map.of());
                if (!resource.getErrors().isEmpty()) throw new IllegalStateException("Syntax control failed: " + control.name() + resource.getErrors());
                resources.add(resource);
            }
            var descriptions = kerml.getInstance(ResourceDescriptionsProvider.class).getResourceDescriptions(set);
            var exports = new ArrayList<Object>();
            for (var description : descriptions.getAllResourceDescriptions()) {
                for (var exported : description.getExportedObjects()) {
                    exports.add(Map.of("name", exported.getName().toString(), "uri", exported.getEObjectURI().toString(), "kind", exported.getEClass().getName()));
                }
            }
            exports.sort(Comparator.comparing(Object::toString));
            var links = new ArrayList<Object>();
            var diagnostics = new ArrayList<Object>();
            for (var resource : resources) {
                var iterator = resource.getAllContents();
                while (iterator.hasNext()) {
                    var object = iterator.next();
                    if (object instanceof FeatureTyping typing) {
                        var target = typing.getType();
                        var link = new TreeMap<String,Object>();
                        link.put("source_uri", resource.getURI().toString());
                        link.put("owner_path", path(typing.getOwningFeature()));
                        link.put("field", "type");
                        link.put("resolved", target != null && !target.eIsProxy());
                        if (target != null && !target.eIsProxy()) {
                            link.put("target_uri", target.eResource().getURI().toString());
                            link.put("target_path", path(target));
                            link.put("target_fragment", target.eResource().getURIFragment(target));
                            link.put("target_kind", target.eClass().getName());
                        }
                        links.add(link);
                    }
                }
                for (var diagnostic : resource.getErrors()) {
                    diagnostics.add(Map.of("source_uri", resource.getURI().toString(), "message", diagnostic.getMessage(), "line", diagnostic.getLine(), "column", diagnostic.getColumn()));
                }
            }
            results.add(Map.of("name", control.name(), "units", control.units(), "description_service", descriptions.getClass().getName(), "exports", exports, "links", links, "diagnostics", diagnostics));
            for (var resource : resources) resource.unload();
        }
        Files.writeString(Path.of(args[0]), new GsonBuilder().setPrettyPrinting().create().toJson(results), StandardCharsets.UTF_8);
    }
}
