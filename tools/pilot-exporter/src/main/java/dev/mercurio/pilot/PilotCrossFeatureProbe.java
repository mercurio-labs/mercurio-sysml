package dev.mercurio.pilot;
import com.google.gson.GsonBuilder;
import java.nio.file.*;
import java.util.*;
import org.omg.sysml.lang.sysml.*;
import org.omg.sysml.util.*;
import org.omg.sysml.adapter.*;
public final class PilotCrossFeatureProbe {
    static void own(Namespace owner, OwningMembership membership, Element child) {
        membership.setVisibility(VisibilityKind.PUBLIC); membership.getOwnedRelatedElement().add(child); owner.getOwnedRelationship().add(membership);
    }
    public static void main(String[] args) throws Exception {
        org.omg.sysml.logic.SysMLLogicStandaloneSetup.doSetup();
        var f = SysMLFactory.eINSTANCE;
        var selections = new ArrayList<Object>();
        for (String candidateKind : List.of("Feature", "MultiplicityRange", "MetadataFeature", "BindingConnector", "Expression", "ReferenceUsage"))
            for (String memberKind : List.of("OwningMembership", "FeatureMembership", "FeatureValue"))
                for (boolean end : List.of(false,true)) for (boolean typedOwner : List.of(false,true)) {
                    var feature=f.createFeature(); feature.setIsEnd(end); feature.setDeclaredName("end");
                    Namespace context=typedOwner ? f.createClassifier() : f.createPackage();
                    own(context,typedOwner ? f.createFeatureMembership() : f.createOwningMembership(),feature);
                    var candidate=(Feature)f.create((org.eclipse.emf.ecore.EClass)SysMLPackage.eINSTANCE.getEClassifier(candidateKind));
                    candidate.setDeclaredName("candidate");
                    var member=(OwningMembership)f.create((org.eclipse.emf.ecore.EClass)SysMLPackage.eINSTANCE.getEClassifier(memberKind));
                    own(feature,member,candidate);
                    var selected=FeatureUtil.getOwnedCrossFeatureOf(feature);
                    selections.add(Map.of("candidate_kind",candidateKind,"membership_kind",memberKind,"is_end",end,"type_owner",typedOwner,
                        "selected",selected==null ? List.of() : List.of(selected.getDeclaredName())));
                }
        var orderedSelections = new ArrayList<Object>();
        for (var kinds : List.of(List.of("MetadataFeature","Feature","Feature"),List.of("Feature","MultiplicityRange","Feature"),List.of("MultiplicityRange","MetadataFeature","BindingConnector"))) {
            var context=f.createClassifier();var end=f.createFeature();end.setIsEnd(true);own(context,f.createFeatureMembership(),end);
            for(int i=0;i<kinds.size();i++) {
                var member=(Feature)f.create((org.eclipse.emf.ecore.EClass)SysMLPackage.eINSTANCE.getEClassifier(kinds.get(i)));
                member.setDeclaredName("member"+i);own(end,f.createOwningMembership(),member);
            }
            var selected=FeatureUtil.getOwnedCrossFeatureOf(end);
            orderedSelections.add(Map.of("kinds",kinds,"selected",selected==null ? List.of() : List.of(selected.getDeclaredName())));
        }
        var selector = FeatureAdapter.class.getDeclaredMethod("getDefaultSupertype");selector.setAccessible(true);
        var crossings = new ArrayList<Object>();
        for (int targetIndex=0;targetIndex<2;targetIndex++) for (boolean end:List.of(false,true)) for(boolean existing:List.of(false,true)) {
            var context=f.createClassifier();
            var ends=new ArrayList<Feature>();
            for(int i=0;i<2;i++) {
                var feature=f.createFeature();feature.setDeclaredName("end"+i);feature.setIsEnd(i!=targetIndex || end);
                own(context,f.createFeatureMembership(),feature);ends.add(feature);
            }
            var owner=ends.get(targetIndex);var cross=f.createFeature();cross.setDeclaredName("cross");own(owner,f.createOwningMembership(),cross);
            if(existing) {var relation=f.createCrossSubsetting();var target=f.createFeature();target.setDeclaredName("existing");relation.setCrossedFeature(target);owner.getOwnedRelationship().add(relation);}
            var adapter=(FeatureAdapter)ElementUtil.getElementAdapter(owner);adapter.addCrossingSpecialization();
            var selected=adapter.getImplicitGeneralTypesOnly(SysMLPackage.Literals.CROSS_SUBSETTING);
            var chains=new ArrayList<Object>();
            var chainDefaults=new ArrayList<String>();
            for(var target:selected) {
                chains.add(((Feature)target).getChainingFeature().stream().map(Element::getDeclaredName).toList());
                chainDefaults.add((String)selector.invoke(ElementUtil.getElementAdapter(target)));
            }
            crossings.add(Map.of("target_index",targetIndex,"is_end",end,"existing",existing,"chains",chains,"chain_defaults",chainDefaults));
        }
        var specializations = new ArrayList<Object>();
        var specialize = FeatureAdapter.class.getDeclaredMethod("addOwnedCrossFeatureSpecialization");
        specialize.setAccessible(true);
        for (int count=0;count<3;count++) for (boolean redefinedEnd:List.of(false,true)) for (boolean withCross:List.of(false,true)) {
            var context=f.createClassifier();var owner=f.createFeature();owner.setIsEnd(true);owner.setIsImpliedIncluded(true);
            own(context,f.createFeatureMembership(),owner);
            var cross=f.createFeature();own(owner,f.createOwningMembership(),cross);
            for(int i=0;i<count;i++) {
                var type=f.createClassifier();type.setDeclaredName("type"+i);type.setIsImpliedIncluded(true);
                var typing=f.createFeatureTyping();typing.setType(type);typing.setSpecific(owner);owner.getOwnedRelationship().add(typing);
            }
            var general=f.createFeature();general.setIsEnd(redefinedEnd);general.setIsImpliedIncluded(true);
            var redefinition=f.createRedefinition();redefinition.setRedefinedFeature(general);redefinition.setSpecific(owner);owner.getOwnedRelationship().add(redefinition);
            if(withCross) {
                var chain=f.createFeature();
                for(String name:List.of("other","generalCross")) {
                    var member=f.createFeature();member.setDeclaredName(name);member.setIsImpliedIncluded(true);
                    var chaining=f.createFeatureChaining();chaining.setChainingFeature(member);chain.getOwnedRelationship().add(chaining);
                }
                var crossing=f.createCrossSubsetting();crossing.setCrossedFeature(chain);crossing.setSpecific(general);general.getOwnedRelationship().add(crossing);
            }
            if (!FeatureUtil.isOwnedCrossFeature(cross)) throw new IllegalStateException("Cross selection failed");
            if (owner.getType().size()!=count) throw new IllegalStateException("Owner types " + owner.getType().size()+" expected "+count);
            if (FeatureUtil.getRedefinedFeaturesWithComputedOf(owner).size()!=1) throw new IllegalStateException("Redefinition missing");
            var adapter=(FeatureAdapter)ElementUtil.getElementAdapter(cross);specialize.invoke(adapter);
            specializations.add(Map.of("type_count",count,"redefined_end",redefinedEnd,"with_cross",withCross,
                "types",adapter.getImplicitGeneralTypesOnly(SysMLPackage.Literals.FEATURE_TYPING).stream().map(Element::getDeclaredName).toList(),
                "subsets",adapter.getImplicitGeneralTypesOnly(SysMLPackage.Literals.SUBSETTING).stream().map(Element::getDeclaredName).toList()));
        }
        var featuringControls = new ArrayList<Object>();
        for(int count=0;count<3;count++) for(int position=0;position<2;position++) for(boolean existing:List.of(false,true)) for(boolean inherited:List.of(false,true)) {
            var context=f.createClassifier();context.setIsImpliedIncluded(true);
            var general=f.createClassifier();general.setIsImpliedIncluded(true);
            var anchor=f.createFeature();anchor.setIsEnd(true);anchor.setDeclaredName("anchor");anchor.setIsImpliedIncluded(true);
            if(inherited) own(general,f.createFeatureMembership(),anchor);
            if(inherited) {var specialization=f.createSubclassification();specialization.setSubclassifier(context);specialization.setSuperclassifier(general);context.getOwnedRelationship().add(specialization);}
            var ends=new ArrayList<Feature>();
            for(int i=0;i<2;i++) {var end=f.createFeature();end.setIsEnd(true);end.setDeclaredName("end"+i);end.setIsImpliedIncluded(true);own(inherited && i!=position ? general : context,f.createFeatureMembership(),end);ends.add(end);}
            if(inherited) {var redef=f.createRedefinition();redef.setSpecific(ends.get(position));redef.setRedefinedFeature(anchor);ends.get(position).getOwnedRelationship().add(redef);}
            var cross=f.createFeature();own(ends.get(position),f.createOwningMembership(),cross);
            for(int i=0;i<count;i++) {
                var type=f.createClassifier();type.setDeclaredName("type"+i);type.setIsImpliedIncluded(true);
                var typing=f.createFeatureTyping();typing.setSpecific(ends.get(1-position));typing.setType(type);ends.get(1-position).getOwnedRelationship().add(typing);
            }
            if(existing) {var type=f.createClassifier();var relation=f.createTypeFeaturing();relation.setFeatureOfType(cross);relation.setFeaturingType(type);cross.getOwnedRelationship().add(relation);}
            if(context.getEndFeature().size()!=2 || ends.get(1-position).getType().size()!=count || !FeatureUtil.isOwnedCrossFeature(cross)) throw new IllegalStateException("Invalid featuring fixture: ends="+context.getEndFeature().size()+" types="+ends.get(1-position).getType().size()+" selected="+FeatureUtil.isOwnedCrossFeature(cross)+" inherited="+inherited+" specials="+context.getOwnedSpecialization().size()+" generalFeatures="+general.getFeature().size()+" inheritedMemberships="+context.getInheritedMembership().size()+" ownerRedefs="+FeatureUtil.getRedefinedFeaturesWithComputedOf(ends.get(position)).size());
            var adapter=(FeatureAdapter)ElementUtil.getElementAdapter(cross);adapter.addOwnedCrossFeatureTypeFeaturing();
            var targets=new ArrayList<String>();adapter.forEachImplicitFeaturingType(type -> targets.add(type.getDeclaredName()));
            featuringControls.add(Map.of("type_count",count,"position",position,"existing",existing,"inherited",inherited,"targets",targets));
        }
        var typingControls = new ArrayList<Object>();
        for(int count=0;count<3;count++) for(boolean storedCrossing:List.of(false,true)) {
            var context=f.createClass();context.setIsImpliedIncluded(true);
            var owner=f.createFeature();owner.setIsEnd(true);own(context,f.createFeatureMembership(),owner);
            var other=f.createFeature();other.setIsEnd(true);other.setIsImpliedIncluded(true);own(context,f.createFeatureMembership(),other);
            var cross=f.createFeature();own(owner,f.createOwningMembership(),cross);
            for(int i=0;i<count;i++) {
                var type=f.createClassifier();type.setDeclaredName("type"+i);type.setIsImpliedIncluded(true);
                var typing=f.createFeatureTyping();typing.setSpecific(owner);typing.setType(type);owner.getOwnedRelationship().add(typing);
            }
            if(storedCrossing) {var target=f.createFeature();target.setIsImpliedIncluded(true);var crossing=f.createCrossSubsetting();crossing.setSpecific(owner);crossing.setCrossedFeature(target);owner.getOwnedRelationship().add(crossing);}
            if(owner.isImpliedIncluded() || !FeatureUtil.isOwnedCrossFeature(cross)) throw new IllegalStateException("Invalid typing fixture");
            var types=owner.getType().stream().map(Element::getDeclaredName).toList();
            var adapter=(FeatureAdapter)ElementUtil.getElementAdapter(cross);specialize.invoke(adapter);
            typingControls.add(Map.of("type_count",count,"stored_crossing",storedCrossing,"types",types,
                "specialization_types",adapter.getImplicitGeneralTypesOnly(SysMLPackage.Literals.FEATURE_TYPING).stream().map(Element::getDeclaredName).toList(), "default",(String)selector.invoke(adapter)));
        }
        var explicitControls = new ArrayList<Object>();
        for(boolean end:List.of(false,true)) for(int length=1;length<=3;length++) {
            var context=f.createClass();context.setIsImpliedIncluded(true);
            var owner=f.createFeature();owner.setIsEnd(end);own(context,f.createFeatureMembership(),owner);
            var chain=f.createFeature();
            for(int i=0;i<length;i++) {
                var member=f.createFeature();member.setIsImpliedIncluded(true);
                if(i==length-1) {var type=f.createClassifier();type.setDeclaredName("targetType");type.setIsImpliedIncluded(true);var typing=f.createFeatureTyping();typing.setSpecific(member);typing.setType(type);member.getOwnedRelationship().add(typing);}
                var part=f.createFeatureChaining();part.setChainingFeature(member);chain.getOwnedRelationship().add(part);
            }
            var relation=f.createCrossSubsetting();relation.setCrossedFeature(chain);relation.setSpecific(owner);relation.getOwnedRelatedElement().add(chain);owner.getOwnedRelationship().add(relation);
            explicitControls.add(Map.of("is_end",end,"chain_length",length,
                "owner_default",(String)selector.invoke(ElementUtil.getElementAdapter(owner)),
                "chain_default",(String)selector.invoke(ElementUtil.getElementAdapter(chain)),
                "owner_types",owner.getType().stream().map(Element::getDeclaredName).toList(),
                "chain_types",chain.getType().stream().map(Element::getDeclaredName).toList()));
        }
        var crossFeatureControls = new ArrayList<Object>();
        for(boolean end:List.of(false,true)) for(boolean typeOwner:List.of(false,true)) for(boolean candidate:List.of(false,true)) {
            Namespace context=typeOwner ? f.createClass() : f.createPackage();context.setIsImpliedIncluded(true);
            var owner=f.createFeature();owner.setIsEnd(end);own(context,typeOwner ? f.createFeatureMembership() : f.createOwningMembership(),owner);
            var other=f.createFeature();other.setIsEnd(true);other.setIsImpliedIncluded(true);own(context,typeOwner ? f.createFeatureMembership() : f.createOwningMembership(),other);
            if(candidate) {var cross=f.createFeature();cross.setDeclaredName("cross");own(owner,f.createOwningMembership(),cross);}
            var selected=FeatureUtil.getCrossFeatureOf(owner);
            crossFeatureControls.add(Map.of("is_end",end,"type_owner",typeOwner,"candidate",candidate,"cross_feature",selected==null ? List.of() : List.of(selected.getDeclaredName())));
        }
        var sourceControls = new ArrayList<Object>();
        var injector = new org.omg.kerml.xtext.KerMLStandaloneSetup().createInjectorAndDoEMFRegistration();
        for (String bounds : List.of("", " [1]", " [0..*]")) for (boolean reverse : List.of(false,true)) {
            String end = "end cross" + bounds + " feature e;";
            String source = "classifier C { " + (reverse ? "end feature other; " + end : end + " end feature other;") + " }";
            var resources = injector.getInstance(org.eclipse.xtext.resource.XtextResourceSet.class);
            var resource = resources.createResource(org.eclipse.emf.common.util.URI.createURI("memory:/cross-source.kerml"));
            resource.load(new java.io.ByteArrayInputStream(source.getBytes(java.nio.charset.StandardCharsets.UTF_8)),Map.of());
            if (!resource.getErrors().isEmpty()) throw new IllegalStateException("Invalid source control " + source + resource.getErrors());
            var iterator = resource.getAllContents();
            while (iterator.hasNext()) {
                var item=iterator.next();
                if (item instanceof Feature owner && "e".equals(owner.getDeclaredName())) {
                    var adapter=(FeatureAdapter)ElementUtil.getElementAdapter(owner);
                    var cross=FeatureUtil.getOwnedCrossFeatureOf(owner);
                    adapter.addCrossingSpecialization();
                    var chains=new ArrayList<Object>();
                    for (var target:adapter.getImplicitGeneralTypesOnly(SysMLPackage.Literals.CROSS_SUBSETTING))
                        chains.add(((Feature)target).getChainingFeature().stream().map(Element::getDeclaredName).toList());
                    sourceControls.add(Map.of("source",source,"cross",cross.getDeclaredName(),"chains",chains));
                }
            }
            resource.unload();
        }
        Files.writeString(Path.of(args[0]),new GsonBuilder().setPrettyPrinting().create().toJson(Map.of("cross_feature_controls",crossFeatureControls,"explicit_controls",explicitControls,"typing_controls",typingControls,"featuring_controls",featuringControls,"specialization_controls",specializations,"selection_controls",selections,"ordered_selection_controls",orderedSelections,"binary_controls",crossings,"source_controls",sourceControls))+"\n");
    }
}
