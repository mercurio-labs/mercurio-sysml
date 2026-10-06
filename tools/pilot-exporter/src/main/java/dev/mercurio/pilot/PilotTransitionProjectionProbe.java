package dev.mercurio.pilot;
import com.google.gson.GsonBuilder;
import com.sun.source.tree.*;
import com.sun.source.util.*;
import javax.tools.*;
import java.nio.file.*;
import java.util.*;
import org.eclipse.emf.ecore.*;
import org.omg.sysml.lang.sysml.*;
import static dev.mercurio.pilot.PilotStepStrategyProbe.*;
/** Ordered enum-dependent transition getters; no adapter completion or validation. */
public final class PilotTransitionProjectionProbe {
 public static void main(String[] args)throws Exception {
  org.omg.sysml.logic.SysMLLogicStandaloneSetup.doSetup();var methods=new TreeMap<String,Object>();
  var compiler=ToolProvider.getSystemJavaCompiler();var diagnostics=new DiagnosticCollector<JavaFileObject>();
  try(var files=compiler.getStandardFileManager(diagnostics,null,java.nio.charset.StandardCharsets.UTF_8)) {
   var task=(JavacTask)compiler.getTask(null,files,diagnostics,List.of("-proc:none","-classpath",args[0]),null,files.getJavaFileObjectsFromStrings(Arrays.asList(args).subList(2,args.length)));
   var units=new ArrayList<CompilationUnitTree>();task.parse().forEach(units::add);task.analyze();var trees=Trees.instance(task);
   require(diagnostics.getDiagnostics().stream().noneMatch(d->d.getKind()==Diagnostic.Kind.ERROR),"Unresolved transition projection algorithms");
   for(var u:units)for(var d:u.getTypeDecls())if(d instanceof ClassTree c)for(var member:c.getMembers())if(member instanceof MethodTree m && m.getBody()!=null) {
    String cls=c.getSimpleName().toString(),name=m.getName().toString();
    if(cls.startsWith("TransitionUsage_")&&name.equals("basicGet") || cls.equals("UsageUtil")&&name.equals("getTransitionFeaturesOf") || cls.equals("NamespaceUtil")&&name.equals("getOwnedMembersOf") || cls.equals("Namespace_ownedMember_SettingDelegate")&&name.equals("basicGet"))
     methods.put(trees.getElement(TreePath.getPath(u,c))+"#"+name,PilotTransitionSourceProbe.resolved(u,m.getBody(),trees));
   }
  }
  require(methods.size()==7,"Missing transition projection algorithms");var f=SysMLFactory.eINSTANCE;var controls=new ArrayList<Object>();var concrete=new ArrayList<String>();
  for(var classifier:SysMLPackage.eINSTANCE.getEClassifiers()) {
   if(!(classifier instanceof EClass cls)||cls.isAbstract()||!(f.create(cls) instanceof Feature))continue;
   concrete.add(cls.getName());
   for(String membership:List.of("TransitionFeatureMembership","FeatureMembership","OwningMembership","Membership"))
    for(String role:List.of("trigger","guard","effect","unset"))for(boolean reverse:List.of(false,true)) {
     var t=f.createTransitionUsage();boolean rejected=false;
     var ids=new ArrayList<String>(List.of("a","b","c"));if(reverse)Collections.reverse(ids);
     for(String id:ids) {
      var feature=(Feature)f.create(cls);feature.setDeclaredName(id);
      var m=(Membership)f.create((EClass)SysMLPackage.eINSTANCE.getEClassifier(membership));
      if(m instanceof TransitionFeatureMembership tm && !role.equals("unset"))tm.setKind(TransitionFeatureKind.get(role));
      try {if(m instanceof OwningMembership om)om.setOwnedMemberElement(feature);else m.setMemberElement(feature);}
      catch(IllegalArgumentException invalid) {require(m instanceof TransitionFeatureMembership && !(feature instanceof Step),"Unexpected setter rejection: "+invalid);rejected=true;break;}
      t.getOwnedRelationship().add(m);
     }
     if(rejected){controls.add(Map.of("kind",cls.getName(),"membership",membership,"role",role,"reverse",reverse,"rejected",true,"result",Map.of()));continue;}
     var result=new TreeMap<String,Object>();
     result.put("trigger_action",t.getTriggerAction().stream().map(Element::getDeclaredName).toList());
     result.put("guard_expression",t.getGuardExpression().stream().map(Element::getDeclaredName).toList());
     result.put("effect_action",t.getEffectAction().stream().map(Element::getDeclaredName).toList());
     result.put("succession",t.getSuccession()==null?List.of():List.of(t.getSuccession().getDeclaredName()));
     controls.add(Map.of("kind",cls.getName(),"membership",membership,"role",role,"reverse",reverse,"rejected",false,"result",result));
    }
  }
  Collections.sort(concrete);
  var output=new TreeMap<String,Object>();output.put("methods",methods);output.put("feature_kinds",concrete);output.put("controls",controls);output.put("fresh_transition_kind",f.createTransitionFeatureMembership().getKind());Files.writeString(Path.of(args[1]),new GsonBuilder().serializeNulls().setPrettyPrinting().create().toJson(output)+"\n");
 }
}
