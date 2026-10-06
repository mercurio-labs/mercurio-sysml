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
/** Fixed owning-type computation plus derived read; not full transformation. */
public final class PilotFixedFeaturingProbe {
 public static void main(String[] args)throws Exception {
  var injector=new org.omg.kerml.xtext.KerMLStandaloneSetup().createInjectorAndDoEMFRegistration();
  var f=SysMLFactory.eINSTANCE;var kinds=new ArrayList<String>();var controls=new ArrayList<Object>();
  for(var c:SysMLPackage.eINSTANCE.getEClassifiers())if(c instanceof EClass cls&&!cls.isAbstract()&&f.create(cls) instanceof Feature)kinds.add(cls.getName());Collections.sort(kinds);
  var compute=org.omg.sysml.adapter.FeatureAdapter.class.getDeclaredMethod("computeFeaturingType");compute.setAccessible(true);
  String source="standard library package Occurrences { class Occurrence; } package P { class Owner :> Occurrences::Occurrence { feature candidate; } class Other { feature chain_a; feature chain_b; } class Explicit; }";
  for(String kind:kinds)for(String shape:List.of("empty","explicit","duplicate","chain")) {
   var set=injector.getInstance(XtextResourceSet.class);var resource=(XtextResource)set.createResource(URI.createURI("memory:/fixed-featuring.kerml"));resource.load(new ByteArrayInputStream(source.getBytes(StandardCharsets.UTF_8)),Map.of());
   var nodes=new HashMap<String,Type>();var it=resource.getAllContents();while(it.hasNext())if(it.next() instanceof Type t){t.setIsImpliedIncluded(true);if(t.getDeclaredName()!=null)nodes.put(t.getDeclaredName(),t);}
   var original=(Feature)nodes.get("candidate");var member=(FeatureMembership)original.getOwningRelationship();
   var feature=(Feature)f.create((EClass)SysMLPackage.eINSTANCE.getEClassifier(kind));feature.setDeclaredName("candidate");feature.setIsImpliedIncluded(false);
   if(feature instanceof Usage u){u.setIsPortion(true);u.setIsComposite(false);}else feature.setIsVariable(false);
   member.setOwnedMemberFeature(feature);
   if(!shape.equals("empty"))for(int i=0;i<(shape.equals("duplicate")?2:1);i++){var r=f.createTypeFeaturing();r.setFeatureOfType(feature);r.setFeaturingType(nodes.get("Explicit"));feature.getOwnedRelationship().add(r);}
   if(shape.equals("chain"))for(String n:List.of("chain_a","chain_b")){var endpoint=(Feature)nodes.get(n);compute.invoke(ElementUtil.getElementAdapter(endpoint));var c=f.createFeatureChaining();c.setChainingFeature(endpoint);feature.getOwnedRelationship().add(c);}
   boolean variable=feature.isVariable();if(variable)throw new IllegalStateException("Nonfixed control "+kind);
   compute.invoke(ElementUtil.getElementAdapter(feature));var before=feature.getFeaturingType().stream().map(Type::getDeclaredName).toList();
   FeatureUtil.insertImplicitTypeFeaturings(feature);var after=feature.getFeaturingType().stream().map(Type::getDeclaredName).toList();
   if(!before.equals(after)||feature.isImpliedIncluded()||!resource.getErrors().isEmpty())throw new IllegalStateException("Invalid fixed featuring observation "+kind+" "+resource.getErrors());
   controls.add(Map.of("kind",kind,"shape",shape,"source",source,"query",before,"after",after,"variable",variable,"complete",feature.isImpliedIncluded()));resource.unload();
  }
  Files.writeString(Path.of(args[1]),new GsonBuilder().setPrettyPrinting().create().toJson(Map.of("feature_kinds",kinds,"controls",controls))+"\n");
 }
}
