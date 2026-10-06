package dev.mercurio.pilot;
import com.google.gson.GsonBuilder;
import java.nio.file.*;
import java.util.*;
import org.eclipse.emf.ecore.util.BasicInternalEList;
import org.eclipse.emf.common.util.EList;
import org.omg.sysml.lang.sysml.*;
import org.omg.sysml.lang.sysml.impl.*;
import org.eclipse.emf.ecore.*;

/** Keep controlled composition observations separate from real canonical package derivation. */
public final class PilotUnionProbe {
    static final Map<String,Membership> members = new LinkedHashMap<>();
    static final Map<Membership,String> ids = new IdentityHashMap<>();
    static EList<Membership> values(String input) {
        EList<Membership> result = new BasicInternalEList<>(Membership.class);
        for (String id : input.split(" ")) if (!id.isEmpty()) result.add(members.get(id));
        return result;
    }
    static class NamespaceInput extends NamespaceImpl {
        EList<Membership> imported, owned;
        NamespaceInput(String a, String b) { imported=values(a); owned=values(b); }
        @Override public EList<Membership> getImportedMembership() { return imported; }
        @Override public EList<Membership> getOwnedMembership() { return owned; }
        @Override public boolean eIsSet(int feature) {
            if (feature==SysMLPackage.NAMESPACE__IMPORTED_MEMBERSHIP) return !imported.isEmpty();
            if (feature==SysMLPackage.NAMESPACE__OWNED_MEMBERSHIP) return !owned.isEmpty();
            return super.eIsSet(feature);
        }
    }
    static class TypeInput extends TypeImpl {
        EList<Membership> imported, owned, inherited;
        TypeInput(String a, String b, String c) { imported=values(a); owned=values(b); inherited=values(c); }
        @Override public EList<Membership> getImportedMembership() { return imported; }
        @Override public EList<Membership> getOwnedMembership() { return owned; }
        @Override public EList<Membership> getInheritedMembership() { return inherited; }
        @Override public boolean eIsSet(int feature) {
            if (feature==SysMLPackage.TYPE__IMPORTED_MEMBERSHIP) return !imported.isEmpty();
            if (feature==SysMLPackage.TYPE__OWNED_MEMBERSHIP) return !owned.isEmpty();
            if (feature==SysMLPackage.TYPE__INHERITED_MEMBERSHIP) return !inherited.isEmpty();
            return super.eIsSet(feature);
        }
    }
    static List<String> names(List<Membership> values) { return values.stream().map(ids::get).toList(); }
    static Map<String,Object> canonicalControl(int scenario) {
        var factory=SysMLFactory.eINSTANCE;
        boolean typed=(scenario>=4 && scenario<8) || scenario>=11;
        Namespace source=typed ? factory.createClass() : factory.createPackage();
        var destination=factory.createPackage();
        var base=factory.createClass(); var external=factory.createPackage();
        var target=factory.createClass();
        var first=factory.createMembership(); first.setMemberName("Z"); first.setMemberElement(target);
        var second=factory.createMembership(); second.setMemberName("A"); second.setMemberElement(target); second.setVisibility((scenario>=4 && scenario<6) || scenario>=11 ? VisibilityKind.PROTECTED : VisibilityKind.PRIVATE);
        Namespace owner=typed && scenario>=6 && scenario<8 ? external : typed ? base : source;
        owner.getOwnedRelationship().add(first); owner.getOwnedRelationship().add(second);
        var local=factory.createMembership(); local.setMemberName("Local"); local.setMemberElement(target);
        destination.getOwnedRelationship().add(local);
        var imported=factory.createNamespaceImport(); imported.setImportedNamespace(source);
        imported.setVisibility(VisibilityKind.PRIVATE); imported.setIsImportAll(scenario > 0 && scenario != 4 && scenario != 6);
        destination.getOwnedRelationship().add(imported);
        Map<String,EObject> objects=new LinkedHashMap<>();
        objects.put("source",source); objects.put("destination",destination); objects.put("target",target);
        objects.put("z",first); objects.put("a",second); objects.put("local",local); objects.put("import",imported);
        if (typed) {
            var specialization=factory.createSubclassification();
            specialization.setSubclassifier((Classifier)source); specialization.setSuperclassifier(base);
            source.getOwnedRelationship().add(specialization);
            objects.put("base",base); objects.put("specialization",specialization);
        }
        if (typed && scenario >= 6 && scenario < 8) {
            var inheritedImport=factory.createNamespaceImport(); inheritedImport.setImportedNamespace(external);
            inheritedImport.setIsImportAll(true); inheritedImport.setVisibility(VisibilityKind.PUBLIC);
            base.getOwnedRelationship().add(inheritedImport);
            objects.put("external",external); objects.put("inheritedImport",inheritedImport);
        }
        if (scenario == 2) {
            var duplicate=factory.createMembershipImport(); duplicate.setImportedMembership(first);
            destination.getOwnedRelationship().add(duplicate); objects.put("duplicate",duplicate);
        }
        if (scenario == 3) {
            var relay=factory.createPackage(); var reexport=factory.createNamespaceImport();
            reexport.setImportedNamespace(source); reexport.setIsImportAll(true); reexport.setVisibility(VisibilityKind.PUBLIC);
            relay.getOwnedRelationship().add(reexport); imported.setImportedNamespace(relay);
            objects.put("relay",relay); objects.put("reexport",reexport);
        }
        if (scenario == 8) {
            for (int i=0;i<2;i++) {
                var relay=factory.createPackage(); var reexport=factory.createNamespaceImport();
                reexport.setImportedNamespace(source); reexport.setVisibility(VisibilityKind.PUBLIC); reexport.setIsImportAll(true);
                relay.getOwnedRelationship().add(reexport);
                if (i==0) imported.setImportedNamespace(relay);
                else {
                    var secondImport=factory.createNamespaceImport(); secondImport.setImportedNamespace(relay);
                    destination.getOwnedRelationship().add(secondImport); objects.put("secondImport",secondImport);
                }
                objects.put("relay"+i,relay); objects.put("reexport"+i,reexport);
            }
        }
        if (scenario == 9) {
            var back=factory.createNamespaceImport(); back.setImportedNamespace(destination);
            back.setVisibility(VisibilityKind.PUBLIC); back.setIsImportAll(true);
            source.getOwnedRelationship().add(back); objects.put("back",back);
        }
        if (scenario == 10 || scenario == 18) {
            var nested=factory.createPackage(); nested.setDeclaredName("Nested");
            var nesting=factory.createOwningMembership(); nesting.setVisibility(VisibilityKind.PRIVATE);
            nesting.getOwnedRelatedElement().add(nested); source.getOwnedRelationship().add(nesting);
            var leaf=factory.createMembership(); leaf.setMemberName("Leaf"); leaf.setMemberElement(target);
            nested.getOwnedRelationship().add(leaf); imported.setIsRecursive(true);
            objects.put("nested",nested); objects.put("nesting",nesting); objects.put("leaf",leaf);
        }
        if (scenario>=11) {
            imported.setIsRecursive(true); imported.setIsImportAll(scenario!=11);
            if (scenario==13) {
                for (int i=0;i<2;i++) {
                    var branch=factory.createClass(); var up=factory.createSubclassification();
                    up.setSubclassifier(branch); up.setSuperclassifier(base); branch.getOwnedRelationship().add(up);
                    var down=factory.createSubclassification(); down.setSubclassifier((Classifier)source); down.setSuperclassifier(branch);
                    source.getOwnedRelationship().add(down);
                    objects.put("branch"+i,branch); objects.put("up"+i,up); objects.put("down"+i,down);
                }
            }
            if (scenario==14) {
                var back=factory.createSubclassification(); back.setSubclassifier(base); back.setSuperclassifier((Classifier)source);
                base.getOwnedRelationship().add(back); objects.put("back",back);
            }
            if (scenario==15) {
                source.getOwnedRelationship().clear(); objects.remove("specialization");
                var conjugation=factory.createConjugation(); conjugation.setConjugatedType((Type)source); conjugation.setOriginalType(base);
                source.getOwnedRelationship().add(conjugation); objects.put("conjugation",conjugation);
            }
            if (scenario==16) {
                var inheritedImport=factory.createNamespaceImport(); inheritedImport.setImportedNamespace(external);
                inheritedImport.setIsImportAll(true); inheritedImport.setVisibility(VisibilityKind.PROTECTED);
                external.getOwnedRelationship().add(second); second.setVisibility(VisibilityKind.PRIVATE);
                base.getOwnedRelationship().add(inheritedImport);
                objects.put("external",external); objects.put("inheritedImport",inheritedImport);
            }
            if (scenario==17) ((Specialization)objects.get("specialization")).setIsImplied(true);
        }
        if (scenario>=19) {
            imported.setIsRecursive(false);
            // Complete materialized inputs, not precomputed inherited results.
            if (scenario>=20 && scenario!=24) {
                var fz=factory.createFeature(); fz.setDeclaredName("Z");
                var fa=factory.createFeature(); fa.setDeclaredName("A");
                first.setMemberElement(fz); second.setMemberElement(fa);
                objects.put("fz",fz); objects.put("fa",fa);
                if (scenario==26) second.setMemberElement(fz);
                if (scenario==21 || scenario==23 || scenario==25) {
                    var redefine=factory.createRedefinition(); redefine.setRedefiningFeature(fz); redefine.setRedefinedFeature(fa);
                    fz.getOwnedRelationship().add(redefine); objects.put("redefine",redefine);
                }
                if (scenario==25) {
                    var reverse=factory.createRedefinition(); reverse.setRedefiningFeature(fa); reverse.setRedefinedFeature(fz);
                    fa.getOwnedRelationship().add(reverse); objects.put("reverseRedefinition",reverse);
                }
                if (scenario==22 || scenario==23 || scenario==27 || scenario==28) {
                    var own=factory.createFeature(); if (scenario!=27 && scenario!=28) own.setDeclaredName("Own");
                    var ownMembership=factory.createFeatureMembership(); ownMembership.getOwnedRelatedElement().add(own);
                    source.getOwnedRelationship().add(ownMembership);
                    var redefine=factory.createRedefinition(); redefine.setRedefiningFeature(own); redefine.setRedefinedFeature(scenario==23 || scenario==28 ? fa : fz);
                    if (scenario!=27) own.getOwnedRelationship().add(redefine);
                    objects.put("own",own); objects.put("ownMembership",ownMembership); if (scenario!=27) objects.put("ownRedefinition",redefine);
                }
            }
            if (scenario==24) ((Specialization)objects.get("specialization")).setIsImplied(true);
            for (EObject object: objects.values()) if (object instanceof Element element) element.setIsImpliedIncluded(true);
            for (EObject object: objects.values()) if (object instanceof Type type) {
                if (!org.omg.sysml.util.TypeUtil.getImplicitGeneralTypesFor(type).isEmpty()) throw new IllegalStateException("Unmaterialized implicit generals");
            }
            // Verify the fixture's completeness assertion against the actual
            // upstream semantic service; do not disable or replace that service.
            for (EObject object: objects.values()) if (object instanceof Feature feature) {
                var stored=feature.getOwnedRedefinition().stream().map(Redefinition::getRedefinedFeature).toList();
                var computed=org.omg.sysml.util.FeatureUtil.getRedefinedFeaturesWithComputedOf(feature);
                if (!new HashSet<>(stored).equals(new HashSet<>(computed))) throw new IllegalStateException("Unmaterialized redefinitions");
            }
        }
        Map<EObject,String> objectIds=new IdentityHashMap<>();
        objects.forEach((id,object)->objectIds.put(object,id));
        List<Object> graph=new ArrayList<>();
        for (var entry:objects.entrySet()) {
            EObject object=entry.getValue(); Map<String,Object> row=new LinkedHashMap<>();
            row.put("@id",entry.getKey()); row.put("@type",object.eClass().getName());
            for (String field:List.of("ownedRelationship","owningRelatedElement","memberElement","memberName","memberShortName","visibility","importedNamespace","importedMembership","isImportAll","isRecursive","ownedRelatedElement","owningRelationship","declaredName","subclassifier","superclassifier","conjugatedType","originalType","isImplied","isImpliedIncluded","redefiningFeature","redefinedFeature")) {
                var feature=object.eClass().getEStructuralFeature(field);
                if (feature==null || feature.isDerived() || (!object.eIsSet(feature) && !field.equals("ownedRelationship"))) continue;
                Object value=object.eGet(feature);
                if (feature instanceof EReference) {
                    if (feature.isMany()) row.put(field,((List<?>)value).stream().map(v->Map.of("@id",Objects.requireNonNull(objectIds.get(v)))).toList());
                    else if (value!=null) row.put(field,Map.of("@id",Objects.requireNonNull(objectIds.get(value))));
                } else row.put(field,value instanceof org.eclipse.emf.common.util.Enumerator ? value.toString() : value);
            }
            // Supply actual resolved inheritance as an explicit semantic dependency.
            // The native comparison does not claim to derive it from specializations.
            if (typed && scenario<11 && object instanceof Type type) {
                row.put("inheritedMembership",type.getInheritedMembership().stream()
                    .map(v->Map.of("@id",Objects.requireNonNull(objectIds.get(v)))).toList());
            }
            graph.add(row);
        }
        return Map.of("case",scenario,"graph",graph,"root","destination",
            "owned_membership",destination.getOwnedMembership().stream().map(objectIds::get).toList(),
            "imported_membership",destination.getImportedMembership().stream().map(objectIds::get).toList(),
            "membership",destination.getMembership().stream().map(objectIds::get).toList(),
            "source_membership",source.getMembership().stream().map(objectIds::get).toList());
    }
    static Map<String,Object> chainingControl(int scenario) {
        var f=SysMLFactory.eINSTANCE;
        Feature source=scenario==7 ? f.createPartUsage() : f.createFeature();
        var first=f.createFeature(); var last=f.createFeature(); var explicit=f.createFeature();
        var target=f.createClass(); var destination=f.createPackage();
        Map<String,EObject> objects=new LinkedHashMap<>();
        objects.put("source",source); objects.put("first",first); objects.put("last",last);
        objects.put("explicit",explicit); objects.put("target",target); objects.put("destination",destination);
        for (var entry:List.of(Map.entry("First",first),Map.entry("Last",last),Map.entry("Explicit",explicit))) {
            var member=f.createMembership(); member.setMemberName(entry.getKey()); member.setMemberElement(target);
            entry.getValue().getOwnedRelationship().add(member); objects.put(entry.getKey(),member);
        }
        for (int i=0;i<2;i++) {
            var chain=f.createFeatureChaining(); chain.setFeatureChained(source); chain.setChainingFeature(i==0?first:last);
            source.getOwnedRelationship().add(chain); objects.put("chain"+i,chain);
        }
        if (scenario==1 || scenario==4) {
            var subset=f.createSubsetting(); subset.setSubsettingFeature(source); subset.setSubsettedFeature(scenario==4?last:explicit);
            source.getOwnedRelationship().add(0,subset); objects.put("subset",subset);
        }
        if (scenario==3) {
            for (int i=0;i<2;i++) {
                var back=f.createFeatureChaining(); back.setFeatureChained(last); back.setChainingFeature(i==0?first:source);
                last.getOwnedRelationship().add(back); objects.put("back"+i,back);
            }
        }
        if (scenario==8) {
            var conjugation=f.createConjugation(); conjugation.setConjugatedType(source); conjugation.setOriginalType(explicit);
            source.getOwnedRelationship().add(conjugation); objects.put("conjugation",conjugation);
        }
        if (scenario==5) ((Membership)objects.get("Last")).setVisibility(VisibilityKind.PROTECTED);
        if (scenario==6) ((Membership)objects.get("Last")).setVisibility(VisibilityKind.PRIVATE);
        var imported=f.createNamespaceImport(); imported.setImportedNamespace(source); imported.setIsRecursive(scenario==2);
        imported.setIsImportAll(scenario==5); destination.getOwnedRelationship().add(imported); objects.put("import",imported);
        for (EObject object:objects.values()) if (object instanceof Element element) element.setIsImpliedIncluded(true);
        for (EObject object:objects.values()) if (object instanceof Type type) {
            if (!org.omg.sysml.util.TypeUtil.getImplicitGeneralTypesFor(type).isEmpty()) throw new IllegalStateException("Unmaterialized chain generals");
        }
        for (EObject object:objects.values()) if (object instanceof Feature feature) {
            if (!org.omg.sysml.util.FeatureUtil.getRedefinedFeaturesWithComputedOf(feature).isEmpty()) throw new IllegalStateException("Unmaterialized chain redefinitions");
        }
        Map<EObject,String> identities=new IdentityHashMap<>(); objects.forEach((id,object)->identities.put(object,id));
        List<Object> graph=new ArrayList<>();
        for (var entry:objects.entrySet()) {
            EObject object=entry.getValue(); Map<String,Object> row=new LinkedHashMap<>();
            row.put("@id",entry.getKey()); row.put("@type",object.eClass().getName());
            for (String field:List.of("ownedRelationship","owningRelatedElement","memberElement","memberName","visibility","importedNamespace","isImportAll","isRecursive","isImpliedIncluded","featureChained","chainingFeature","subsettingFeature","subsettedFeature","conjugatedType","originalType")) {
                var feature=object.eClass().getEStructuralFeature(field);
                if (feature==null || feature.isDerived() || (!object.eIsSet(feature) && !field.equals("ownedRelationship"))) continue;
                Object value=object.eGet(feature);
                if (feature instanceof EReference) {
                    if (feature.isMany()) row.put(field,((List<?>)value).stream().map(v->Map.of("@id",Objects.requireNonNull(identities.get(v)))).toList());
                    else if (value!=null) row.put(field,Map.of("@id",Objects.requireNonNull(identities.get(value))));
                } else row.put(field,value instanceof org.eclipse.emf.common.util.Enumerator ? value.toString() : value);
            }
            graph.add(row);
        }
        return Map.of("case",scenario,"graph",graph,"root","destination",
            "membership",destination.getMembership().stream().map(identities::get).toList(),
            "source_membership",source.getMembership().stream().map(identities::get).toList());
    }
    public static void main(String[] args) throws Exception {
        org.omg.sysml.logic.SysMLLogicStandaloneSetup.doSetup();
        for (String id : List.of("a", "b", "c", "d")) {
            Membership m=SysMLFactory.eINSTANCE.createMembership(); members.put(id,m); ids.put(m,id);
        }
        List<Object> controls=new ArrayList<>();
        for (String[] inputs : new String[][] {{"", "", ""}, {"a", "b", "c"}, {"a b", "b c", "c d"}, {"b a", "a b", "d c"}}) {
            for (boolean typed : new boolean[] {false,true}) {
                Namespace n=typed ? new TypeInput(inputs[0], inputs[1], inputs[2]) : new NamespaceInput(inputs[0], inputs[1]);
                Map<String,Object> row=new LinkedHashMap<>();
                row.put("kind",typed ? "Type" : "Namespace");
                row.put("imported_membership",names(n.getImportedMembership()));
                row.put("owned_membership",names(n.getOwnedMembership()));
                if (typed) row.put("inherited_membership",names(((Type)n).getInheritedMembership()));
                row.put("membership",names(n.getMembership()));
                controls.add(row);
            }
        }
        List<Object> canonical=new ArrayList<>();
        for (int scenario=0; scenario<17; scenario++) canonical.add(canonicalControl(scenario));
        canonical.add(canonicalControl(18));
        for (int scenario=19; scenario<26; scenario++) canonical.add(canonicalControl(scenario));
        canonical.add(canonicalControl(27)); canonical.add(canonicalControl(28));
        List<Object> chaining=new ArrayList<>();
        for (int scenario=0;scenario<9;scenario++) chaining.add(chainingControl(scenario));
        Files.writeString(Path.of(args[0]), new GsonBuilder().setPrettyPrinting().create().toJson(Map.of("chaining",chaining,"composition",controls,"canonical",canonical,"implied_disagreement",canonicalControl(17),"redefinition_alias_disagreement",canonicalControl(26))));
    }
}
