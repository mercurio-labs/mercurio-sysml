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

/** Independent build-time observations of FeatureAdapter.computeValueConnector.
 * No native endpoint hints or prepared lifecycle flags. Whole owner/resource
 * lifecycle and validation remain separate from this explicitly named stage. */
public final class PilotGeneralValueBindingReference {
    private static final Gson JSON=new GsonBuilder().serializeNulls().disableHtmlEscaping().setPrettyPrinting().create();
    private static final Map<EObject,String> ROLES=new IdentityHashMap<>();
    private static JsonObject identity(EObject value) {
        if(value==null||value.eIsProxy())throw new IllegalStateException("Absent/proxy endpoint");
        var row=new JsonObject();row.addProperty("kind",value.eClass().getName());
        if(ROLES.containsKey(value)){row.addProperty("local_role",ROLES.get(value));return row;}
        if(value.eResource()==null)throw new IllegalStateException("Unregistered detached endpoint");
        String path=value.eResource().getURI().toFileString().replace((char)92,'/');
        int start=path.indexOf("/sysml.library/");row.addProperty("resource",start<0?path:path.substring(start+1));
        row.addProperty("emf_fragment",value.eResource().getURIFragment(value));
        if(value instanceof Element element)row.addProperty("qualified_name",element.getQualifiedName());
        return row;
    }
    private static int count(org.eclipse.emf.ecore.resource.Resource resource) {
        int n=0;var iterator=resource.getAllContents();while(iterator.hasNext()){iterator.next();n++;}return n;
    }
    private static JsonArray identities(Collection<? extends EObject> values) {
        var rows=new JsonArray();for(EObject value:values)rows.add(identity(value));return rows;
    }
    private static JsonObject feature(Feature node) {
        var row=new JsonObject();row.addProperty("kind",node.eClass().getName());
        row.addProperty("is_implied_included",node.isImpliedIncluded());row.addProperty("is_end",node.isEnd());
        row.addProperty("is_variable",node.isVariable());row.addProperty("is_composite",node.isComposite());row.addProperty("is_portion",node.isPortion());
        row.addProperty("direction",node.getDirection()==null?null:node.getDirection().getLiteral());
        var effects=new JsonArray();
        for(Relationship relation:node.getOwnedRelationship()) {
            if(relation instanceof Membership)continue;
            var effect=new JsonObject();effect.addProperty("kind",relation.eClass().getName());effect.addProperty("is_implied",relation.isImplied());
            if(relation instanceof Specialization s)effect.add("target",identity(s.getGeneral()));
            else if(relation instanceof TypeFeaturing f)effect.add("target",identity(f.getFeaturingType()));
            else if(relation instanceof FeatureChaining f)effect.add("target",identity(f.getChainingFeature()));
            else {effect.addProperty("unassessed_relation",true);effect.add("identity",identity(relation));}
            effects.add(effect);
        }
        row.add("effects",effects);row.add("type",identities(node.getType()));row.add("featuring_type",identities(node.getFeaturingType()));
        row.add("chaining_feature",identities(node.getChainingFeature()));return row;
    }
    public static void main(String[] args)throws Exception {
        Path library=Path.of(args[0]).toAbsolutePath().normalize();
        var spec=JsonParser.parseString(Files.readString(Path.of(args[1]))).getAsJsonObject();
        System.setProperty("org.eclipse.emf.common.util.ReferenceClearingQueue","false");
        var interactive=SysMLInteractive.getInstance();interactive.getLibraryIndexCache().setIndexDisabled(true);
        interactive.setVerbose(false);interactive.loadLibrary(library.toString().replace((char)92,'/'));
        var set=interactive.getResourceSet();var admitted=new HashSet<Path>();
        for(var item:spec.getAsJsonArray("source_files")) {
            Path source=Path.of(item.getAsString()).toAbsolutePath().normalize();admitted.add(source);
            set.getResource(URI.createFileURI(source.toString().replace((char)92,'/')),true);
        }
        var rows=new JsonArray();
        for(var item:spec.getAsJsonArray("valuations")) {
            var control=item.getAsJsonObject();var row=new JsonObject();row.add("control",control);
            if(!control.get("eligible").getAsBoolean()) {
                row.addProperty("status","required_link_negative_not_qualified");rows.add(row);continue;
            }
            Path source=Path.of(control.get("source_file").getAsString()).toAbsolutePath().normalize();
            if(!admitted.contains(source)||source.startsWith(library))throw new IllegalArgumentException("Valuation outside original source allowlist");
            var resource=set.getResource(URI.createFileURI(source.toString().replace((char)92,'/')),true);
            EObject anchor=resource.getEObject(control.get("valuation_fragment").getAsString());
            if(!(anchor instanceof FeatureValue value)||value.eIsProxy())throw new IllegalStateException("Canonical valuation anchor absent");
            Feature owner=value.getFeatureWithValue();Expression expression=value.getValue();
            if(owner==null||expression==null)throw new IllegalStateException("Missing imported valuation ownership/value endpoint");
            ROLES.clear();ROLES.put(owner,"owner");ROLES.put(expression,"value");
            row.addProperty("owner_completion_before",owner.isImpliedIncluded());
            row.addProperty("expression_completion_before",expression.isImpliedIncluded());row.addProperty("nodes_before",count(resource));
            row.addProperty("is_default",value.isDefault());row.addProperty("is_initial",value.isInitial());
            try {
                var adapter=(FeatureAdapter)ElementAdapterFactory.getAdapter(owner);
                Method method=FeatureAdapter.class.getDeclaredMethod("computeValueConnector");method.setAccessible(true);method.invoke(adapter);
                Feature result=expression.getResult();if(result!=null)ROLES.put(result,"value_result");
                var bindings=new JsonArray();
                adapter.forEachImplicitBindingConnector((BiConsumer<BindingConnector,EClass>)(connector,membership)->{
                    ROLES.clear();ROLES.put(owner,"owner");ROLES.put(expression,"value");if(result!=null)ROLES.put(result,"value_result");
                    ROLES.put(connector,"connector");int ordinal=0;
                    for(Feature end:connector.getOwnedFeature())if(end.isEnd())ROLES.put(end,"end."+(ordinal++));
                    for(Feature end:connector.getOwnedFeature())if(end.isEnd())for(Relationship relation:end.getOwnedRelationship()) {
                        if(!(relation instanceof ReferenceSubsetting reference))continue;
                        Feature endpoint=reference.getReferencedFeature();
                        if(endpoint!=owner&&endpoint!=expression&&endpoint!=result&&!endpoint.getChainingFeature().isEmpty())ROLES.put(endpoint,"value_chain");
                    }
                    var connectorAdapter=(FeatureAdapter)ElementAdapterFactory.getAdapter(connector);
                    connectorAdapter.forEachImplicitFeaturingType(type->{
                        if(type instanceof Feature feature&&!feature.getChainingFeature().isEmpty()&&feature!=owner)ROLES.put(feature,"initial_context");
                    });
                    ElementUtil.transformAll(connector,true);
                    var binding=feature(connector);binding.addProperty("membership",membership.getName());
                    binding.add("related_feature",identities(connector.getRelatedFeature()));
                    var ends=new JsonArray();for(Feature end:connector.getOwnedFeature())if(end.isEnd())ends.add(feature(end));binding.add("ends",ends);
                    var chains=new JsonArray();
                    for(var entry:ROLES.entrySet())if(entry.getKey() instanceof Feature chain
                        &&(entry.getValue().equals("value_chain")||entry.getValue().equals("initial_context"))) {
                        var shape=feature(chain);shape.addProperty("local_role",entry.getValue());chains.add(shape);
                    }
                    binding.add("chains",chains);bindings.add(binding);
                });
                row.add("bindings",bindings);row.addProperty("status","reference_observed");
            }catch(Exception failure) {
                row.addProperty("status","reference_stage_failed");row.addProperty("error",failure.toString());
                Throwable cause=failure;while(cause.getCause()!=null)cause=cause.getCause();row.addProperty("root_error",cause.toString());
            }
            row.addProperty("owner_completion_after",owner.isImpliedIncluded());
            row.addProperty("expression_completion_after",expression.isImpliedIncluded());row.addProperty("nodes_after",count(resource));
            rows.add(row);System.err.println(control.get("context").getAsString()+" "+row.get("status").getAsString());
        }
        var output=new JsonObject();output.addProperty("schema","dev.mercurio.general-value-binding-reference.v1");
        output.addProperty("qualification_certificate",false);output.addProperty("stage","FeatureAdapter.computeValueConnector plus fresh connector transformAll");
        output.addProperty("enclosing_owner_transform_validation","not_assessed");output.add("observations",rows);
        Files.writeString(Path.of(args[2]),JSON.toJson(output));
    }
}
