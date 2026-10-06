package dev.mercurio.pilot;
import java.nio.file.*;
import java.util.*;
import java.lang.reflect.Method;
import java.util.function.BiConsumer;
import com.google.gson.*;
import org.eclipse.emf.common.util.URI;
import org.eclipse.emf.ecore.*;
import org.omg.sysml.interactive.SysMLInteractive;
import org.omg.sysml.lang.sysml.*;
import org.omg.sysml.adapter.*;
import org.omg.sysml.util.ElementUtil;

/** Independent build-time evidence for the fixed reference-binding stage.
 * Enclosing expression/resource completion and validation remain separate. */
public final class PilotReferenceBindingContextReference {
    private static final Gson JSON=new GsonBuilder().serializeNulls().disableHtmlEscaping().setPrettyPrinting().create();
    private static final Map<EObject,String> ROLES=new IdentityHashMap<>();
    private static JsonObject identity(EObject value) {
        if(ROLES.containsKey(value)){var local=new JsonObject();local.addProperty("kind",value.eClass().getName());local.addProperty("local_role",ROLES.get(value));return local;}
        if(value==null||value.eIsProxy()||value.eResource()==null)throw new IllegalStateException("Unresolved/detached external endpoint");
        var result=new JsonObject();result.addProperty("kind",value.eClass().getName());
        String path=value.eResource().getURI().toFileString().replace((char)92,'/');
        int index=path.indexOf("/sysml.library/");result.addProperty("resource",index<0?path:path.substring(index+1));
        result.addProperty("emf_fragment",value.eResource().getURIFragment(value));return result;
    }
    private static void invoke(java.lang.Class<?> type,String name,Object adapter)throws Exception {
        Method method=type.getDeclaredMethod(name);method.setAccessible(true);method.invoke(adapter);
    }
    private static JsonObject feature(Feature node) {
        var row=new JsonObject();row.addProperty("kind",node.eClass().getName());
        row.addProperty("is_implied_included",node.isImpliedIncluded());
        row.addProperty("is_end",node.isEnd());
        var effects=new JsonArray();
        for(Relationship relation:node.getOwnedRelationship()) {
            if(relation instanceof Membership)continue;
            var effect=new JsonObject();effect.addProperty("kind",relation.eClass().getName());
            effect.addProperty("is_implied",relation.isImplied());
            if(relation instanceof Specialization s)effect.add("target",identity(s.getGeneral()));
            else if(relation instanceof TypeFeaturing f)effect.add("target",identity(f.getFeaturingType()));
            else throw new IllegalStateException("Unassessed binding relation "+relation.eClass().getName());
            effects.add(effect);
        }
        row.add("effects",effects);return row;
    }
    private static void prepareFeaturing(Feature feature,Set<Feature> seen)throws Exception {
        if(!seen.add(feature))return;
        Namespace namespace=feature.getOwningNamespace();
        if(namespace instanceof Feature owner)prepareFeaturing(owner,seen);
        var adapter=(FeatureAdapter)ElementAdapterFactory.getAdapter(feature);
        invoke(FeatureAdapter.class,"computeFeaturingType",adapter);
        adapter.addImplicitFeaturingTypes();
    }
    public static void main(String[] args)throws Exception {
        var spec=JsonParser.parseString(Files.readString(Path.of(args[1]))).getAsJsonObject();
        System.setProperty("org.eclipse.emf.common.util.ReferenceClearingQueue","false");
        var interactive=SysMLInteractive.getInstance();interactive.getLibraryIndexCache().setIndexDisabled(true);
        interactive.setVerbose(false);interactive.loadLibrary(args[0].replace((char)92,'/'));
        var set=interactive.getResourceSet();
        for(var item:spec.getAsJsonArray("source_files"))
            set.getResource(URI.createFileURI(Path.of(item.getAsString()).toString().replace((char)92,'/')),true);
        var rows=new JsonArray();
        for(var item:spec.getAsJsonArray("expressions")) {
            var control=item.getAsJsonObject();var row=new JsonObject();row.add("control",control);
            var resource=set.getResource(URI.createFileURI(Path.of(control.get("source_file").getAsString()).toString().replace((char)92,'/')),true);
            var owner=(FeatureReferenceExpression)resource.getEObject(control.get("emf_fragment").getAsString());
            row.addProperty("completion_before",owner.isImpliedIncluded());
            if(control.get("link_negative").getAsBoolean()) {
                row.addProperty("status","link_negative_not_qualified");
                row.addProperty("completion_after",owner.isImpliedIncluded());rows.add(row);continue;
            }
            try {
                var adapter=(FeatureReferenceExpressionAdapter)ElementAdapterFactory.getAdapter(owner);
                adapter.addAdditionalMembers();
                invoke(FeatureReferenceExpressionAdapter.class,"addResultSubsetting",adapter);
                adapter.computeImplicitGeneralTypes();adapter.removeUnnecessaryImplicitGeneralTypes();
                var featuringVisited=Collections.newSetFromMap(new IdentityHashMap<Feature,Boolean>());
                prepareFeaturing(owner,featuringVisited);
                prepareFeaturing(owner.getReferent(),featuringVisited);
                prepareFeaturing(owner.getResult(),featuringVisited);
                invoke(FeatureReferenceExpressionAdapter.class,"addReferenceConnector",adapter);
                var bindings=new JsonArray();
                adapter.forEachImplicitBindingConnector((BiConsumer<BindingConnector,EClass>)(connector,membership)->{
                    ROLES.clear();ROLES.put(connector,"connector");
                    int ordinal=0;for(Feature end:connector.getOwnedFeature())if(end.isEnd())ROLES.put(end,"end."+(ordinal++));
                    ElementUtil.transformAll(connector,true);
                    var binding=feature(connector);binding.addProperty("membership",membership.getName());
                    var ends=new JsonArray();
                    for(Feature end:connector.getOwnedFeature())if(end.isEnd())ends.add(feature(end));
                    binding.add("ends",ends);bindings.add(binding);
                });
                row.add("referent",identity(owner.getReferent()));row.add("result",identity(owner.getResult()));
                row.add("bindings",bindings);row.addProperty("status","reference_observed");
            }catch(Exception failure) {
                row.addProperty("status","reference_stage_failed");row.addProperty("error",failure.toString());
                Throwable cause=failure;while(cause.getCause()!=null)cause=cause.getCause();
                row.addProperty("root_error",cause.toString());
            }
            row.addProperty("completion_after",owner.isImpliedIncluded());rows.add(row);
            System.err.println(control.get("context").getAsString()+" "+row.get("status").getAsString());
        }
        var output=new JsonObject();output.addProperty("schema","dev.mercurio.reference-binding-context-reference.v1");
        output.addProperty("qualification_certificate",false);
        output.addProperty("enclosing_transform_validation","not_assessed");
        output.add("observations",rows);Files.writeString(Path.of(args[2]),JSON.toJson(output));
    }
}
