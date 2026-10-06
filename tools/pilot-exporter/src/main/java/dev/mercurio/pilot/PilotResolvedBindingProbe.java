package dev.mercurio.pilot;
import com.google.gson.GsonBuilder;
import java.io.ByteArrayInputStream;
import java.nio.charset.StandardCharsets;
import java.nio.file.*;
import java.util.*;
import org.eclipse.emf.common.util.URI;
import org.eclipse.emf.ecore.*;
import org.eclipse.xtext.resource.*;
import org.omg.sysml.lang.sysml.*;
import org.omg.sysml.util.*;
public final class PilotResolvedBindingProbe {
 public static void main(String[] args)throws Exception {
  SysMLPackage.eINSTANCE.eClass();
  org.omg.sysml.logic.SysMLLogicStandaloneSetup.doSetup();
  var injector=new org.omg.kerml.xtext.KerMLStandaloneSetup().createInjectorAndDoEMFRegistration();var f=SysMLFactory.eINSTANCE;var controls=new ArrayList<Object>();
  String source="standard library package Base { feature things; } standard library package Links { assoc Link { feature participant; } feature selfLinks; } standard library package Occurrences { class Occurrence; } package P { class Owner { feature x; feature y; } class Other; }";
  var kinds=List.of("StateUsage","ReferenceUsage","TransitionUsage","SuccessionAsUsage");
  for(String first:kinds)for(String second:kinds)for(String ownerKind:List.of("Class","TransitionUsage"))for(String placement:List.of("same","other")) {
   var set=injector.getInstance(XtextResourceSet.class);var resource=(XtextResource)set.createResource(URI.createURI("memory:/resolved-binding.kerml"));resource.load(new ByteArrayInputStream(source.getBytes(StandardCharsets.UTF_8)),Map.of());
   var names=new HashMap<String,Element>();var qualified=new HashMap<String,Element>();var it=resource.getAllContents();while(it.hasNext())if(it.next() instanceof Element e){if(e instanceof Type t)t.setIsImpliedIncluded(true);if(e.getDeclaredName()!=null){names.put(e.getDeclaredName(),e);qualified.put(e.getQualifiedName(),e);}}
   SysMLLibraryUtil.setProviderLookup(r->(c,n)->qualified.get(n));
   // Complete the bounded library Features through the real upstream producer,
   // rather than assume their stored flags contain the required defaults.
   ElementUtil.transformAll(names.get("participant"),true);
   ElementUtil.transformAll(names.get("selfLinks"),true);

   var original=(Type)names.get("Owner");Type owner=original;
   if(!ownerKind.equals("Class")){owner=f.createTransitionUsage();owner.setDeclaredName("Owner");owner.setIsImpliedIncluded(true);var m=(OwningMembership)original.getOwningRelationship();m.setOwnedMemberElement(owner);owner.getOwnedRelationship().addAll(new ArrayList<>(original.getOwnedRelationship()));}
   var endpoints=new ArrayList<Feature>();int index=0;
   for(String kind:List.of(first,second)){String name=index++==0?"x":"y";var old=(Feature)names.get(name);var member=(FeatureMembership)old.getOwningRelationship();var feature=(Usage)f.create((EClass)SysMLPackage.eINSTANCE.getEClassifier(kind));feature.setDeclaredName(name);feature.setIsImpliedIncluded(true);feature.setIsPortion(true);feature.setIsComposite(false);member.setOwnedMemberFeature(feature);var featuring=f.createTypeFeaturing();featuring.setFeatureOfType(feature);featuring.setFeaturingType(owner);feature.getOwnedRelationship().add(featuring);endpoints.add(feature);}
   Type caller=placement.equals("same")?owner:(Type)names.get("Other");var adapter=(org.omg.sysml.adapter.TypeAdapter)ElementUtil.getElementAdapter(caller);
   var binding=adapter.addBindingConnector(endpoints.get(0),endpoints.get(1));TypeUtil.insertImplicitBindingConnectors(caller);
   var ends=binding.getOwnedFeature().stream().map(e->PilotCompatibilityExporter.bindingSnapshot(e,binding)).toList();
   if(!resource.getErrors().isEmpty())throw new IllegalStateException(resource.getErrors().toString());
   var row=new TreeMap<String,Object>(Map.of("source",source,"first",first,"second",second,"owner_kind",ownerKind,"placement",placement,"membership",binding.getOwningRelationship().eClass().getName(),"connector",PilotCompatibilityExporter.bindingSnapshot(binding,binding),"ends",ends,"source_endpoint",binding.getSource().stream().map(Element::getDeclaredName).toList(),"target_endpoint",binding.getTarget().stream().map(Element::getDeclaredName).toList()));row.put("library_inputs",Map.of("participant",PilotCompatibilityExporter.bindingSnapshot((Feature)names.get("participant"),binding),"selfLinks",PilotCompatibilityExporter.bindingSnapshot((Feature)names.get("selfLinks"),binding)));controls.add(row);resource.unload();
  }
  Files.writeString(Path.of(args[1]),new GsonBuilder().setPrettyPrinting().create().toJson(Map.of("endpoint_kinds",kinds,"controls",controls))+"\n");
 }
}
