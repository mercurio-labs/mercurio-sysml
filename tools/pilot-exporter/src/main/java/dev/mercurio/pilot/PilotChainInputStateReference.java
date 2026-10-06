package dev.mercurio.pilot;

import java.nio.charset.StandardCharsets;
import java.nio.file.*;
import java.util.*;
import com.google.gson.*;
import org.eclipse.emf.common.util.URI;
import org.eclipse.emf.ecore.*;
import org.omg.sysml.interactive.SysMLInteractive;
import org.omg.sysml.lang.sysml.Element;

/** Build-time independent getters for imported Ecore provider reference contracts.
 * No explicit transformation, input publication, native endpoint map or prepared flags.
 * Any state changes caused by an upstream getter are recorded, never suppressed. */
public final class PilotChainInputStateReference {
    private static final Gson JSON = new GsonBuilder().serializeNulls().disableHtmlEscaping().setPrettyPrinting().create();
    private static String resourceName(EObject value) {
        String name=value.eResource().getURI().toFileString().replace((char)92,'/');
        int index=name.indexOf("/sysml.library/");
        return index>=0?name.substring(index+1):name;
    }
    private static JsonObject identity(EObject value) {
        var result=new JsonObject();
        result.addProperty("kind",value.eClass().getName());
        result.addProperty("resource",resourceName(value));
        result.addProperty("emf_fragment",value.eResource().getURIFragment(value));
        if(value instanceof Element element)result.addProperty("qualified_name",element.getQualifiedName());
        return result;
    }
    private static String fragment(String nativeId) {
        String[] parts=nativeId.split("\\.");
        if(parts.length<3||!parts[0].equals("definition")||!parts[1].equals("resource"))throw new IllegalArgumentException("Not a native resource identity");
        if((parts.length-3)%2!=0)throw new IllegalArgumentException("Not a canonical containment path");
        var fields=Map.of("owned_relationship","ownedRelationship","owned_related_element","ownedRelatedElement");
        var result=new StringBuilder("/");
        for(int i=3;i<parts.length;i+=2) {
            String name=fields.get(parts[i]);
            if(name==null)throw new IllegalArgumentException("Unsupported canonical containment field");
            int ordinal=Integer.parseInt(parts[i+1]);
            if(ordinal<0)throw new IllegalArgumentException("Negative containment ordinal");
            result.append("/@").append(name).append('.').append(ordinal);
        }
        return result.toString();
    }
    private static int nodeCount(org.eclipse.emf.ecore.resource.Resource resource) {
        int count=0;var all=resource.getAllContents();while(all.hasNext()){all.next();count++;}return count;
    }
    public static void main(String[] args) throws Exception {
        Path library=Path.of(args[0]).toAbsolutePath().normalize();
        var spec=JsonParser.parseString(Files.readString(Path.of(args[1]))).getAsJsonObject();
        System.setProperty("org.eclipse.emf.common.util.ReferenceClearingQueue","false");
        var interactive=SysMLInteractive.getInstance();
        interactive.getLibraryIndexCache().setIndexDisabled(true);
        interactive.setVerbose(false);
        interactive.loadLibrary(library.toString().replace((char)92,'/'));
        var set=interactive.getResourceSet();
        var admitted=new HashSet<Path>();
        for(var item:spec.getAsJsonArray("source_files")) admitted.add(Path.of(item.getAsString()).toAbsolutePath().normalize());
        var initialCounts=new JsonObject();
        for(Path source:admitted) {
            var resource=set.getResource(URI.createFileURI(source.toString().replace((char)92,'/')),true);
            initialCounts.addProperty(source.toString().replace((char)92,'/'),nodeCount(resource));
        }
        var observations=new JsonArray();
        for(var item:spec.getAsJsonArray("ports")) {
            var port=item.getAsJsonObject();
            String nativeId=port.get("owner_id").getAsString();
            String[] parts=nativeId.split("\\.");
            String file=new String(HexFormat.of().parseHex(parts[2]),StandardCharsets.UTF_8);
            Path source=Path.of(file).toAbsolutePath().normalize();
            if(!source.startsWith(library)&&!admitted.contains(source))throw new IllegalArgumentException("Port outside pinned library/source allowlist");
            var resource=set.getResource(URI.createFileURI(source.toString().replace((char)92,'/')),true);
            EObject owner=resource.getEObject(port.get("emf_fragment").getAsString());
            if(owner==null||!owner.eClass().getName().equals(port.get("owner_kind").getAsString().replace("SysML::","")))
                throw new IllegalStateException("Canonical owner/metaclass mismatch for "+port.get("context").getAsString()+"."+port.get("field").getAsString()+": expected "+port.get("owner_kind").getAsString()+", found "+(owner==null?"absent":owner.eClass().getName()));
            String field=port.get("feature_name").getAsString();
            var feature=owner.eClass().getEStructuralFeature(field);
            if(!(feature instanceof EReference))
                throw new IllegalStateException("Expected imported EReference: "+field);
            boolean before=owner instanceof Element e&&e.isImpliedIncluded();
            int beforeNodes=nodeCount(resource);
            int beforeOwned=owner instanceof Element e?e.getOwnedRelationship().size():0;
            var endpoints=new JsonArray();
            var row=new JsonObject();
            row.add("root",port);row.add("owner",identity(owner));
            row.addProperty("feature",feature.getName());
            row.addProperty("feature_declaring_kind",feature.getEContainingClass().getName());
            row.addProperty("derived",feature.isDerived());row.addProperty("many",feature.isMany());
            try {
                Object target=owner.eGet(feature,true);
                Iterable<?> values=feature.isMany()?(Iterable<?>)target:target==null?List.of():List.of(target);
                for(Object value:values) {
                    if(!(value instanceof EObject endpoint)||endpoint.eIsProxy()||!feature.getEType().isInstance(endpoint))
                        throw new IllegalStateException("Unresolved or ill-typed Pilot endpoint: "+field);
                    endpoints.add(identity(endpoint));
                }
                row.addProperty("status","reference_observed");
            } catch(Exception failure) {
                row.addProperty("status","reference_getter_failed");
                row.addProperty("failure_type",failure.getClass().getName());
                row.addProperty("error",failure.getMessage());
            }
            boolean after=owner instanceof Element e&&e.isImpliedIncluded();
            row.add("endpoints",endpoints);
            row.addProperty("provider_completion_before",before);row.addProperty("provider_completion_after",after);
            row.addProperty("source_nodes_before",beforeNodes);row.addProperty("source_nodes_after",nodeCount(resource));
            row.addProperty("owner_relationships_before",beforeOwned);
            row.addProperty("owner_relationships_after",owner instanceof Element e?e.getOwnedRelationship().size():0);
            observations.add(row);
            System.err.println("Context "+port.get("context").getAsString()+" "+port.get("field").getAsString()+" -> "+endpoints.size()+" endpoints");
        }
        var result=new JsonObject();result.addProperty("schema","dev.mercurio.lifecycle-shared-service-reference.v1");
        result.addProperty("mutations_applied",false);
        result.addProperty("native_qualification","not_assessed");result.addProperty("qualification_certificate",false);
        result.addProperty("scope","Ordered imported Ecore provider reference getters on pinned resources. Derived algorithms execute in upstream tooling. Provider state changes are recorded; source/library validation and complete transformation are not assessed.");
        result.add("initial_source_node_counts",initialCounts);
        var finalCounts=new JsonObject();
        for(Path source:admitted) {
            var resource=set.getResource(URI.createFileURI(source.toString().replace((char)92,'/')),true);
            finalCounts.addProperty(source.toString().replace((char)92,'/'),nodeCount(resource));
        }
        result.add("final_source_node_counts",finalCounts);result.add("observations",observations);

        // Inspect the already-observed chain inputs only after every fixed read.
        // This supplemental state is diagnostic evidence, not a native answer.
        var chainState = new JsonArray();
        var seen = new HashSet<String>();
        for (var item: spec.getAsJsonArray("ports")) {
            var port = item.getAsJsonObject();
            if (!port.get("strategy_group").getAsString().equals("registered_chain_producer_readiness")
                    || !seen.add(port.get("owner_id").getAsString())) continue;
            String nativeId = port.get("owner_id").getAsString();
            String file = new String(HexFormat.of().parseHex(nativeId.split("\\.")[2]), StandardCharsets.UTF_8);
            var resource = set.getResource(URI.createFileURI(file.replace((char)92,'/')), true);
            var parameter = (org.omg.sysml.lang.sysml.Feature)resource.getEObject(port.get("emf_fragment").getAsString());
            var row = new JsonObject();
            row.add("root", port);
            row.add("parameter", identity(parameter));
            row.addProperty("direction", parameter.getDirection() == null ? null : parameter.getDirection().getLiteral());
            row.addProperty("parameter_complete", parameter.isImpliedIncluded());
            var owningType = parameter.getOwningType();
            row.add("owning_type", owningType == null ? JsonNull.INSTANCE : identity(owningType));
            row.addProperty("owning_type_complete", owningType != null && owningType.isImpliedIncluded());
            var children = new JsonArray();
            for (var child: parameter.getOwnedFeature()) {
                var state = new JsonObject();
                state.add("feature", identity(child));
                state.addProperty("declared_name", child.getDeclaredName());
                state.addProperty("direction", child.getDirection() == null ? null : child.getDirection().getLiteral());
                state.addProperty("complete", child.isImpliedIncluded());
                var implicit = new JsonArray();
                var adapter = (org.omg.sysml.adapter.TypeAdapter)org.omg.sysml.adapter.ElementAdapterFactory.getAdapter(child);
                for (var endpoint: adapter.getImplicitGeneralTypesOnly(org.omg.sysml.lang.sysml.SysMLPackage.Literals.REDEFINITION))
                    implicit.add(identity(endpoint));
                state.add("cached_implicit_redefinitions", implicit);
                var computed = new JsonArray();
                for (var endpoint: org.omg.sysml.util.FeatureUtil.getRedefinedFeaturesWithComputedOf(child))
                    computed.add(identity(endpoint));
                state.add("computed_redefinitions", computed);
                children.add(state);
            }
            row.add("owned_features", children);
            chainState.add(row);
        }
        result.add("chain_input_state", chainState);
        Files.writeString(Path.of(args[2]),JSON.toJson(result));
    }
}
