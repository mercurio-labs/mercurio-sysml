package dev.mercurio.pilot;
import com.google.gson.GsonBuilder;
import java.nio.file.*;
import java.util.*;
import java.io.ByteArrayInputStream;
import java.nio.charset.StandardCharsets;
import org.eclipse.emf.common.util.URI;
import org.eclipse.xtext.resource.*;
import org.omg.sysml.lang.sysml.*;
import org.omg.sysml.util.*;
/** Variable Feature snapshot construction; fresh factory candidate, fixture resources. */
public final class PilotSnapshotFeaturingProbe {
 public static void main(String[] args)throws Exception {
  SysMLPackage.eINSTANCE.eClass();org.omg.sysml.logic.SysMLLogicStandaloneSetup.doSetup();
  var injector=new org.omg.kerml.xtext.KerMLStandaloneSetup().createInjectorAndDoEMFRegistration();
  var f=SysMLFactory.eINSTANCE;var controls=new ArrayList<Object>();
  var compute=org.omg.sysml.adapter.FeatureAdapter.class.getDeclaredMethod("computeFeaturingType");compute.setAccessible(true);
  String source="standard library package Base { classifier Anything; feature things; } standard library package Links { class SelfLink; } standard library package Occurrences { class HappensLink; class Occurrence { feature snapshots; } } package P { class Owner; package Nested { class Inner; } class Explicit; }";
  for(String kind:List.of("Feature","ReferenceUsage"))for(String shape:List.of("named","nested","anonymous","occurrence"))for(boolean explicit:List.of(false,true)) {
   var set=injector.getInstance(XtextResourceSet.class);var resource=(XtextResource)set.createResource(URI.createURI("memory:/snapshot.kerml"));resource.load(new ByteArrayInputStream(source.getBytes(StandardCharsets.UTF_8)),Map.of());
   var names=new HashMap<String,Element>();var qualified=new HashMap<String,Element>();var it=resource.getAllContents();while(it.hasNext())if(it.next() instanceof Element e&&e.getDeclaredName()!=null){names.put(e.getDeclaredName(),e);qualified.put(e.getQualifiedName(),e);}
   SysMLLibraryUtil.setProviderLookup(r->(c,n)->qualified.get(n));
   var owner=(Type)names.get(shape.equals("nested")?"Inner":shape.equals("occurrence")?"Occurrence":"Owner");if(shape.equals("anonymous"))owner.setDeclaredName(null);
   var candidate=(Feature)f.create((org.eclipse.emf.ecore.EClass)SysMLPackage.eINSTANCE.getEClassifier(kind));candidate.setDeclaredName("candidate");if(!(candidate instanceof Usage))candidate.setIsVariable(true);
   var membership=f.createFeatureMembership();membership.setOwnedMemberFeature(candidate);owner.getOwnedRelationship().add(membership);
   if(explicit){var r=f.createTypeFeaturing();r.setFeatureOfType(candidate);r.setFeaturingType((Type)names.get("Explicit"));candidate.getOwnedRelationship().add(r);}
   if(!candidate.isVariable())throw new IllegalStateException(kind+shape+" not variable");
   Type target=(Type)compute.invoke(ElementUtil.getElementAdapter(candidate));
   var before=candidate.getFeaturingType().stream().map(Element::getDeclaredName).toList();
   FeatureUtil.insertImplicitTypeFeaturings(candidate);var after=candidate.getFeaturingType().stream().map(Element::getDeclaredName).toList();
   var contexts=target instanceof Feature feature?feature.getOwnedTypeFeaturing().stream().map(r->r.getFeaturingType()==owner).toList():List.of();
   var redefined=target instanceof Feature feature?feature.getOwnedRedefinition().stream().map(r->r.getRedefinedFeature().getDeclaredName()).toList():List.of();
   if(!before.equals(after)||candidate.isImpliedIncluded()||!resource.getErrors().isEmpty())throw new IllegalStateException(shape+" snapshot");
   var row=new TreeMap<String,Object>();row.put("kind",kind);row.put("variable",candidate.isVariable());row.put("shape",shape);row.put("source",source);row.put("explicit",explicit);row.put("query",before);row.put("after",after);row.put("target_name",target.getDeclaredName());row.put("direct_library",target==names.get("snapshots"));row.put("context_is_owner",contexts);row.put("redefined",redefined);row.put("candidate_complete",false);controls.add(row);resource.unload();
  }
  Files.writeString(Path.of(args[1]),new GsonBuilder().serializeNulls().setPrettyPrinting().create().toJson(Map.of("controls",controls))+"\n");
 }
}
