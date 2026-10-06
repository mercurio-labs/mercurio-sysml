package dev.mercurio.pilot;
import com.google.gson.GsonBuilder;
import com.sun.source.tree.*;
import com.sun.source.util.*;
import javax.tools.*;
import java.nio.file.*;
import java.util.*;
import java.io.ByteArrayInputStream;
import java.nio.charset.StandardCharsets;
import org.eclipse.emf.common.util.URI;
import org.eclipse.xtext.resource.*;
import org.omg.sysml.lang.sysml.*;
import org.omg.sysml.util.*;
/** Resolved lifecycle admission and independent fresh transition-link transforms. */
public final class PilotFreshTransitionLinkProbe {
 public static void main(String[] args)throws Exception {
  SysMLPackage.eINSTANCE.eClass();org.omg.sysml.logic.SysMLLogicStandaloneSetup.doSetup();var methods=new TreeMap<String,Object>();
  var compiler=ToolProvider.getSystemJavaCompiler();var diagnostics=new DiagnosticCollector<JavaFileObject>();
  try(var files=compiler.getStandardFileManager(diagnostics,null,StandardCharsets.UTF_8)) {
   var task=(JavacTask)compiler.getTask(null,files,diagnostics,List.of("-proc:none","-classpath",args[0]),null,files.getJavaFileObjectsFromStrings(Arrays.asList(args).subList(2,args.length)));
   var units=new ArrayList<CompilationUnitTree>();task.parse().forEach(units::add);task.analyze();var trees=Trees.instance(task);
   if(diagnostics.getDiagnostics().stream().anyMatch(d->d.getKind()==Diagnostic.Kind.ERROR))throw new IllegalStateException(diagnostics.getDiagnostics().toString());
   for(var u:units)for(var d:u.getTypeDecls())if(d instanceof ClassTree c)for(var member:c.getMembers())if(member instanceof MethodTree m && m.getBody()!=null) {
    String cls=c.getSimpleName().toString(),name=m.getName().toString();
    if((List.of("FeatureAdapter","TypeAdapter","UsageAdapter").contains(cls)&&name.equals("doTransform")) || cls.equals("FeatureAdapter")&&List.of("computeFeaturingType","computeValueConnector","forceComputeRedefinitions").contains(name) || cls.equals("UsageImpl")&&name.equals("isVariable") || cls.equals("ReferenceUsageAdapter")&&name.equals("addDefaultGeneralType"))
     methods.put(trees.getElement(TreePath.getPath(u,c))+"#"+name,PilotOwnerTypingProbe.resolved(u,m.getBody(),trees));
   }
  }
  if(methods.size()!=8)throw new IllegalStateException("Missing resolved lifecycle methods");
  var injector=new org.omg.kerml.xtext.KerMLStandaloneSetup().createInjectorAndDoEMFRegistration();var f=SysMLFactory.eINSTANCE;var controls=new ArrayList<Object>();
  String text="standard library package Base { classifier Anything; datatype DataValue; feature things; feature dataValues; } standard library package Objects { struct Object; feature objects; } standard library package Occurrences { class Occurrence { feature snapshots; } feature occurrences; assoc HappensLink; } standard library package Links { assoc SelfLink; } standard library package Actions { behavior Action; } standard library package TransitionPerformances { class TransitionPerformance { feature transitionLink; } } package P { class roleClass; struct roleStructure; datatype roleData; }";
  for(int roleMask=0;roleMask<8;roleMask++)for(boolean previous:List.of(false,true)) {
   var set=injector.getInstance(XtextResourceSet.class);var resource=(XtextResource)set.createResource(URI.createURI("memory:/fresh-transition-link.kerml"));resource.load(new ByteArrayInputStream(text.getBytes(StandardCharsets.UTF_8)),Map.of());
   var names=new HashMap<String,Element>();var qualified=new HashMap<String,Element>();var it=resource.getAllContents();while(it.hasNext())if(it.next() instanceof Element e){if(e instanceof Type t)t.setIsImpliedIncluded(true);if(e.getDeclaredName()!=null){names.put(e.getDeclaredName(),e);qualified.put(e.getQualifiedName(),e);}}
   SysMLLibraryUtil.setProviderLookup(r->(c,n)->qualified.get(n));
   var owner=f.createTransitionUsage();owner.setDeclaredName("transition");owner.setIsImpliedIncluded(true);var om=f.createOwningMembership();om.setOwnedMemberElement(owner);((Namespace)names.get("P")).getOwnedRelationship().add(om);
   if(previous){var p=f.createFeature();p.setDirection(FeatureDirectionKind.IN);var m=f.createParameterMembership();m.setOwnedMemberFeature(p);owner.getOwnedRelationship().add(m);}
   var target=f.createReferenceUsage();TypeUtil.addOwnedFeatureTo(owner,target);
   var role=(Feature)names.get("transitionLink");int bit=0;for(String kind:List.of("roleData","roleClass","roleStructure")){if((roleMask&(1<<bit))!=0){var t=f.createFeatureTyping();t.setTypedFeature(role);t.setType((Type)names.get(kind));role.getOwnedRelationship().add(t);}bit++;}
   var libraryInputs=new TreeMap<String,Object>();for(String name:List.of("dataValues","objects","occurrences")){var input=(Feature)names.get(name);ElementUtil.transformAll(input,true);libraryInputs.put(name,PilotCompatibilityExporter.bindingSnapshot(input,null));}
   ElementUtil.transformAll(role,true);var roleInput=PilotCompatibilityExporter.bindingSnapshot(role,null);var before=target.isImpliedIncluded();ElementUtil.transformAll(target,true);
   var row=new TreeMap<String,Object>();row.put("source",text);row.put("role_input",roleInput);row.put("library_inputs",libraryInputs);row.put("role_mask",roleMask);row.put("preceding_parameter",previous);row.put("complete_before",before);row.put("after",PilotCompatibilityExporter.bindingSnapshot(target,null));row.put("name",target.effectiveName());row.put("may_time_vary",target.isMayTimeVary());row.put("definition",target.getDefinition().stream().map(Element::getDeclaredName).toList());
   var snapshot=row.get("after");ElementUtil.transformAll(target,true);row.put("repeat",PilotCompatibilityExporter.bindingSnapshot(target,null));controls.add(row);resource.unload();
  }
  Files.writeString(Path.of(args[1]),new GsonBuilder().serializeNulls().setPrettyPrinting().create().toJson(Map.of("methods",methods,"controls",controls))+"\n");
 }
}
