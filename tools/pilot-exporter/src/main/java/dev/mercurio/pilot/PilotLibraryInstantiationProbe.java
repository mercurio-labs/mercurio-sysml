package dev.mercurio.pilot;
import com.google.gson.GsonBuilder;
import com.sun.source.tree.*;
import com.sun.source.util.*;
import javax.tools.*;
import java.nio.file.*;
import java.util.*;
import org.eclipse.emf.ecore.*;
import org.omg.sysml.lang.sysml.*;
import org.omg.sysml.util.*;
import static dev.mercurio.pilot.PilotStepStrategyProbe.*;
/** Bounded build-time library selection observations, not transformation qualification. */
public final class PilotLibraryInstantiationProbe {
 public static void main(String[] args)throws Exception {
  org.omg.sysml.logic.SysMLLogicStandaloneSetup.doSetup();
  var definitions=new TreeMap<String,Object>();
  var compiler=ToolProvider.getSystemJavaCompiler();var diagnostics=new DiagnosticCollector<JavaFileObject>();
  var selected=Map.of("SysMLLibraryUtil",List.of("getLibraryType"),"ExpressionUtil",List.of("getOperatorQualifiedNames"),"OperatorExpression_instantiatedType_InvocationDelegate",List.of("dynamicInvoke"),"TriggerInvocationExpression_instantiatedType_InvocationDelegate",List.of("dynamicInvoke"));
  try(var files=compiler.getStandardFileManager(diagnostics,null,java.nio.charset.StandardCharsets.UTF_8)) {
   var task=(JavacTask)compiler.getTask(null,files,diagnostics,List.of("-proc:none","-classpath",args[0]),null,files.getJavaFileObjectsFromStrings(Arrays.asList(args).subList(2,args.length)));
   var units=new ArrayList<CompilationUnitTree>();task.parse().forEach(units::add);task.analyze();var trees=Trees.instance(task);
   require(diagnostics.getDiagnostics().stream().noneMatch(d->d.getKind()==Diagnostic.Kind.ERROR),"Unresolved library definitions: "+diagnostics.getDiagnostics());
   for(var unit:units)for(var declaration:unit.getTypeDecls())if(declaration instanceof ClassTree cls)for(var member:cls.getMembers()) {
    String key=trees.getElement(TreePath.getPath(unit,cls)).toString();
    if(member instanceof MethodTree m && m.getBody()!=null && (selected.getOrDefault(cls.getSimpleName().toString(),List.of()).contains(m.getName().toString()) || m.getName().toString().equals("<init>")))definitions.put(key+"#"+m.getName()+"/"+m.getParameters().size(),PilotResultConstructionExporter.tree(unit,m.getBody(),trees));
    if(member instanceof VariableTree v && v.getInitializer()!=null && (v.getName().toString().equals("LIBRARY_PACKAGE_NAMES") || key.endsWith("ExpressionImpl")))definitions.put(key+"#"+v.getName(),PilotResultConstructionExporter.tree(unit,v.getInitializer(),trees));
   }
  }
  var f=SysMLFactory.eINSTANCE;var controls=new ArrayList<Object>();var defaults=new TreeMap<String,Object>();var roles=new TreeMap<String,Object>();
  var kinds=List.of("OperatorExpression","CollectExpression","SelectExpression","IndexExpression","FeatureChainExpression","TriggerInvocationExpression");
  for(String kind:kinds) {
   var cls=(EClass)SysMLPackage.eINSTANCE.getEClassifier(kind);
   var sample=(InstantiationExpression)f.create(cls);
   defaults.put(kind,sample instanceof OperatorExpression op?op.getOperator():(((TriggerInvocationExpression)sample).getKind()==null?null:((TriggerInvocationExpression)sample).getKind().toString()));
   var variants=kind.equals("TriggerInvocationExpression")?Arrays.asList("default","null","when","at","after"):Arrays.asList("default","null","+","a::b","a b");
   for(String variant:variants) {
    var target=(InstantiationExpression)f.create(cls);
    String selector;
    String[] candidates;
    if(target instanceof OperatorExpression op) {
     if(!variant.equals("default"))op.setOperator(variant.equals("null")?null:variant);
     selector=op.getOperator();candidates=selector==null?new String[0]:ExpressionUtil.getOperatorQualifiedNames(selector);
    } else {
     var trigger=(TriggerInvocationExpression)target;
     if(!variant.equals("default"))trigger.setKind(variant.equals("null")?null:TriggerKind.get(variant));
     selector=trigger.getKind()==null?null:trigger.getKind().toString();
     candidates=selector==null?new String[0]:new String[]{ImplicitGeneralizationMap.getDefaultSupertypeFor(target.getClass(),selector)};
     if(selector!=null)roles.put(selector,candidates[0]);
    }
    int count=candidates.length==0?1:(int)Math.pow(4,candidates.length);
    for(int code=0;code<count;code++) {
     int codeValue=code;var values=new ArrayList<String>();var lookup=new TreeMap<String,Element>();
     for(int i=0;i<candidates.length;i++) {
      String value=List.of("absent","Package","Class","Function").get(codeValue%4);codeValue/=4;values.add(value);
      if(!value.equals("absent")){var element=(Element)f.create((EClass)SysMLPackage.eINSTANCE.getEClassifier(value));element.setElementId("target"+i);lookup.put(candidates[i],element);}
     }
     var requested=new ArrayList<String>();SysMLLibraryUtil.setProviderLookup(r->(c,n)->{requested.add(n);return lookup.get(n);});
     var row=new TreeMap<String,Object>();row.put("kind",kind);row.put("variant",variant);row.put("selector",selector);row.put("candidates",Arrays.asList(candidates));row.put("values",values);
     try {var result=target.instantiatedType();row.put("result",result==null?null:result.getElementId());row.put("status","ready");require(target.getInstantiatedType()==result,"setting disagrees");}
     catch(ClassCastException e){row.put("result",null);row.put("status","wrong_return_type");}
     row.put("requested",requested);controls.add(row);
    }
   }
  }
  Files.writeString(Path.of(args[1]),new GsonBuilder().serializeNulls().setPrettyPrinting().create().toJson(Map.of("definitions",definitions,"operator_packages",Arrays.asList(ExpressionUtil.LIBRARY_PACKAGE_NAMES),"defaults",defaults,"trigger_roles",roles,"controls",controls))+"\n");
 }
}
