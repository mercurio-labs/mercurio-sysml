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
import static dev.mercurio.pilot.PilotStepStrategyProbe.*;
public final class PilotReferenceLifecycleProbe {
 public static void main(String[] args)throws Exception {
  SysMLPackage.eINSTANCE.eClass();org.omg.sysml.logic.SysMLLogicStandaloneSetup.doSetup();
  var methods=new TreeMap<String,Object>();var constants=new TreeMap<String,Object>();
  var compiler=ToolProvider.getSystemJavaCompiler();var diagnostics=new DiagnosticCollector<JavaFileObject>();
  try(var files=compiler.getStandardFileManager(diagnostics,null,StandardCharsets.UTF_8)) {
   var task=(JavacTask)compiler.getTask(null,files,diagnostics,List.of("-proc:none","-classpath",args[0]),null,files.getJavaFileObjectsFromStrings(Arrays.asList(args).subList(2,args.length)));
   var units=new ArrayList<CompilationUnitTree>();task.parse().forEach(units::add);task.analyze();var trees=Trees.instance(task);
   require(diagnostics.getDiagnostics().stream().noneMatch(d->d.getKind()==Diagnostic.Kind.ERROR),"Unresolved Reference link algorithms");
   for(var u:units)for(var d:u.getTypeDecls())if(d instanceof ClassTree c)for(var member:c.getMembers()) {
    String cls=c.getSimpleName().toString();
    if(member instanceof MethodTree m && m.getBody()!=null && (cls.equals("ReferenceUsageAdapter")&&List.of("addRedefinitions","addDefaultGeneralType").contains(m.getName().toString()) || cls.equals("UsageUtil")&&m.getName().contentEquals("getTransitionLinkFeatureOf") || cls.equals("FeatureAdapter")&&List.of("getRedefinedFeaturesWithComputed","isComputeRedefinitions").contains(m.getName().toString())))
     methods.put(trees.getElement(TreePath.getPath(u,c))+"#"+m.getName(),PilotTransitionSourceProbe.resolved(u,m.getBody(),trees));
    if(cls.equals("ReferenceUsageAdapter")&&member instanceof VariableTree v&&v.getName().contentEquals("TRANSITION_LINK_FEATURE"))constants.put(v.getName().toString(),PilotTransitionSourceProbe.resolved(u,v.getInitializer(),trees));
   }
  }
  require(methods.size()==5&&constants.size()==1,"Incomplete resolved reference programs");
  var injector=new org.omg.kerml.xtext.KerMLStandaloneSetup().createInjectorAndDoEMFRegistration();var f=SysMLFactory.eINSTANCE;var controls=new ArrayList<Object>();
  String source="standard library package Base { classifier Anything; datatype DataValue; feature things; feature dataValues; } standard library package Objects { struct Object; feature objects; } standard library package Occurrences { class Occurrence; feature occurrences; } standard library package TransitionPerformances { class TransitionPerformance { feature transitionLink; } } package P { class own0; struct own1; datatype own2; class role0; struct role1; datatype role2; }";
  for(int ownedMask=0;ownedMask<8;ownedMask++)for(int roleMask=0;roleMask<8;roleMask++)for(boolean parameter:List.of(false,true))for(boolean explicit:List.of(false,true)) {
   var set=injector.getInstance(XtextResourceSet.class);var resource=(XtextResource)set.createResource(URI.createURI("memory:/reference-lifecycle.kerml"));resource.load(new ByteArrayInputStream(source.getBytes(StandardCharsets.UTF_8)),Map.of());
   var names=new HashMap<String,Element>();var qualified=new HashMap<String,Element>();var it=resource.getAllContents();while(it.hasNext())if(it.next() instanceof Element e && e.getDeclaredName()!=null){names.put(e.getDeclaredName(),e);qualified.put(e.getQualifiedName(),e);}
   SysMLLibraryUtil.setProviderLookup(r->(c,n)->qualified.get(n));
   var transition=f.createTransitionUsage();transition.setDeclaredName("transition");PilotTransitionSourceProbe.own((Namespace)names.get("P"),transition,"OwningMembership");
   if(parameter) {var previous=f.createReferenceUsage();previous.setDeclaredName("previous");PilotTransitionSourceProbe.own(transition,previous,"ParameterMembership");}
   var target=f.createReferenceUsage();target.setDeclaredName("candidate");PilotTransitionSourceProbe.own(transition,target,"FeatureMembership");
   Feature role=(Feature)names.get("transitionLink");
   for(int bit=0;bit<3;bit++) {
    if((ownedMask & (1<<bit))!=0) {var typing=f.createFeatureTyping();typing.setTypedFeature(target);typing.setType((Type)names.get("own"+bit));target.getOwnedRelationship().add(typing);}
    if((roleMask & (1<<bit))!=0) {var typing=f.createFeatureTyping();typing.setTypedFeature(role);typing.setType((Type)names.get("role"+bit));role.getOwnedRelationship().add(typing);}
   }
   if(explicit) {var r=f.createRedefinition();r.setRedefiningFeature(target);r.setRedefinedFeature(role);target.getOwnedRelationship().add(r);}
   var adapter=(org.omg.sysml.adapter.FeatureAdapter)ElementUtil.getElementAdapter(target);var selector=method(adapter.getClass(),"getDefaultSupertype");selector.setAccessible(true);
   String selected=(String)selector.invoke(adapter);
   var types=FeatureUtil.getAllTypesOf(target).stream().map(Element::getDeclaredName).toList();
   var projected=target.getDefinition().stream().map(Element::getDeclaredName).toList();
   controls.add(Map.of("source",source,"owned_mask",ownedMask,"role_mask",roleMask,"preceding_parameter",parameter,"explicit_role",explicit,"selected",UsageUtil.getTransitionLinkFeatureOf(transition)==target,"default_supertype",selected,"types",types,"definitions",projected,"candidate_complete_after_query",target.isImpliedIncluded()));resource.unload();
  }
  Files.writeString(Path.of(args[1]),new GsonBuilder().serializeNulls().setPrettyPrinting().create().toJson(Map.of("methods",methods,"constants",constants,"controls",controls))+"\n");
 }
}
