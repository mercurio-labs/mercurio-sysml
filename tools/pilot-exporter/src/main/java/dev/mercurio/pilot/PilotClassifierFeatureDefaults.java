package dev.mercurio.pilot;

import com.google.gson.GsonBuilder;
import java.nio.file.*;
import java.util.*;
import org.eclipse.emf.ecore.EClass;
import org.omg.sysml.lang.sysml.*;
import org.omg.sysml.util.ElementUtil;

/** Independent selector controls for concrete Ecore Classifier owners.
 * Factory observations do not certify parsing, linking or transformation. */
public final class PilotClassifierFeatureDefaults {
    private static java.lang.reflect.Method selector(java.lang.Class<?> type) throws Exception {
        while (type != null) {
            try { var method=type.getDeclaredMethod("getDefaultSupertype"); method.setAccessible(true); return method; }
            catch (NoSuchMethodException ignored) { type=type.getSuperclass(); }
        }
        throw new IllegalStateException("Missing Feature selector");
    }
    public static void main(String[] args) throws Exception {
        org.omg.sysml.logic.SysMLLogicStandaloneSetup.doSetup();
        var factory=SysMLFactory.eINSTANCE;
        var owners=new ArrayList<String>(); var rows=new ArrayList<Object>();
        for (var classifier:SysMLPackage.eINSTANCE.getEClassifiers()) {
            if (!(classifier instanceof EClass ownerClass) || ownerClass.isAbstract() || ownerClass.isInterface()
                || !SysMLPackage.eINSTANCE.getClassifier().isSuperTypeOf(ownerClass)) continue;
            owners.add(ownerClass.getName());
            for (int mask=0;mask<8;mask++) for (boolean composite:List.of(false,true)) for (boolean portion:List.of(false,true)) {
                var owner=(Type)factory.create(ownerClass);
                var feature=factory.createFeature(); feature.setIsImpliedIncluded(true);
                feature.setIsComposite(composite); feature.setIsPortion(portion);
                var membership=factory.createFeatureMembership();membership.getOwnedRelatedElement().add(feature);owner.getOwnedRelationship().add(membership);
                for (int bit=0;bit<3;bit++) if ((mask & (1<<bit))!=0) {
                    Type type=bit==0?factory.createClass():bit==1?factory.createStructure():factory.createDataType();type.setIsImpliedIncluded(true);
                    var typing=factory.createFeatureTyping();typing.setTypedFeature(feature);typing.setType(type);feature.getOwnedRelationship().add(typing);
                }
                var adapter=ElementUtil.getElementAdapter(feature);
                var row=new TreeMap<String,Object>();row.put("owner_kind",ownerClass.getName());row.put("typing_mask",mask);row.put("composite",composite);row.put("portion",portion);
                row.put("owner_ancestry",ownerClass.getEAllSuperTypes().stream().map(EClass::getName).sorted().toList());
                row.put("default_supertype",selector(adapter.getClass()).invoke(adapter));rows.add(row);
            }
        }
        if (rows.size()!=owners.size()*32 || owners.isEmpty()) throw new IllegalStateException("Incomplete Classifier controls");
        var result=new TreeMap<String,Object>();result.put("schema","dev.mercurio.classifier-feature-default-controls.v1");result.put("concrete_classifier_owners",owners);result.put("controls",rows);
        result.put("scope","Feature default selector under every concrete pinned Classifier owner; incomplete owner, supplied complete candidate/typing dependencies. Factory selector observations only; no raw resource or lifecycle qualification.");
        Files.writeString(Path.of(args[0]),new GsonBuilder().setPrettyPrinting().create().toJson(result)+"\n");
    }
}
