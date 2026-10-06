package dev.mercurio.pilot;
import com.google.gson.GsonBuilder;
import com.sun.source.tree.*;
import com.sun.source.util.*;
import javax.tools.*;
import java.nio.file.*;
import java.util.*;
import org.omg.sysml.lang.sysml.*;
import org.omg.sysml.util.*;
/** Independent prepared-stage observations. Neither flags nor full lifecycle are qualified. */
public final class PilotTransitionContextStage {
 public static void main(String[] args) throws Exception {
  org.omg.sysml.logic.SysMLLogicStandaloneSetup.doSetup();
  var definitions=new TreeMap<String,Object>();var compiler=ToolProvider.getSystemJavaCompiler();var diagnostics=new DiagnosticCollector<JavaFileObject>();
  try(var files=compiler.getStandardFileManager(diagnostics,null,null)) {
   var task=(JavacTask)compiler.getTask(null,files,diagnostics,List.of("-proc:none","-classpath",args[0]),null,files.getJavaFileObjectsFromStrings(Arrays.asList(args).subList(2,args.length)));
   var units=new ArrayList<CompilationUnitTree>();task.parse().forEach(units::add);task.analyze();
   if(diagnostics.getDiagnostics().stream().anyMatch(d->d.getKind()==Diagnostic.Kind.ERROR))throw new IllegalStateException(diagnostics.getDiagnostics().toString());
   var trees=Trees.instance(task);
   for(var unit:units)for(var decl:unit.getTypeDecls())if(decl instanceof ClassTree cls)
    for(var member:cls.getMembers())if(member instanceof MethodTree m && Set.of("transform","doTransform","addContextFeaturingType","computeFeaturingType","insertImplicitTypeFeaturings").contains(m.getName().toString()))
     definitions.put(unit.getPackageName()+"."+cls.getSimpleName()+"#"+m.getName(),PilotResultConstructionExporter.tree(unit,m.getBody(),trees));
  }
  var controls=new ArrayList<Object>();var f=SysMLFactory.eINSTANCE;
  for(String parentKind:List.of("TransitionUsage","StateUsage","Namespace"))for(String shape:List.of("empty","one","two","duplicate"))for(boolean existing:List.of(false,true)) {
   Namespace parent=parentKind.equals("TransitionUsage")?f.createTransitionUsage():parentKind.equals("StateUsage")?f.createStateUsage():f.createNamespace();parent.setElementId("parent");
   var root=f.createNamespace();var pm=f.createOwningMembership();pm.getOwnedRelatedElement().add(parent);root.getOwnedRelationship().add(pm);
   var a=f.createClass();a.setElementId("a");var b=f.createClass();b.setElementId("b");for(var type:List.of(a,b)){var member=f.createOwningMembership();member.getOwnedRelatedElement().add(type);root.getOwnedRelationship().add(member);}
   if(parent instanceof Feature feature && !shape.equals("empty"))for(var type:shape.equals("one")?List.of(a):shape.equals("two")?List.of(a,b):List.of(a,a)){var tf=f.createTypeFeaturing();tf.setFeatureOfType(feature);tf.setFeaturingType(type);feature.getOwnedRelationship().add(tf);}
   var child=f.createSuccessionAsUsage();child.setElementId("child");var cm=f.createOwningMembership();cm.getOwnedRelatedElement().add(child);parent.getOwnedRelationship().add(cm);
   if(existing){var tf=f.createTypeFeaturing();tf.setFeatureOfType(child);tf.setFeaturingType(a);child.getOwnedRelationship().add(tf);}
   // Supply exactly the upstream in-progress entry guard, not isImpliedIncluded.
   // Parent featuring inputs are explicitly stored and its common owning-Type
   // producer is invoked before requesting the reentrant child context stage.
   var adapter=ElementUtil.getElementAdapter(parent);var guard=PilotFeatureRedefinitionExporter.method(adapter.getClass(),"transform").getDeclaringClass().getDeclaredField("isTransformed");guard.setAccessible(true);guard.setBoolean(adapter,true);
   if(parent instanceof Feature){var compute=PilotFeatureRedefinitionExporter.method(adapter.getClass(),"computeFeaturingType");compute.setAccessible(true);compute.invoke(adapter);}
   var childAdapter=ElementUtil.getElementAdapter(child);var context=PilotFeatureRedefinitionExporter.method(childAdapter.getClass(),"addContextFeaturingType");context.setAccessible(true);context.invoke(childAdapter);
   var before=child.getFeaturingType().stream().map(Type::getElementId).toList();FeatureUtil.insertImplicitTypeFeaturings(child);var after=child.getFeaturingType().stream().map(Type::getElementId).toList();
   if(!before.equals(after)||parent.isImpliedIncluded()||child.isImpliedIncluded())throw new IllegalStateException("Unproved completion or changed projection");
   controls.add(Map.of("parent",parentKind,"shape",shape,"existing",existing,"query",before,"physical",child.getOwnedTypeFeaturing().stream().map(r->Map.of("target",r.getFeaturingType().getElementId(),"implied",r.isImplied())).toList(),"stage_guard_supplied",true,"completion_flags",false));
  }
  Files.writeString(Path.of(args[1]),new GsonBuilder().setPrettyPrinting().create().toJson(Map.of("definitions",definitions,"controls",controls))+"\n");
 }
}
