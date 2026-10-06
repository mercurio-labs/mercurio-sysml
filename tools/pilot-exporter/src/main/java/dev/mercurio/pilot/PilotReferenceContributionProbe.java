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
public final class PilotReferenceContributionProbe {
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
    if(member instanceof MethodTree m && m.getBody()!=null && (cls.equals("TypeUtil")&&m.getName().contentEquals("insertImplicitSpecializations") || cls.equals("TypeAdapter")&&m.getName().contentEquals("removeUnnecessaryImplicitGeneralTypes"))) methods.put(trees.getElement(TreePath.getPath(u,c))+"#"+m.getName(),PilotTransitionSourceProbe.resolved(u,m.getBody(),trees));
    if(member instanceof MethodTree m && m.getBody()!=null && (cls.equals("ReferenceUsageAdapter")&&List.of("addRedefinitions","addDefaultGeneralType").contains(m.getName().toString()) || cls.equals("UsageUtil")&&m.getName().contentEquals("getTransitionLinkFeatureOf") || cls.equals("FeatureAdapter")&&List.of("getRedefinedFeaturesWithComputed","isComputeRedefinitions").contains(m.getName().toString())))
     methods.put(trees.getElement(TreePath.getPath(u,c))+"#"+m.getName(),PilotTransitionSourceProbe.resolved(u,m.getBody(),trees));
    if(cls.equals("ReferenceUsageAdapter")&&member instanceof VariableTree v&&v.getName().contentEquals("TRANSITION_LINK_FEATURE"))constants.put(v.getName().toString(),PilotTransitionSourceProbe.resolved(u,v.getInitializer(),trees));
   }
  }
  require(methods.size()==7&&constants.size()==1,"Incomplete resolved reference programs");
  var injector=new org.omg.kerml.xtext.KerMLStandaloneSetup().createInjectorAndDoEMFRegistration();var f=SysMLFactory.eINSTANCE;var controls=new ArrayList<Object>();
  String source="standard library package Base { classifier Anything; feature things; } standard library package TransitionPerformances { class TransitionPerformance { feature transitionLink; } } package P { feature other; }";
  for(String membership:List.of("FeatureMembership","EndFeatureMembership"))for(String preceding:List.of("none","parameter"))for(String explicit:List.of("none","role","other")) {
   var set=injector.getInstance(XtextResourceSet.class);var resource=(XtextResource)set.createResource(URI.createURI("memory:/reference-link.kerml"));resource.load(new ByteArrayInputStream(source.getBytes(StandardCharsets.UTF_8)),Map.of());
   var names=new HashMap<String,Element>();var qualified=new HashMap<String,Element>();var it=resource.getAllContents();while(it.hasNext())if(it.next() instanceof Element e && e.getDeclaredName()!=null){names.put(e.getDeclaredName(),e);qualified.put(e.getQualifiedName(),e);}
   SysMLLibraryUtil.setProviderLookup(r->(c,n)->qualified.get(n));
   var transition=f.createTransitionUsage();transition.setDeclaredName("transition");resource.getContents().add(transition);
   if(!preceding.equals("none")){var p=f.createReferenceUsage();p.setDeclaredName("previous");PilotTransitionSourceProbe.own(transition,p,preceding.equals("parameter")?"ParameterMembership":"FeatureMembership");}
   var target=f.createReferenceUsage();target.setDeclaredName("candidate");PilotTransitionSourceProbe.own(transition,target,membership);
   if(!explicit.equals("none")){var r=f.createRedefinition();r.setRedefiningFeature(target);r.setRedefinedFeature((Feature)(explicit.equals("self")?target:names.get(explicit.equals("role")?"transitionLink":"other")));target.getOwnedRelationship().add(r);}
   var adapter=(org.omg.sysml.adapter.FeatureAdapter)ElementUtil.getElementAdapter(target);
   var values=adapter.getRedefinedFeaturesWithComputed().stream().map(Element::getDeclaredName).toList();var replay=adapter.getRedefinedFeaturesWithComputed().stream().map(Element::getDeclaredName).toList();require(values.equals(replay),"Unstable reference query");
   adapter.removeUnnecessaryImplicitGeneralTypes();TypeUtil.insertImplicitSpecializations(target);
   var stored=target.getOwnedSpecialization().stream().map(r->Map.of("kind",r.eClass().getName(),"target",r.getGeneral().getDeclaredName(),"specific",r.getSpecific().getDeclaredName(),"implied",r.isImplied())).toList();
   var after=adapter.getRedefinedFeaturesWithComputed().stream().map(Element::getDeclaredName).toList();
   adapter.removeUnnecessaryImplicitGeneralTypes();TypeUtil.insertImplicitSpecializations(target);
   var again=target.getOwnedSpecialization().stream().map(r->Map.of("kind",r.eClass().getName(),"target",r.getGeneral().getDeclaredName(),"specific",r.getSpecific().getDeclaredName(),"implied",r.isImplied())).toList();require(stored.equals(again),"Non-idempotent physical contribution");
   controls.add(Map.of("source",source,"membership",membership,"preceding",preceding,"explicit",explicit,"selected",UsageUtil.getTransitionLinkFeatureOf(transition)==target,"redefinitions",values,"stored",stored,"after",after,"complete",target.isImpliedIncluded()));resource.unload();
  }
  Files.writeString(Path.of(args[1]),new GsonBuilder().serializeNulls().setPrettyPrinting().create().toJson(Map.of("methods",methods,"constants",constants,"controls",controls))+"\n");
 }
}
