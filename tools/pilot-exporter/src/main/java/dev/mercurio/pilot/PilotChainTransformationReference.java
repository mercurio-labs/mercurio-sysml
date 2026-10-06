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

/** Build-time observation of the actual two chain constraint producers.
 * This stage does not invoke full doTransform, validation or publication. */
public final class PilotChainTransformationReference {
    private static final Gson JSON=new GsonBuilder().serializeNulls().disableHtmlEscaping().setPrettyPrinting().create();
    private static JsonObject identity(EObject value) {
        var row=new JsonObject();row.addProperty("kind",value.eClass().getName());
        if(value.eResource()==null)throw new IllegalStateException("Expected contained original endpoint");
        String name=value.eResource().getURI().toFileString().replace((char)92,'/');
        int index=name.indexOf("/sysml.library/");
        row.addProperty("resource",index<0?name:name.substring(index+1));
        row.addProperty("emf_fragment",value.eResource().getURIFragment(value));
        if(value instanceof Element e)row.addProperty("qualified_name",e.getQualifiedName());
        return row;
    }
    private static JsonArray identities(Collection<? extends EObject> values) {
        var rows=new JsonArray();for(var value:values)rows.add(identity(value));return rows;
    }
    private static JsonObject flags(FeatureChainExpression chain,Feature parameter,Feature source,Feature result) {
        var row=new JsonObject();
        row.addProperty("chain",chain.isImpliedIncluded());row.addProperty("parameter",parameter.isImpliedIncluded());
        row.addProperty("source",source!=null&&source.isImpliedIncluded());row.addProperty("result",result!=null&&result.isImpliedIncluded());
        return row;
    }
    private static int count(org.eclipse.emf.ecore.resource.Resource resource) {
        int n=0;var all=resource.getAllContents();while(all.hasNext()){all.next();n++;}return n;
    }
    public static void main(String[] args) throws Exception {
        var spec=JsonParser.parseString(Files.readString(Path.of(args[1]))).getAsJsonObject();
        System.setProperty("org.eclipse.emf.common.util.ReferenceClearingQueue","false");
        var interactive=SysMLInteractive.getInstance();interactive.getLibraryIndexCache().setIndexDisabled(true);
        interactive.setVerbose(false);interactive.loadLibrary(args[0].replace((char)92,'/'));
        var set=interactive.getResourceSet();
        for(var item:spec.getAsJsonArray("source_files")) {
            var path=Path.of(item.getAsString()).toAbsolutePath().normalize();
            set.getResource(URI.createFileURI(path.toString().replace((char)92,'/')),true);
        }
        Method resultProducer=FeatureChainExpressionAdapter.class.getDeclaredMethod("addResultTyping");
        Method targetProducer=FeatureChainExpressionAdapter.class.getDeclaredMethod("addTargetRedefinition");
        resultProducer.setAccessible(true);targetProducer.setAccessible(true);
        var observations=new JsonArray();
        for(var item:spec.getAsJsonArray("chains")) {
            var control=item.getAsJsonObject();var row=new JsonObject();row.add("control",control);
            var path=Path.of(control.get("source_file").getAsString()).toAbsolutePath().normalize();
            var resource=set.getResource(URI.createFileURI(path.toString().replace((char)92,'/')),true);
            var parameter=(Feature)resource.getEObject(control.get("parameter_fragment").getAsString());
            var chain=(FeatureChainExpression)parameter.getOwningType();
            int before=count(resource);row.addProperty("source_nodes_before",before);
            row.addProperty("chain_complete_before",chain.isImpliedIncluded());
            var adapter=(FeatureChainExpressionAdapter)ElementAdapterFactory.getAdapter(chain);
            adapter.addAdditionalMembers();
            var source=chain.sourceTargetFeature();var result=chain.getResult();
            row.add("flags_before_producers",flags(chain,parameter,source,result));
            resultProducer.invoke(adapter);targetProducer.invoke(adapter);
            row.add("chain",identity(chain));row.add("parameter",identity(parameter));
            row.add("source",source==null?JsonNull.INSTANCE:identity(source));
            row.add("result",result==null?JsonNull.INSTANCE:identity(result));
            row.add("target",chain.getTargetFeature()==null?JsonNull.INSTANCE:identity(chain.getTargetFeature()));
            var redefinitions=new JsonArray();
            if(source!=null) {
                var sourceAdapter=(org.omg.sysml.adapter.TypeAdapter)ElementAdapterFactory.getAdapter(source);
                redefinitions=identities(sourceAdapter.getImplicitGeneralTypesOnly(SysMLPackage.Literals.REDEFINITION));
            }
            row.add("cached_source_redefinitions",redefinitions);
            var subsettings=new JsonArray();
            if(result!=null) {
                var resultAdapter=(org.omg.sysml.adapter.TypeAdapter)ElementAdapterFactory.getAdapter(result);
                for(var general:resultAdapter.getImplicitGeneralTypesOnly(SysMLPackage.Literals.SUBSETTING)) {
                    var sub=new JsonObject();sub.addProperty("kind",general.eClass().getName());
                    var links=new JsonArray();
                    if(general instanceof Feature feature) {
                        for(var relation:feature.getOwnedFeatureChaining())links.add(identity(relation.getChainingFeature()));
                    }
                    sub.add("direct_chain",links);subsettings.add(sub);
                }
            }
            row.add("cached_result_subsettings",subsettings);
            row.add("flags_after_producers",flags(chain,parameter,source,result));
            var getters=new JsonArray();
            for(String field:List.of("inheritedMembership","type")) {
                var read=new JsonObject();read.addProperty("field",field);
                try {
                    var feature=parameter.eClass().getEStructuralFeature(field);
                    Object value=parameter.eGet(feature);
                    var values=value instanceof List<?> list?list:value==null?List.of():List.of(value);
                    var endpoints=new JsonArray();
                    for(var endpoint:values)endpoints.add(identity((EObject)endpoint));
                    read.addProperty("status","reference_observed");read.add("endpoints",endpoints);
                } catch(Exception failure) {
                    read.addProperty("status","reference_getter_failed");read.addProperty("error",failure.toString());
                }
                getters.add(read);
            }
            row.add("parameter_getters",getters);row.add("flags_after_getters",flags(chain,parameter,source,result));
            row.addProperty("source_nodes_after",count(resource));observations.add(row);
            System.err.println(control.get("context").getAsString()+" redefinitions="+redefinitions.size()+" result plans="+subsettings.size());
        }
        var result=new JsonObject();result.addProperty("schema","dev.mercurio.explicit-chain-producer-reference.v1");
        result.addProperty("qualification_certificate",false);result.addProperty("full_transformation","not_assessed");
        result.addProperty("semantic_validation","not_assessed");result.add("observations",observations);
        Files.writeString(Path.of(args[2]),JSON.toJson(result));
    }
}
