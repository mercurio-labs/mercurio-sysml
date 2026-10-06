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
import org.omg.sysml.util.*;

/** Actual fixed first-library-feature producer and dependent wrapper lifecycle.
 * No full resource transformation, validation or release qualification. */
public final class PilotFirstLibraryFeaturingReference {
    private static final Gson JSON=new GsonBuilder().serializeNulls().disableHtmlEscaping().setPrettyPrinting().create();
    private static final Map<EObject,String> ROLES=new IdentityHashMap<>();
    private static JsonObject identity(EObject node) {
        if(node==null||node.eIsProxy())throw new IllegalStateException("Absent/proxy endpoint");
        var row=new JsonObject();row.addProperty("kind",node.eClass().getName());
        if(ROLES.containsKey(node)){row.addProperty("local_role",ROLES.get(node));return row;}
        if(node.eResource()==null)throw new IllegalStateException("Unregistered detached endpoint");
        String path=node.eResource().getURI().toFileString().replace((char)92,'/');int offset=path.indexOf("/sysml.library/");
        row.addProperty("resource",offset<0?path:path.substring(offset+1));row.addProperty("emf_fragment",node.eResource().getURIFragment(node));return row;
    }
    private static JsonArray identities(Collection<? extends EObject> nodes) {
        var rows=new JsonArray();for(EObject node:nodes)rows.add(identity(node));return rows;
    }
    private static int count(org.eclipse.emf.ecore.resource.Resource resource) {
        int count=0;var all=resource.getAllContents();while(all.hasNext()){all.next();count++;}return count;
    }
    private static Object invoke(Feature node,String name)throws Exception {
        Method method=FeatureAdapter.class.getDeclaredMethod(name);method.setAccessible(true);return method.invoke(ElementAdapterFactory.getAdapter(node));
    }
    private static JsonObject featuring(Feature node) {
        var row=new JsonObject();row.add("identity",identity(node));row.addProperty("is_implied_included",node.isImpliedIncluded());
        row.addProperty("is_variable",node.isVariable());row.add("owning_type",identity(node.getOwningType()));
        row.add("featuring_type",identities(node.getFeaturingType()));var relations=new JsonArray();
        for(TypeFeaturing relation:node.getOwnedTypeFeaturing()) {
            var effect=new JsonObject();effect.addProperty("kind",relation.eClass().getName());effect.addProperty("is_implied",relation.isImplied());
            effect.addProperty("is_implied_included",relation.isImpliedIncluded());effect.add("feature_of_type",identity(relation.getFeatureOfType()));
            effect.add("featuring_type",identity(relation.getFeaturingType()));effect.add("owning_related_element",identity(relation.getOwningRelatedElement()));
            effect.add("owned_related_element",identities(relation.getOwnedRelatedElement()));relations.add(effect);
        }
        row.add("owned_type_featuring",relations);return row;
    }
    private static JsonObject chain(Feature node) {
        var row=new JsonObject();row.addProperty("kind",node.eClass().getName());row.addProperty("is_implied_included",node.isImpliedIncluded());
        var effects=new JsonArray();for(Relationship relation:node.getOwnedRelationship()) {
            var effect=new JsonObject();effect.addProperty("kind",relation.eClass().getName());effect.addProperty("is_implied",relation.isImplied());
            if(relation instanceof FeatureChaining chaining)effect.add("target",identity(chaining.getChainingFeature()));
            else if(relation instanceof Specialization specialization)effect.add("target",identity(specialization.getGeneral()));
            else throw new IllegalStateException("Unassessed wrapper relationship "+relation.eClass().getName());effects.add(effect);
        }
        row.add("effects",effects);row.add("type",identities(node.getType()));row.add("featuring_type",identities(node.getFeaturingType()));
        row.add("chaining_feature",identities(node.getChainingFeature()));return row;
    }
    public static void main(String[] args)throws Exception {
        Path library=Path.of(args[0]).toAbsolutePath().normalize();var spec=JsonParser.parseString(Files.readString(Path.of(args[1]))).getAsJsonObject();
        System.setProperty("org.eclipse.emf.common.util.ReferenceClearingQueue","false");
        var interactive=SysMLInteractive.getInstance();interactive.getLibraryIndexCache().setIndexDisabled(true);interactive.setVerbose(false);
        interactive.loadLibrary(library.toString().replace((char)92,'/'));var set=interactive.getResourceSet();
        var features=new JsonArray();
        for(var item:spec.getAsJsonArray("library_features")) {
            var control=item.getAsJsonObject();Path source=Path.of(control.get("source_file").getAsString()).toAbsolutePath().normalize();
            if(!source.startsWith(library))throw new IllegalArgumentException("Feature outside actual pinned library");
            var resource=set.getResource(URI.createFileURI(source.toString().replace((char)92,'/')),true);
            if(!(resource.getEObject(control.get("emf_fragment").getAsString()) instanceof Feature feature)||feature.eIsProxy())throw new IllegalStateException("Canonical Feature absent");
            if(feature.isVariable()||feature.getOwningType()==null||!feature.eClass().getName().equals(control.get("kind").getAsString()))throw new IllegalStateException("Not fixed canonical Type-owned Feature");
            var row=new JsonObject();row.add("control",control);row.add("before",featuring(feature));row.addProperty("nodes_before",count(resource));
            var owner=(EObject)invoke(feature,"computeFeaturingType");row.add("producer_result",identity(owner));row.add("after_producer",featuring(feature));
            FeatureUtil.insertImplicitTypeFeaturings(feature);row.add("after_insertion",featuring(feature));int nodes=count(resource);row.addProperty("nodes_after",nodes);
            invoke(feature,"computeFeaturingType");FeatureUtil.insertImplicitTypeFeaturings(feature);row.add("after_replay",featuring(feature));row.addProperty("nodes_after_replay",count(resource));
            if(count(resource)!=nodes)throw new IllegalStateException("Featuring producer replay allocated nodes");features.add(row);
        }
        var wrappers=new JsonArray();
        for(var item:spec.getAsJsonArray("crossing_controls")) {
            var control=item.getAsJsonObject();Path source=Path.of(control.get("source_file").getAsString()).toAbsolutePath().normalize();
            if(!source.startsWith(library))throw new IllegalArgumentException("Crossing outside pinned library");
            var resource=set.getResource(URI.createFileURI(source.toString().replace((char)92,'/')),true);
            if(!(resource.getEObject(control.get("emf_fragment").getAsString()) instanceof Feature end))throw new IllegalStateException("Crossing end absent");
            var cross=FeatureUtil.getOwnedCrossFeatureOf(end);if(cross==null)throw new IllegalStateException("Owned cross absent");
            ((FeatureAdapter)ElementAdapterFactory.getAdapter(end)).addCrossingSpecialization();TypeUtil.insertImplicitSpecializations(end);
            var wrapper=end.getOwnedCrossSubsetting().getCrossedFeature();invoke(cross,"addOwnedCrossFeatureSpecialization");TypeUtil.insertImplicitSpecializations(cross);
            ElementUtil.transformAll(wrapper,true);var row=new JsonObject();row.addProperty("context","actual-Links-library");row.addProperty("role","cross_chain");row.addProperty("ordinal",0);row.add("shape",chain(wrapper));wrappers.add(row);
        }
        for(var item:spec.getAsJsonArray("initial_valuations")) {
            var control=item.getAsJsonObject();Path source=Path.of(control.get("source_file").getAsString()).toAbsolutePath().normalize();
            if(source.startsWith(library))throw new IllegalArgumentException("Initial valuation must be an original source");
            var resource=set.getResource(URI.createFileURI(source.toString().replace((char)92,'/')),true);
            if(!(resource.getEObject(control.get("valuation_fragment").getAsString()) instanceof FeatureValue value)||!value.isInitial())throw new IllegalStateException("Original initial valuation absent");
            var owner=value.getFeatureWithValue();var expression=value.getValue();if(owner==null||expression==null)throw new IllegalStateException("Initial ownership/value missing");
            var adapter=(FeatureAdapter)ElementAdapterFactory.getAdapter(owner);invoke(owner,"computeValueConnector");var result=expression.getResult();final int[] ordinal={0};
            adapter.forEachImplicitBindingConnector((BiConsumer<BindingConnector,EClass>)(connector,membership)->{
                ROLES.clear();ROLES.put(owner,"owner");ROLES.put(expression,"value");if(result!=null)ROLES.put(result,"value_result");
                var context=new ArrayList<Feature>();((FeatureAdapter)ElementAdapterFactory.getAdapter(connector)).forEachImplicitFeaturingType(type->{if(type instanceof Feature feature&&!feature.getChainingFeature().isEmpty()&&feature!=owner)context.add(feature);});
                var member=(OwningMembership)SysMLFactory.eINSTANCE.create(membership);member.setIsImplied(true);member.getOwnedRelatedElement().add(connector);owner.getOwnedRelationship().add(member);
                ElementUtil.transformAll(connector,true);
                for(Feature wrapper:context){ElementUtil.transformAll(wrapper,true);var row=new JsonObject();row.addProperty("context",control.get("context").getAsString());row.addProperty("role","initial_context");row.addProperty("ordinal",ordinal[0]);row.add("shape",chain(wrapper));wrappers.add(row);}
                ordinal[0]++;
            });
            if(ordinal[0]!=2)throw new IllegalStateException("Original duplicate connector observations changed");
        }
        var output=new JsonObject();output.addProperty("schema","dev.mercurio.first-library-featuring-reference.v1");output.addProperty("qualification_certificate",false);
        output.addProperty("stage","Actual FeatureAdapter.computeFeaturingType + FeatureUtil.insertImplicitTypeFeaturings for every first library Feature in the fixed 44-wrapper bundle, then the retained canonical crossing/initial wrapper producers and explicit wrapper transformAll");
        output.add("features",features);output.add("wrappers",wrappers);Files.writeString(Path.of(args[2]),JSON.toJson(output));
    }
}
