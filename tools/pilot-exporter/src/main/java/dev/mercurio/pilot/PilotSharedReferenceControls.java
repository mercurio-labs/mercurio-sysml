package dev.mercurio.pilot;

import java.nio.charset.StandardCharsets;
import java.nio.file.*;
import java.util.*;
import com.google.gson.*;
import org.eclipse.emf.common.util.URI;
import org.eclipse.emf.ecore.*;
import org.omg.sysml.interactive.SysMLInteractive;
import org.omg.sysml.lang.sysml.*;

/** Independent build-time EMF controls for six reviewed structural delegates.
 * Factory fixtures do not establish actual source/library lifecycle support. */
public final class PilotSharedReferenceControls {
    private static final SysMLFactory F=SysMLFactory.eINSTANCE;
    private static OwningMembership own(Namespace parent, Element child) {
        var membership=F.createOwningMembership();
        parent.getOwnedRelationship().add(membership);
        membership.getOwnedRelatedElement().add(child);return membership;
    }
    public static void main(String[] args) throws Exception {
        System.setProperty("org.eclipse.emf.common.util.ReferenceClearingQueue","false");
        var interactive=SysMLInteractive.getInstance();
        interactive.getLibraryIndexCache().setIndexDisabled(true);interactive.setVerbose(false);
        interactive.loadLibrary(Path.of(args[0]).toAbsolutePath().toString().replace((char)92,'/'));
        var p=F.createPackage();
        var resource=interactive.getResourceSet().createResource(URI.createURI("memory:/shared-reference-controls.kerml"));
        resource.getContents().add(p);
        var G=F.createClassifier();var H=F.createClassifier();var A=F.createClassifier();
        own(p,G);own(p,H);own(p,A);
        var f=F.createFeature();var g=F.createFeature();var z=F.createFeature();
        for(var feature:List.of(f,g)) {
            var member=F.createFeatureMembership();G.getOwnedRelationship().add(member);
            member.getOwnedRelatedElement().add(feature);
        }
        var m1=F.createMultiplicityRange();var m2=F.createMultiplicityRange();
        own(G,m1);own(G,m2);
        var hg=F.createSubclassification();H.getOwnedRelationship().add(hg);
        hg.setSubclassifier(H);hg.setSuperclassifier(G);
        var alias=F.createMembership();A.getOwnedRelationship().add(alias);alias.setMemberElement(m1);
        var c1=F.createCrossSubsetting();var c2=F.createCrossSubsetting();
        for(var cross:List.of(c1,c2)) {f.getOwnedRelationship().add(cross);cross.setSubsettingFeature(f);}
        c1.setSubsettedFeature(g);c2.setSubsettedFeature(f);
        var v=F.createFeatureValue();f.getOwnedRelationship().add(v);
        var e=F.createLiteralInteger();e.setValue(1);v.getOwnedRelatedElement().add(e);
        var w=F.createFeatureValue();p.getOwnedRelationship().add(w);
        var we=F.createLiteralInteger();we.setValue(2);w.getOwnedRelatedElement().add(we);
        var D=F.createDependency();D.getClient().addAll(List.of(f,g));D.getSupplier().addAll(List.of(g,f));
        var objects=new LinkedHashMap<String,Element>();
        objects.put("P",p);objects.put("G",G);objects.put("H",H);objects.put("A",A);
        objects.put("f",f);objects.put("g",g);objects.put("z",z);
        objects.put("m1",m1);objects.put("m2",m2);objects.put("c1",c1);objects.put("c2",c2);
        objects.put("v",v);objects.put("e",e);objects.put("w",w);objects.put("we",we);objects.put("D",D);
        var inverse=new IdentityHashMap<EObject,String>();objects.forEach((name,obj)->inverse.put(obj,name));
        String[][] controls={
            {"P","ownedMember","owned_member"},{"G","ownedMember","owned_member"},{"A","ownedMember","owned_member"},
            {"G","multiplicity","multiplicity"},{"H","multiplicity","multiplicity"},{"A","multiplicity","multiplicity"},
            {"f","ownedCrossSubsetting","owned_cross_subsetting"},{"g","ownedCrossSubsetting","owned_cross_subsetting"},
            {"v","featureWithValue","feature_with_value"},{"w","featureWithValue","feature_with_value"},
            {"P","owningNamespace","owning_namespace"},{"G","owningNamespace","owning_namespace"},
            {"f","owningNamespace","owning_namespace"},{"m1","owningNamespace","owning_namespace"},
            {"v","owningNamespace","owning_namespace"},{"z","owningNamespace","owning_namespace"},
            {"D","relatedElement","related_element"},{"c1","relatedElement","related_element"},
            {"v","relatedElement","related_element"}};
        var rows=new JsonArray();
        for(var request:controls) {
            var owner=objects.get(request[0]);boolean before=owner.isImpliedIncluded();
            var feature=owner.eClass().getEStructuralFeature(request[1]);
            if(!(feature instanceof EReference))throw new IllegalStateException("Expected imported EReference");
            var targets=new JsonArray();Object value=owner.eGet(feature);
            Collection<?> values;
            if(value instanceof Collection<?>) values=(Collection<?>)value;
            else if(value==null) values=List.of();
            else values=List.of(value);
            for(Object target:values) {
                String name=inverse.get((EObject)target);if(name==null)throw new IllegalStateException("Unexpected fixture target for "+request[0]+"."+request[1]+": "+((EObject)target).eClass().getName());
                targets.add(name);
            }
            var row=new JsonObject();row.addProperty("owner",request[0]);row.addProperty("feature",request[1]);
            row.addProperty("native_field",request[2]);row.add("targets",targets);
            row.addProperty("completion_before",before);row.addProperty("completion_after",owner.isImpliedIncluded());
            rows.add(row);
        }
        var result=new JsonObject();result.addProperty("schema","dev.mercurio.shared-reference-controls.v1");
        result.addProperty("qualification_certificate",false);result.addProperty("native_context_qualification","not_assessed");
        result.addProperty("fixture_preparation","Factory containment inputs; no completion flags set; actual libraries unprepared.");
        result.add("observations",rows);
        Files.writeString(Path.of(args[1]),new GsonBuilder().serializeNulls().setPrettyPrinting().create().toJson(result)+"\n",StandardCharsets.UTF_8);
    }
}
