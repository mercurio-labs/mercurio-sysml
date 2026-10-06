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
/** Supplied complete Function ancestry; selector components, not full transformation. */
public final class PilotExpressionParameterProbe {
 public static void main(String[] args)throws Exception {
  org.omg.sysml.logic.SysMLLogicStandaloneSetup.doSetup();SysMLLibraryUtil.setProviderLookup(r->(c,n)->null);
  var methods=new TreeMap<String,Object>();var compiler=ToolProvider.getSystemJavaCompiler();var diagnostics=new DiagnosticCollector<JavaFileObject>();
  try(var files=compiler.getStandardFileManager(diagnostics,null,java.nio.charset.StandardCharsets.UTF_8)) {
   var task=(JavacTask)compiler.getTask(null,files,diagnostics,List.of("-proc:none","-classpath",args[0]),null,files.getJavaFileObjectsFromStrings(Arrays.asList(args).subList(2,args.length)));
   var units=new ArrayList<CompilationUnitTree>();task.parse().forEach(units::add);task.analyze();
   PilotFeatureRedefinitionExporter.require(diagnostics.getDiagnostics().stream().noneMatch(d->d.getKind()==Diagnostic.Kind.ERROR),"Unresolved expression adapter");var trees=Trees.instance(task);
   for(var unit:units)for(var declaration:unit.getTypeDecls())if(declaration instanceof ClassTree cls)for(var member:cls.getMembers())if(member instanceof MethodTree method && method.getName().contentEquals("getGeneralTypes") && cls.getSimpleName().contentEquals("ExpressionAdapter")) methods.put(trees.getElement(TreePath.getPath(unit,cls))+"#"+method.getName(),PilotMixedParameterProbe.node(unit,method.getBody(),trees));
  }
  var bindings=new TreeMap<String,Object>();var controls=new ArrayList<Object>();
  for(var classifier:SysMLPackage.eINSTANCE.getEClassifiers()) {
   if(!(classifier instanceof EClass cls)||cls.isAbstract())continue;var sample=SysMLFactory.eINSTANCE.create(cls);if(!(sample instanceof Expression expression))continue;
   var adapter=ElementUtil.getElementAdapter(expression);var dispatch=new TreeMap<String,String>();
   for(String name:List.of("getRelevantFeatures","getParameterRelevantFeatures","getRelevantParameters","getGeneralTypes","filterIgnoredParameters")) {
    java.lang.Class<?>[] params=name.equals("getGeneralTypes")?new java.lang.Class<?>[]{Type.class,Element.class}:name.equals("filterIgnoredParameters")?new java.lang.Class<?>[]{List.class}:new java.lang.Class<?>[]{Type.class};
    dispatch.put(name,PilotFeatureRedefinitionExporter.key(PilotFeatureRedefinitionExporter.method(adapter.getClass(),name,params)));
   }
   if(!dispatch.get("getRelevantFeatures").equals("org.omg.sysml.adapter.ExpressionAdapter#getRelevantFeatures"))continue;
   bindings.put(cls.getName(),dispatch);
   for(String shape:List.of("direct","short","inherited","diamond","private","protected","multiple","chain_equal","chain_distinct","explicit","no_result","result_first"))for(int position=0;position<4;position++) controls.add(PilotFeatureRedefinitionExporter.parameterControl(cls,shape,position));
  }
  Files.writeString(Path.of(args[1]),new GsonBuilder().serializeNulls().setPrettyPrinting().create().toJson(Map.of("methods",methods,"bindings",bindings,"controls",controls))+"\n");
 }
}
