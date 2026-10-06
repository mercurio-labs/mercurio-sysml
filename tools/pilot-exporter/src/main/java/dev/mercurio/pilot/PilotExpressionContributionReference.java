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

/** Build-time observation of the shared default/reduction stage.
 * Full connectors, validation, publication and lifecycle completion are separate. */
public final class PilotExpressionContributionReference {
    private static final Gson JSON=new GsonBuilder().serializeNulls().disableHtmlEscaping().setPrettyPrinting().create();
    private static JsonObject identity(EObject value) {
        if(value.eIsProxy()||value.eResource()==null)throw new IllegalStateException("Unresolved or detached default endpoint");
        var result=new JsonObject();result.addProperty("kind",value.eClass().getName());
        String path=value.eResource().getURI().toFileString().replace((char)92,'/');
        int index=path.indexOf("/sysml.library/");result.addProperty("resource",index<0?path:path.substring(index+1));
        result.addProperty("emf_fragment",value.eResource().getURIFragment(value));
        return result;
    }
    private static void invoke(java.lang.Class<?> type,String name,Object adapter) throws Exception {
        Method method=type.getDeclaredMethod(name);method.setAccessible(true);method.invoke(adapter);
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
            var source=Path.of(control.get("source_file").getAsString());
            var resource=set.getResource(URI.createFileURI(source.toString().replace((char)92,'/')),true);
            var owner=(Expression)resource.getEObject(control.get("emf_fragment").getAsString());
            row.addProperty("completion_before",owner.isImpliedIncluded());
            if(control.get("link_negative").getAsBoolean()) {
                row.addProperty("status","link_negative_not_qualified");row.addProperty("completion_after",owner.isImpliedIncluded());
                rows.add(row);continue;
            }
            try {
                var adapter=(org.omg.sysml.adapter.TypeAdapter)ElementAdapterFactory.getAdapter(owner);
                adapter.addAdditionalMembers();
                if(owner instanceof FeatureChainExpression) {
                    invoke(FeatureChainExpressionAdapter.class,"addResultTyping",adapter);
                    invoke(FeatureChainExpressionAdapter.class,"addTargetRedefinition",adapter);
                } else if(owner instanceof FeatureReferenceExpression) {
                    invoke(FeatureReferenceExpressionAdapter.class,"addResultSubsetting",adapter);
                }
                adapter.computeImplicitGeneralTypes();adapter.removeUnnecessaryImplicitGeneralTypes();
                var effects=new JsonArray();
                for(var kind:List.of(SysMLPackage.Literals.FEATURE_TYPING,SysMLPackage.Literals.SUBSETTING)) {
                    for(var target:adapter.getImplicitGeneralTypesOnly(kind)) {
                        var effect=new JsonObject();effect.addProperty("kind",kind.getName());effect.add("target",identity(target));effects.add(effect);
                    }
                }
                row.add("effects",effects);row.addProperty("status","reference_observed");
            }catch(Exception failure) {
                row.addProperty("status","reference_stage_failed");row.addProperty("error",failure.toString());
            }
            row.addProperty("completion_after",owner.isImpliedIncluded());rows.add(row);
            System.err.println(control.get("context").getAsString()+" "+owner.eClass().getName()+" "+row.get("status").getAsString());
        }
        var result=new JsonObject();result.addProperty("schema","dev.mercurio.expression-contribution-reference.v1");
        result.addProperty("qualification_certificate",false);result.addProperty("full_transform_validation","not_assessed");
        result.add("observations",rows);Files.writeString(Path.of(args[2]),JSON.toJson(result));
    }
}
