package dev.mercurio.pilot;

import java.nio.charset.StandardCharsets;
import java.nio.file.*;
import java.util.*;
import com.google.gson.*;
import org.eclipse.emf.common.util.*;
import org.eclipse.emf.ecore.*;
import org.omg.sysml.interactive.SysMLInteractive;
import org.omg.sysml.lang.sysml.*;

/** Independent build-time direction operation controls. Actual library providers
 * are never prepared. Cycle cases explicitly supply materialized fixture Types;
 * this is operation evidence, not source/lifecycle qualification. */
public final class PilotDirectionOperationControls {
    public static void main(String[] args) throws Exception {
        System.setProperty("org.eclipse.emf.common.util.ReferenceClearingQueue","false");
        var interactive=SysMLInteractive.getInstance();
        interactive.getLibraryIndexCache().setIndexDisabled(true);
        interactive.setVerbose(false);
        interactive.loadLibrary(Path.of(args[0]).toAbsolutePath().toString().replace((char)92,'/'));
        var factory=SysMLFactory.eINSTANCE;
        var root=factory.createPackage();
        root.setDeclaredName("P");
        var resource=interactive.getResourceSet().createResource(URI.createURI("memory:/direction-operation-controls.kerml"));
        resource.getContents().add(root);
        var types=new LinkedHashMap<String,Classifier>();
        for(String name:List.of("G","H","C","D","A","B")) {
            var type=factory.createClassifier();type.setDeclaredName(name);
            var member=factory.createOwningMembership();
            root.getOwnedRelationship().add(member);member.getOwnedRelatedElement().add(type);
            types.put(name,type);
        }
        var features=new LinkedHashMap<String,Feature>();
        for(String name:List.of("i","o","b","u")) {
            var feature=factory.createFeature();feature.setDeclaredName(name);
            feature.setDirection(switch(name) {case "i" -> FeatureDirectionKind.IN;case "o" -> FeatureDirectionKind.OUT;
                case "b" -> FeatureDirectionKind.INOUT;default -> null;});
            var membership=factory.createFeatureMembership();
            types.get("G").getOwnedRelationship().add(membership);membership.getOwnedRelatedElement().add(feature);
            features.put(name,feature);
        }
        for(String[] pair:new String[][]{{"H","G"},{"A","B"},{"B","A"}}) {
            var relationship=factory.createSubclassification();
            types.get(pair[0]).getOwnedRelationship().add(relationship);
            relationship.setSubclassifier(types.get(pair[0]));relationship.setSuperclassifier(types.get(pair[1]));
        }
        for(String[] pair:new String[][]{{"C","G"},{"D","C"}}) {
            var relationship=factory.createConjugation();
            types.get(pair[0]).getOwnedRelationship().add(relationship);
            relationship.setConjugatedType(types.get(pair[0]));relationship.setOriginalType(types.get(pair[1]));
        }
        // Supplied operation inputs only; never inferred source/provider completion.
        types.get("A").setIsImpliedIncluded(true);types.get("B").setIsImpliedIncluded(true);
        var operation=SysMLPackage.eINSTANCE.getType().getEOperations().stream()
            .filter(o->o.getName().equals("directionOf")).findFirst().orElseThrow();
        if(operation.getLowerBound()!=0||operation.getUpperBound()!=1
            ||!operation.getEType().getName().equals("FeatureDirectionKind")
            ||operation.getEParameters().size()!=1||!operation.getEParameters().get(0).getEType().getName().equals("Feature"))
            throw new IllegalStateException("Resolved operation signature changed");
        var rows=new JsonArray();
        for(String owner:List.of("G","H","C","D","A","B")) {
            for(String name:owner.equals("A")||owner.equals("B")?List.of("i"):List.of("i","o","b","u")) {
                var type=types.get(owner);boolean before=type.isImpliedIncluded();
                Object value=type.eInvoke(operation,new BasicEList<>(List.of(features.get(name))));
                var row=new JsonObject();row.addProperty("owner",owner);row.addProperty("feature",name);
                row.addProperty("value",value==null?null:((FeatureDirectionKind)value).getLiteral());
                row.addProperty("supplied_materialized_operation_input",owner.equals("A")||owner.equals("B"));
                row.addProperty("completion_before",before);row.addProperty("completion_after",type.isImpliedIncluded());
                rows.add(row);
            }
        }
        var output=new JsonObject();output.addProperty("schema","dev.mercurio.direction-operation-reference.v1");
        output.addProperty("qualification_certificate",false);output.addProperty("native_context_qualification","not_assessed");
        output.addProperty("operation","Type.directionOf(Feature)");
        output.addProperty("scope","Factory operation inputs, including supplied materialized cycles. Actual providers and source transformations are not qualified.");
        output.add("observations",rows);
        var json=new GsonBuilder().serializeNulls().setPrettyPrinting().create();
        Files.writeString(Path.of(args[1]),json.toJson(output)+"\n",StandardCharsets.UTF_8);
    }
}
