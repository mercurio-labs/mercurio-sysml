package dev.mercurio.pilot;
import com.google.gson.GsonBuilder;
import java.io.ByteArrayInputStream;
import java.nio.charset.StandardCharsets;
import java.nio.file.*;
import java.util.*;
import org.eclipse.emf.common.util.URI;
import org.eclipse.xtext.resource.*;
import org.omg.sysml.lang.sysml.*;
import org.omg.sysml.util.*;
/** Independent receiver-stage transform, including reentrant added members; supplied child lifecycle is separate. */
public final class PilotTransitionOwnerLifecycleProbe {
 public static void main(String[] args)throws Exception {
  SysMLPackage.eINSTANCE.eClass();
  org.omg.sysml.logic.SysMLLogicStandaloneSetup.doSetup();
  var injector=new org.omg.kerml.xtext.KerMLStandaloneSetup().createInjectorAndDoEMFRegistration();var f=SysMLFactory.eINSTANCE;var controls=new ArrayList<Object>();
  String text="standard library package Base { feature things; } standard library package Links { assoc Link { feature participant; } feature selfLinks; } standard library package Occurrences { class Occurrence; feature happensBeforeLinks; } standard library package TransitionPerformances { class TransitionPerformance { feature transitionLink; }  } standard library package Actions { behavior Action; feature transitionActions; } package P { class Owner; }";
  for(boolean sourceAction:List.of(false,true))for(boolean succession:List.of(false,true))for(boolean existing:List.of(false,true))for(int parameters=0;parameters<3;parameters++) {
   var set=injector.getInstance(XtextResourceSet.class);var resource=(XtextResource)set.createResource(URI.createURI("memory:/transition-members.kerml"));resource.load(new ByteArrayInputStream(text.getBytes(StandardCharsets.UTF_8)),Map.of());
   var names=new HashMap<String,Element>();var qualified=new HashMap<String,Element>();var it=resource.getAllContents();while(it.hasNext())if(it.next() instanceof Element e){if(e instanceof Type t)t.setIsImpliedIncluded(true);if(e.getDeclaredName()!=null){names.put(e.getDeclaredName(),e);qualified.put(e.getQualifiedName(),e);}}
   SysMLLibraryUtil.setProviderLookup(r->(c,n)->qualified.get(n));ElementUtil.transformAll(names.get("participant"),true);ElementUtil.transformAll(names.get("selfLinks"),true);
   var owner=(Type)names.get("Owner");var t=f.createTransitionUsage();t.setDeclaredName("transition");t.setIsImpliedIncluded(false);t.setIsPortion(true);TypeUtil.addOwnedFeatureTo(owner,t);
   Feature source=sourceAction?f.createStateUsage():f.createFeature();source.setDeclaredName("source");source.setIsImpliedIncluded(true);if(source instanceof Usage u)u.setIsPortion(true);TypeUtil.addOwnedFeatureTo(owner,source);
   var featuring=f.createTypeFeaturing();featuring.setFeaturingType(owner);source.getOwnedRelationship().add(featuring);
   var alias=f.createMembership();alias.setMemberElement(source);t.getOwnedRelationship().add(alias);
   if(succession){var end=f.createSuccessionAsUsage();end.setDeclaredName("succession");end.setIsImpliedIncluded(true);end.setIsPortion(true);var member=f.createOwningMembership();member.setOwnedMemberElement(end);t.getOwnedRelationship().add(member);}
   if(existing){var link=f.createReferenceUsage();link.setDeclaredName("existing");link.setIsImpliedIncluded(true);link.setIsPortion(true);TypeUtil.addOwnedFeatureTo(t,link);}
   for(int i=0;i<parameters;i++){var p=f.createFeature();p.setDeclaredName("parameter"+i);p.setDirection(FeatureDirectionKind.IN);p.setIsImpliedIncluded(true);var m=f.createParameterMembership();m.setOwnedMemberFeature(p);t.getOwnedRelationship().add(m);}
   var successionRole=(Feature)names.get("happensBeforeLinks");ElementUtil.transformAll(successionRole,true);var successionRoleInput=PilotCompatibilityExporter.bindingSnapshot(successionRole,null);var role=(Feature)names.get("transitionLink");ElementUtil.transformAll(role,true);var ownerRole=(Feature)names.get("transitionActions");ElementUtil.transformAll(ownerRole,true);var ownerRoleInput=PilotCompatibilityExporter.bindingSnapshot(ownerRole,null);var roleInput=PilotCompatibilityExporter.bindingSnapshot(role,null);
   var before=PilotCompatibilityExporter.bindingSnapshot(t,null);ElementUtil.transform(t);TypeUtil.insertImplicitBindingConnectors(t);TypeUtil.insertImplicitSpecializations(t);FeatureUtil.insertImplicitTypeFeaturings(t);t.setIsImpliedIncluded(true);
   var link=UsageUtil.getTransitionLinkFeatureOf(t);if(link!=null && !existing)ElementUtil.transformAll(link,true);if(link!=null && link.getDeclaredName()==null)link.setDeclaredName("generatedLink");
   var connectors=new ArrayList<Object>();for(var m:t.getOwnedMembership())if(m.getMemberElement() instanceof BindingConnector b){var row=new TreeMap<String,Object>();row.put("membership",m.eClass().getName());row.put("source",b.getSource().stream().map(Element::getDeclaredName).toList());row.put("target",b.getTarget().stream().map(Element::getDeclaredName).toList());row.put("connector",PilotCompatibilityExporter.bindingSnapshot(b,b));row.put("ends",b.getOwnedFeature().stream().map(e->PilotCompatibilityExporter.bindingSnapshot(e,b)).toList());connectors.add(row);}
   var row=new TreeMap<String,Object>();row.put("source",text);row.put("parent_complete",owner.isImpliedIncluded());row.put("succession_role_input",successionRoleInput);row.put("owner_role_input",ownerRoleInput);row.put("owner_before",before);row.put("owner_after",PilotCompatibilityExporter.bindingSnapshot(t,null));row.put("role_input",roleInput);row.put("link_snapshot",link==null?null:PilotCompatibilityExporter.bindingSnapshot(link,null));row.put("source_action",sourceAction);row.put("succession",succession);row.put("existing_link",existing);row.put("parameters",parameters);row.put("link",link==null?null:link.getDeclaredName());row.put("connectors",connectors);ElementUtil.transform(t);row.put("owner_repeat",PilotCompatibilityExporter.bindingSnapshot(t,null));row.put("repeat_connector_count",t.getOwnedMembership().stream().filter(m->m.getMemberElement() instanceof BindingConnector).count());row.put("library_inputs",Map.of("participant",PilotCompatibilityExporter.bindingSnapshot((Feature)names.get("participant"),null),"selfLinks",PilotCompatibilityExporter.bindingSnapshot((Feature)names.get("selfLinks"),null)));controls.add(row);resource.unload();
  }
  Files.writeString(Path.of(args[1]),new GsonBuilder().serializeNulls().setPrettyPrinting().create().toJson(Map.of("controls",controls))+"\n");
 }
}
