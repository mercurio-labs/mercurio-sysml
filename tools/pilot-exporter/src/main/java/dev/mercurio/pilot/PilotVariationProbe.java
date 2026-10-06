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
public final class PilotVariationProbe {
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
    if(member instanceof MethodTree m && m.getBody()!=null && (cls.equals("UsageAdapter")&&m.getName().contentEquals("addVariationTyping") || cls.equals("UsageUtil")&&List.of("isVariant","getOwningVariationDefinitionFor","getOwningVariationUsageFor").contains(m.getName().toString())))
     methods.put(trees.getElement(TreePath.getPath(u,c))+"#"+m.getName(),PilotTransitionSourceProbe.resolved(u,m.getBody(),trees));
   }
  }
  require(methods.size()==4,"Incomplete variation programs");
  var f=SysMLFactory.eINSTANCE;var controls=new ArrayList<Object>();var kinds=new ArrayList<String>();var owners=new ArrayList<String>();
  for(var classifier:SysMLPackage.eINSTANCE.getEClassifiers())if(classifier instanceof org.eclipse.emf.ecore.EClass cls&&!cls.isAbstract()) {
   var object=f.create(cls);if(object instanceof Usage)kinds.add(cls.getName());if(object instanceof Usage||object instanceof Definition)owners.add(cls.getName());
  }
  Collections.sort(kinds);owners.addAll(List.of("Class","Package"));Collections.sort(owners);
  for(String kind:kinds)for(String ownerKind:owners)for(String membership:List.of("VariantMembership","FeatureMembership","OwningMembership"))for(boolean variation:List.of(false,true)) {
   var owner=(Namespace)f.create((org.eclipse.emf.ecore.EClass)SysMLPackage.eINSTANCE.getEClassifier(ownerKind));owner.setDeclaredName("owner");
   if(owner instanceof Usage u)u.setIsVariation(variation);else if(owner instanceof Definition d)d.setIsVariation(variation);
   var root=f.createPackage();root.setDeclaredName("root");PilotTransitionSourceProbe.own(root,owner,"OwningMembership");
   var target=(Usage)f.create((org.eclipse.emf.ecore.EClass)SysMLPackage.eINSTANCE.getEClassifier(kind));target.setDeclaredName("candidate");PilotTransitionSourceProbe.own(owner,target,membership);
   var adapter=ElementUtil.getElementAdapter(target);boolean bound=adapter instanceof org.omg.sysml.adapter.UsageAdapter;java.lang.reflect.Method action=bound?method(adapter.getClass(),"addVariationTyping"):null;if(bound){action.setAccessible(true);action.invoke(adapter);}
   var pending=new ArrayList<Object>();TypeUtil.forEachImplicitGeneralTypeOf(target,(k,g)->pending.add(Map.of("kind",k.getName(),"target",g.getDeclaredName())));
   if(bound)action.invoke(adapter);var repeated=new ArrayList<Object>();TypeUtil.forEachImplicitGeneralTypeOf(target,(k,g)->repeated.add(Map.of("kind",k.getName(),"target",g.getDeclaredName())));require(pending.equals(repeated),"Unstable variation contribution");
   var physical=new ArrayList<Object>();
   boolean observedPhysical=bound&&variation&&membership.equals("VariantMembership")&&List.of("Definition","Usage").contains(ownerKind);
   if(observedPhysical){var typeAdapter=(org.omg.sysml.adapter.TypeAdapter)adapter;typeAdapter.removeUnnecessaryImplicitGeneralTypes();TypeUtil.insertImplicitSpecializations(target);for(var r:target.getOwnedSpecialization())physical.add(Map.of("kind",r.eClass().getName(),"target",r.getGeneral().getDeclaredName(),"specific",r.getSpecific().getDeclaredName(),"implied",r.isImplied(),"owned_related",r.getOwnedRelatedElement().size()));}
   var row=new TreeMap<String,Object>(Map.of("kind",kind,"owner",ownerKind,"membership",membership,"variation",variation,"pending",pending,"bound",bound,"adapter",adapter.getClass().getName()));row.put("physical_observed",observedPhysical);row.put("physical",physical);controls.add(row);
  }
  Files.writeString(Path.of(args[1]),new GsonBuilder().serializeNulls().setPrettyPrinting().create().toJson(Map.of("methods",methods,"usage_kinds",kinds,"owner_kinds",owners,"controls",controls))+"\n");
 }
}
