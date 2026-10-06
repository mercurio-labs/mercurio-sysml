package dev.mercurio.pilot;

import java.nio.file.*;
import java.util.*;
import com.google.gson.*;
import org.omg.sysml.lang.sysml.*;

/** Independent build-time operation controls on unprepared Ecore prototypes.
 * These controls do not qualify complete, validated expression models. */
public final class PilotChainSelectionRegisteredControls {
    private static final SysMLFactory F = SysMLFactory.eINSTANCE;
    private static final Gson JSON = new GsonBuilder().serializeNulls().setPrettyPrinting().create();
    private static Feature input(FeatureChainExpression chain, String name, FeatureDirectionKind direction, boolean child) {
        ParameterMembership membership = F.createParameterMembership();
        Feature feature = F.createFeature();
        feature.setDeclaredName(name);
        feature.setDirection(direction);
        chain.getOwnedRelationship().add(membership);
        membership.getOwnedRelatedElement().add(feature);
        if (child) {
            FeatureMembership childMembership = F.createFeatureMembership();
            Feature target = F.createFeature();
            target.setDeclaredName(name + ".target");
            feature.getOwnedRelationship().add(childMembership);
            childMembership.getOwnedRelatedElement().add(target);
        }
        return feature;
    }
    private static void member(FeatureChainExpression chain, boolean returned, String name, boolean dataType) {
        OwningMembership membership = returned ? F.createReturnParameterMembership() : F.createOwningMembership();
        Element target = dataType ? F.createDataType() : F.createFeature();
        target.setDeclaredName(name);
        if (returned) ((Feature)target).setDirection(FeatureDirectionKind.OUT);
        chain.getOwnedRelationship().add(membership);
        membership.getOwnedRelatedElement().add(target);
    }
    private static JsonObject source(String name, FeatureDirectionKind direction, boolean first, boolean second, boolean bothOut) {
        var chain = F.createFeatureChainExpression();
        input(chain,"first",direction,first);
        input(chain,"second",bothOut ? FeatureDirectionKind.OUT : FeatureDirectionKind.IN,second);
        var row = new JsonObject();
        row.addProperty("id",name);
        row.addProperty("operation","sourceTargetFeature");
        row.addProperty("first_direction",direction.getLiteral());
        row.addProperty("first_owned_feature",first);
        row.addProperty("second_owned_feature",second);
        row.addProperty("second_direction",bothOut ? "out" : "in");
        Feature endpoint = chain.sourceTargetFeature();
        row.addProperty("pilot_endpoint",endpoint == null ? null : endpoint.getDeclaredName());
        row.addProperty("completion_flag",chain.isImpliedIncluded());
        return row;
    }
    private static JsonObject target(String name, boolean returnedFirst, boolean dataType) {
        var chain = F.createFeatureChainExpression();
        input(chain,"first",FeatureDirectionKind.IN,false);
        if (returnedFirst) member(chain,true,"result",false);
        member(chain,false,"target",dataType);
        if (!returnedFirst) member(chain,true,"result",false);
        var row = new JsonObject();
        row.addProperty("id",name);
        row.addProperty("operation","targetFeature");
        row.addProperty("return_before_target",returnedFirst);
        row.addProperty("target_is_data_type",dataType);
        Feature endpoint = chain.getTargetFeature();
        row.addProperty("pilot_endpoint",endpoint == null ? null : endpoint.getDeclaredName());
        row.addProperty("completion_flag",chain.isImpliedIncluded());
        return row;
    }
    public static void main(String[] args) throws Exception {
        System.setProperty("org.eclipse.emf.common.util.ReferenceClearingQueue","false");
        org.omg.sysml.interactive.SysMLInteractive.getInstance();
        var rows = new JsonArray();
        rows.add(source("first_in",FeatureDirectionKind.IN,true,true,false));
        rows.add(source("empty_first_in",FeatureDirectionKind.IN,false,true,false));
        rows.add(source("out_then_in",FeatureDirectionKind.OUT,true,true,false));
        rows.add(source("inout_then_in",FeatureDirectionKind.INOUT,true,true,false));
        rows.add(source("empty_inputs",FeatureDirectionKind.IN,false,false,false));
        rows.add(source("no_input",FeatureDirectionKind.OUT,true,true,true));
        rows.add(target("target_then_return",false,false));
        rows.add(target("return_then_target",true,false));
        rows.add(target("non_feature_target",false,true));
        var result = new JsonObject();
        result.addProperty("schema","dev.mercurio.chain-selection-reference-controls.v1");
        result.addProperty("qualification_certificate",false);
        result.addProperty("native_qualification","not_assessed");
        result.addProperty("scope","Unprepared Ecore prototypes; complete validation and expression transformation are not assessed.");
        result.add("controls",rows);
        Files.writeString(Path.of(args[0]),JSON.toJson(result));
    }
}
