package dev.mercurio.pilot;
import com.google.gson.GsonBuilder;
import java.nio.file.*;
import java.util.*;
import org.eclipse.emf.ecore.EClass;
import org.omg.sysml.lang.sysml.*;
import org.omg.sysml.util.*;
import java.io.ByteArrayInputStream;
import java.nio.charset.StandardCharsets;
import org.eclipse.emf.common.util.URI;
import org.eclipse.xtext.resource.*;
/** Fresh positive explicit ancestry only; no complete transformation claim. */
public final class PilotExplicitSpecializationProbe {
 public static void main(String[] args)throws Exception {
  SysMLPackage.eINSTANCE.eClass();org.omg.sysml.logic.SysMLLogicStandaloneSetup.doSetup();
  var f=SysMLFactory.eINSTANCE; var controls=new ArrayList<Object>();
  for(String kind:List.of("Feature","ReferenceUsage","ActionUsage","TransitionUsage"))
   for(String shape:List.of("direct","chain","diamond","cycle")) {
    var nodes=new HashMap<String,Feature>();
    nodes.put("root",(Feature)f.create((EClass)SysMLPackage.eINSTANCE.getEClassifier(kind)));
    for(String id:List.of("a","b","goal"))nodes.put(id,f.createFeature());
    List<List<String>> edges=switch(shape){
     case "direct"->List.of(List.of("root","goal"));
     case "chain"->List.of(List.of("root","a"),List.of("a","goal"));
     case "diamond"->List.of(List.of("root","a"),List.of("root","b"),List.of("a","goal"),List.of("b","goal"));
     default->List.of(List.of("root","a"),List.of("a","root"),List.of("a","goal"));
    };
    for(var edge:edges){var r=f.createSubsetting();r.setSubsettingFeature(nodes.get(edge.get(0)));r.setSubsettedFeature(nodes.get(edge.get(1)));nodes.get(edge.get(0)).getOwnedRelationship().add(r);}
    boolean result=TypeUtil.specializes(nodes.get("root"),nodes.get("goal"));
    if(!result||nodes.get("root").isImpliedIncluded())throw new IllegalStateException(kind+shape);
    controls.add(Map.of("kind",kind,"shape",shape,"edges",edges,"specializes",result,"complete",false));
   }
  var featuring=new ArrayList<Object>();
  var injector=new org.omg.kerml.xtext.KerMLStandaloneSetup().createInjectorAndDoEMFRegistration();
  String source="standard library package Occurrences { class Occurrence; } package P;";
  var compute=org.omg.sysml.adapter.FeatureAdapter.class.getDeclaredMethod("computeFeaturingType");compute.setAccessible(true);
  for(String kind:List.of("Feature","ReferenceUsage","ActionUsage","TransitionUsage")) {
   var set=injector.getInstance(XtextResourceSet.class);var resource=(XtextResource)set.createResource(URI.createURI("memory:/explicit-featuring.kerml"));resource.load(new ByteArrayInputStream(source.getBytes(StandardCharsets.UTF_8)),Map.of());
   var names=new HashMap<String,Element>();var qualified=new HashMap<String,Element>();var it=resource.getAllContents();while(it.hasNext())if(it.next() instanceof Element e&&e.getDeclaredName()!=null){names.put(e.getDeclaredName(),e);qualified.put(e.getQualifiedName(),e);}
   SysMLLibraryUtil.setProviderLookup(r->(c,n)->qualified.get(n));
   var owner=(Feature)f.create((EClass)SysMLPackage.eINSTANCE.getEClassifier(kind));owner.setDeclaredName("owner");
   var membership=f.createOwningMembership();membership.setOwnedMemberElement(owner);((Namespace)names.get("P")).getOwnedRelationship().add(membership);
   var typing=f.createFeatureTyping();typing.setTypedFeature(owner);typing.setType((Type)names.get("Occurrence"));owner.getOwnedRelationship().add(typing);
   var candidate=f.createReferenceUsage();candidate.setDeclaredName("candidate");candidate.setIsPortion(true);
   var fm=f.createFeatureMembership();fm.setOwnedMemberFeature(candidate);owner.getOwnedRelationship().add(fm);
   boolean variable=candidate.isVariable();compute.invoke(ElementUtil.getElementAdapter(candidate));
   var before=candidate.getFeaturingType().stream().map(Element::getDeclaredName).toList();FeatureUtil.insertImplicitTypeFeaturings(candidate);
   var after=candidate.getFeaturingType().stream().map(Element::getDeclaredName).toList();
   if(variable||!before.equals(List.of("owner"))||!before.equals(after)||owner.isImpliedIncluded()||candidate.isImpliedIncluded()||!resource.getErrors().isEmpty())throw new IllegalStateException(kind+" featuring");
   featuring.add(Map.of("owner_kind",kind,"source",source,"variable",variable,"query",before,"after",after,"owner_complete",false,"candidate_complete",false));resource.unload();
  }
  Files.writeString(Path.of(args[1]),new GsonBuilder().setPrettyPrinting().create().toJson(Map.of("controls",controls,"featuring_controls",featuring))+"\n");
 }
}
