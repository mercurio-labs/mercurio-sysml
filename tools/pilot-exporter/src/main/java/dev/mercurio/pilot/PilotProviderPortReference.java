package dev.mercurio.pilot;

import java.nio.charset.StandardCharsets;
import java.nio.file.*;
import java.util.*;
import com.google.gson.*;
import org.eclipse.emf.common.util.URI;
import org.eclipse.emf.ecore.*;
import org.omg.sysml.interactive.SysMLInteractive;
import org.omg.sysml.lang.sysml.Element;

/** Build-time reference getters for exact imported stored ports. No transformation,
 * input publication, endpoint map for native execution or prepared flags. */
public final class PilotProviderPortReference {
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
    public static void main(String[] args) throws Exception {
        Path library=Path.of(args[0]).toAbsolutePath().normalize();
        var spec=JsonParser.parseString(Files.readString(Path.of(args[1]))).getAsJsonObject();
        System.setProperty("org.eclipse.emf.common.util.ReferenceClearingQueue","false");
        var interactive=SysMLInteractive.getInstance();
        interactive.getLibraryIndexCache().setIndexDisabled(true);
        interactive.setVerbose(false);
        interactive.loadLibrary(library.toString().replace((char)92,'/'));
        var set=interactive.getResourceSet();
        var observations=new JsonArray();
        for(var item:spec.getAsJsonArray("ports")) {
            var port=item.getAsJsonObject();
            String nativeId=port.get("owner_id").getAsString();
            String[] parts=nativeId.split("\\.");
            String file=new String(HexFormat.of().parseHex(parts[2]),StandardCharsets.UTF_8);
            Path source=Path.of(file).toAbsolutePath().normalize();
            if(!source.startsWith(library))throw new IllegalArgumentException("Port outside pinned library");
            var resource=set.getResource(URI.createFileURI(source.toString().replace((char)92,'/')),true);
            EObject owner=resource.getEObject(fragment(nativeId));
            if(owner==null||!owner.eClass().getName().equals(port.get("owner_kind").getAsString().replace("SysML::","")))
                throw new IllegalStateException("Canonical owner/metaclass mismatch: "+nativeId);
            String importedFeature=port.get("feature_id").getAsString();
            String field=importedFeature.substring(importedFeature.lastIndexOf('/')+1);
            var feature=owner.eClass().getEStructuralFeature(field);
            if(!(feature instanceof EReference)||feature.isDerived()||feature.isVolatile()||feature.isMany())
                throw new IllegalStateException("Expected exact imported stored scalar reference: "+importedFeature);
            boolean before=owner instanceof Element e&&e.isImpliedIncluded();
            Object target=owner.eGet(feature,true);
            if(!(target instanceof EObject endpoint)||endpoint.eIsProxy()||!feature.getEType().isInstance(endpoint))
                throw new IllegalStateException("Unresolved or ill-typed Pilot endpoint: "+importedFeature);
            boolean after=owner instanceof Element e&&e.isImpliedIncluded();
            if(before!=after)throw new IllegalStateException("Stored port read changed provider completion");
            var row=new JsonObject();
            row.add("root",port);row.add("owner",identity(owner));row.add("endpoint",identity(endpoint));
            row.addProperty("feature",feature.getName());row.addProperty("feature_declaring_kind",feature.getEContainingClass().getName());
            row.addProperty("provider_completion_before",before);row.addProperty("provider_completion_after",after);
            row.addProperty("status","reference_observed");
            observations.add(row);
            System.err.println("Provider port "+port.get("field").getAsString()+" -> "+identity(endpoint).get("qualified_name"));
        }
        var result=new JsonObject();result.addProperty("schema","dev.mercurio.provider-port-reference.v1");
        result.addProperty("native_qualification","not_assessed");result.addProperty("qualification_certificate",false);
        result.addProperty("scope","Exact canonical stored reference getters; source/library validation and transformation completion are not assessed.");
        result.add("observations",observations);
        Files.writeString(Path.of(args[2]),JSON.toJson(result));
    }
}
