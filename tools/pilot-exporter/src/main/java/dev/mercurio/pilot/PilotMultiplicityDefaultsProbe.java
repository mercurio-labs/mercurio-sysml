package dev.mercurio.pilot;
import com.google.gson.GsonBuilder;
import com.sun.source.tree.*;
import com.sun.source.util.*;
import javax.tools.*;
import java.nio.file.*;
import java.util.*;
import org.eclipse.emf.ecore.EClass;
import org.omg.sysml.lang.sysml.*;
import org.omg.sysml.util.*;
public final class PilotMultiplicityDefaultsProbe {
 static java.lang.reflect.Method method(java.lang.Class<?> cls,String name,java.lang.Class<?>...parameters)throws Exception {
  while(cls!=null){try{var m=cls.getDeclaredMethod(name,parameters);m.setAccessible(true);return m;}catch(NoSuchMethodException ignored){cls=cls.getSuperclass();}}
  throw new IllegalStateException(name);
 }
 public static void main(String[]args)throws Exception {
  org.omg.sysml.logic.SysMLLogicStandaloneSetup.doSetup();
  var methods=new TreeMap<String,Object>();var compiler=ToolProvider.getSystemJavaCompiler();var diagnostics=new DiagnosticCollector<JavaFileObject>();
  try(var files=compiler.getStandardFileManager(diagnostics,null,java.nio.charset.StandardCharsets.UTF_8)){
   var task=(JavacTask)compiler.getTask(null,files,diagnostics,List.of("-proc:none","-classpath",args[0]),null,files.getJavaFileObjects(args[2]));
   var units=new ArrayList<CompilationUnitTree>();task.parse().forEach(units::add);task.analyze();
   if(diagnostics.getDiagnostics().stream().anyMatch(d->d.getKind()==Diagnostic.Kind.ERROR))throw new IllegalStateException(diagnostics.getDiagnostics().toString());
   var trees=Trees.instance(task);
   for(var unit:units)for(var decl:unit.getTypeDecls())if(decl instanceof ClassTree cls)for(var member:cls.getMembers())if(member instanceof MethodTree m && Set.of("getDefaultSupertype","getRelevantFeatures").contains(m.getName().toString()))methods.put("org.omg.sysml.adapter.MultiplicityAdapter#"+m.getName(),PilotExpressionRedefinitionProbe.node(unit,m.getBody(),trees));
  }
  var f=SysMLFactory.eINSTANCE;var bindings=new TreeMap<String,Object>();var controls=new ArrayList<Object>();
  for(var classifier:SysMLPackage.eINSTANCE.getEClassifiers()){
   if(!(classifier instanceof EClass cls)||cls.isAbstract()||!SysMLPackage.eINSTANCE.getMultiplicity().isSuperTypeOf(cls))continue;
   var probe=(Multiplicity)f.create(cls);var adapter=ElementUtil.getElementAdapter(probe);var resolved=method(adapter.getClass(),"getDefaultSupertype");
   if(!resolved.getDeclaringClass().getSimpleName().equals("MultiplicityAdapter"))throw new IllegalStateException("Unreviewed provider");
   var names=new TreeMap<String,String>();for(String category:List.of("base","classifier","feature"))names.put(category,ImplicitGeneralizationMap.getDefaultSupertypeFor(probe.getClass(),category));bindings.put(cls.getName(),names);
   for(String context:List.of("detached","Package","Classifier","Class","Feature"))for(String membership:List.of("OwningMembership","FeatureMembership")){
    var target=(Multiplicity)f.create(cls);if(!context.equals("detached")){var owner=(Namespace)f.create((EClass)SysMLPackage.eINSTANCE.getEClassifier(context));var mem=(OwningMembership)f.create((EClass)SysMLPackage.eINSTANCE.getEClassifier(membership));mem.getOwnedRelatedElement().add(target);owner.getOwnedRelationship().add(mem);}
    var a=ElementUtil.getElementAdapter(target);String name=(String)resolved.invoke(a);var relevant=(List<?>)method(a.getClass(),"getRelevantFeatures",Type.class).invoke(a,f.createClassifier());
    controls.add(Map.of("kind",cls.getName(),"context",context,"membership",membership,"default_name",name,"relevant_count",relevant.size(),"canonical_owner",target.getOwner()==null?"detached":target.getOwner().eClass().getName()));
   }
  }
  Files.writeString(Path.of(args[1]),new GsonBuilder().setPrettyPrinting().create().toJson(Map.of("methods",methods,"bindings",bindings,"controls",controls))+"\n");
 }
}
