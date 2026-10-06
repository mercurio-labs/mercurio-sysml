package dev.mercurio.pilot;

import java.nio.file.*;
import java.util.*;
import java.lang.reflect.Method;
import com.google.gson.*;
import org.eclipse.emf.common.util.URI;
import org.eclipse.emf.ecore.*;
import org.omg.sysml.interactive.SysMLInteractive;
import org.omg.sysml.lang.sysml.*;
import org.omg.sysml.adapter.*;
import org.omg.sysml.util.FeatureUtil;
import org.omg.sysml.util.TypeUtil;

/** Independent observations of two named crossing producers on original pinned
 * library EObjects. Full resource transformation/validation is not claimed. */
public final class PilotLibraryCrossingReference {
    private static final Gson JSON=new GsonBuilder().serializeNulls().disableHtmlEscaping().setPrettyPrinting().create();
    private static final Map<EObject,String> ROLES=new IdentityHashMap<>();
    private static JsonObject identity(EObject value) {
        if(value==null||value.eIsProxy())throw new IllegalStateException("Absent/proxy endpoint");
        var row=new JsonObject();row.addProperty("kind",value.eClass().getName());
        if(ROLES.containsKey(value)){row.addProperty("local_role",ROLES.get(value));return row;}
        if(value.eResource()==null)throw new IllegalStateException("Unregistered detached endpoint");
        String path=value.eResource().getURI().toFileString().replace((char)92,'/');
        int index=path.indexOf("/sysml.library/");row.addProperty("resource",index<0?path:path.substring(index+1));
        row.addProperty("emf_fragment",value.eResource().getURIFragment(value));return row;
    }
    private static JsonArray identities(Collection<? extends EObject> nodes) {
        var result=new JsonArray();for(EObject node:nodes)result.add(identity(node));return result;
    }
    private static int count(org.eclipse.emf.ecore.resource.Resource resource) {
        int count=0;var all=resource.getAllContents();while(all.hasNext()){all.next();count++;}return count;
    }
    private static JsonObject relation(Specialization node) {
        var result=new JsonObject();result.addProperty("kind",node.eClass().getName());
        result.addProperty("is_implied",node.isImplied());result.addProperty("is_implied_included",node.isImpliedIncluded());
        result.add("specific",identity(node.getSpecific()));result.add("general",identity(node.getGeneral()));
        result.add("owned_related_element",identities(node.getOwnedRelatedElement()));
        result.add("owning_related_element",identity(node.getOwningRelatedElement()));return result;
    }
    public static void main(String[] args)throws Exception {
        Path library=Path.of(args[0]).toAbsolutePath().normalize();
        var spec=JsonParser.parseString(Files.readString(Path.of(args[1]))).getAsJsonObject();
        System.setProperty("org.eclipse.emf.common.util.ReferenceClearingQueue","false");
        var interactive=SysMLInteractive.getInstance();interactive.getLibraryIndexCache().setIndexDisabled(true);
        interactive.setVerbose(false);interactive.loadLibrary(library.toString().replace((char)92,'/'));
        var observations=new JsonArray();
        for(var item:spec.getAsJsonArray("controls")) {
            var control=item.getAsJsonObject();var row=new JsonObject();row.add("control",control);
            Path source=Path.of(control.get("source_file").getAsString()).toAbsolutePath().normalize();
            if(!source.startsWith(library))throw new IllegalArgumentException("Anchor outside pinned library");
            var resource=interactive.getResourceSet().getResource(URI.createFileURI(source.toString().replace((char)92,'/')),true);
            if(!(resource.getEObject(control.get("emf_fragment").getAsString()) instanceof Feature end))throw new IllegalStateException("Original Feature anchor absent");
            var cross=FeatureUtil.getOwnedCrossFeatureOf(end);if(cross==null)throw new IllegalStateException("Original cross Feature absent");
            ROLES.clear();row.addProperty("nodes_before",count(resource));row.addProperty("end_completion_before",end.isImpliedIncluded());
            row.addProperty("cross_completion_before",cross.isImpliedIncluded());
            ((FeatureAdapter)ElementAdapterFactory.getAdapter(end)).addCrossingSpecialization();
            TypeUtil.insertImplicitSpecializations(end);
            var crossing=end.getOwnedCrossSubsetting();if(crossing==null)throw new IllegalStateException("Crossing producer established no relation");
            var chain=crossing.getCrossedFeature();if(chain==null)throw new IllegalStateException("Crossed chain absent");
            ROLES.put(crossing,"crossing");ROLES.put(chain,"chain");
            row.add("crossing",relation(crossing));
            var chaining=new JsonArray();int ordinal=0;
            for(var link:chain.getOwnedFeatureChaining()) {
                ROLES.put(link,"chain-link-"+ordinal++);var linkRow=new JsonObject();
                linkRow.addProperty("kind",link.eClass().getName());linkRow.addProperty("is_implied",link.isImplied());
                linkRow.addProperty("is_implied_included",link.isImpliedIncluded());linkRow.add("chaining_feature",identity(link.getChainingFeature()));
                linkRow.add("owning_related_element",identity(link.getOwningRelatedElement()));
                linkRow.add("owned_related_element",identities(link.getOwnedRelatedElement()));chaining.add(linkRow);
            }
            row.add("ordered_chain",chaining);row.addProperty("chain_kind",chain.eClass().getName());
            row.addProperty("chain_completion",chain.isImpliedIncluded());row.add("chain_owning_relationship",identity(chain.getOwningRelationship()));
            Method method=FeatureAdapter.class.getDeclaredMethod("addOwnedCrossFeatureSpecialization");method.setAccessible(true);
            method.invoke(ElementAdapterFactory.getAdapter(cross));
            TypeUtil.insertImplicitSpecializations(cross);
            var effects=new JsonArray();
            for(var owned:cross.getOwnedRelationship())if(owned instanceof Specialization specialization)effects.add(relation(specialization));
            row.add("cross_specializations",effects);row.addProperty("end_completion_after",end.isImpliedIncluded());
            row.addProperty("cross_completion_after",cross.isImpliedIncluded());row.addProperty("nodes_after",count(resource));
            row.addProperty("status","crossing_producers_observed");observations.add(row);
        }
        var output=new JsonObject();output.addProperty("schema","dev.mercurio.library-crossing-reference.v1");
        output.addProperty("qualification_certificate",false);output.addProperty("stage","FeatureAdapter.addCrossingSpecialization and addOwnedCrossFeatureSpecialization, each followed by TypeUtil.insertImplicitSpecializations");
        output.add("observations",observations);Files.writeString(Path.of(args[2]),JSON.toJson(output));
    }
}
