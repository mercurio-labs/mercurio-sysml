package dev.mercurio.pilot;
import com.google.gson.GsonBuilder;
import com.sun.source.tree.*;
import com.sun.source.util.*;
import javax.tools.*;
import java.nio.file.*;
import java.nio.charset.StandardCharsets;
import java.util.*;
import org.omg.sysml.lang.sysml.*;
import org.omg.sysml.util.*;
/** Build-time resolved scope dependencies and bounded completed-fixture controls. */
public final class PilotRelativeNamespaceProbe {
 static void completed(Element e)throws Exception {var a=ElementUtil.getElementAdapter(e);var g=org.omg.sysml.adapter.ElementAdapter.class.getDeclaredField("isTransformed");g.setAccessible(true);g.setBoolean(a,true);}
 public static void main(String[] args)throws Exception {
  org.omg.sysml.logic.SysMLLogicStandaloneSetup.doSetup();var methods=new TreeMap<String,Object>();
  var selected=Map.of("NamespaceUtil",List.of("getRelativeNamespaceFor","getResultNamespaceFor"),"ExpressionUtil",List.of("getArgumentOf","addArguments"),"FeatureUtil",List.of("getValuationFor","getValueExpressionFor","isInputParameter"),"TypeUtil",List.of("getInputOf"),"AssignmentActionUsage_targetArgument_SettingDelegate",List.of("basicGet"));
  var compiler=ToolProvider.getSystemJavaCompiler();var diagnostics=new DiagnosticCollector<JavaFileObject>();
  try(var files=compiler.getStandardFileManager(diagnostics,null,StandardCharsets.UTF_8)) {
   var task=(JavacTask)compiler.getTask(null,files,diagnostics,List.of("-proc:none","-classpath",args[0]),null,files.getJavaFileObjectsFromStrings(Arrays.asList(args).subList(2,args.length)));
   var units=new ArrayList<CompilationUnitTree>();task.parse().forEach(units::add);task.analyze();var trees=Trees.instance(task);
   if(diagnostics.getDiagnostics().stream().anyMatch(d->d.getKind()==Diagnostic.Kind.ERROR))throw new IllegalStateException(diagnostics.getDiagnostics().toString());
   for(var u:units)for(var d:u.getTypeDecls())if(d instanceof ClassTree c)for(var member:c.getMembers())if(member instanceof MethodTree m&&m.getBody()!=null&&selected.getOrDefault(c.getSimpleName().toString(),List.of()).contains(m.getName().toString())) methods.put(trees.getElement(TreePath.getPath(u,c))+"#"+m.getName(),PilotOwnerTypingProbe.resolved(u,m.getBody(),trees));
  }
  if(methods.size()!=9)throw new IllegalStateException("Missing resolved methods: "+methods.keySet());
  var f=SysMLFactory.eINSTANCE;var controls=new ArrayList<Object>();
  for(String kind:List.of("FeatureChainExpression","AssignmentActionUsage"))for(String shape:List.of("normal","reverse_features","reverse_inputs","unmapped_first","missing_first_value","missing_first_result","two_values","two_redefinitions","empty"))for(String direction:List.of("none","in","out","inout")) {
   var function=f.createFunction();function.setElementId("function");function.setIsImpliedIncluded(true);
   var inputs=new ArrayList<Feature>();for(int i=0;i<2;i++){var in=f.createFeature();in.setElementId("input"+i);in.setIsImpliedIncluded(true);if(!direction.equals("none"))in.setDirection(FeatureDirectionKind.get(direction));inputs.add(in);}
   if(shape.equals("reverse_inputs"))Collections.reverse(inputs);for(var in:inputs)TypeUtil.addOwnedFeatureTo(function,in);
   SysMLLibraryUtil.setProviderLookup(r->(c,n)->function);
   Type ns=kind.equals("FeatureChainExpression")?f.createFeatureChainExpression():f.createAssignmentActionUsage();ns.setElementId("namespace");ns.setIsImpliedIncluded(true);completed(ns);
   if(ns instanceof FeatureChainExpression){var typing=f.createFeatureTyping();typing.setTypedFeature((Feature)ns);typing.setType(function);ns.getOwnedRelationship().add(typing);}
   var parameters=new ArrayList<Feature>();
   for(int i=0;i<2&&!shape.equals("empty");i++) {
    var parameter=f.createFeature();parameter.setElementId("parameter"+i);parameter.setIsImpliedIncluded(false);parameter.setDirection(FeatureDirectionKind.IN);
    if(!(shape.equals("unmapped_first")&&i==0)){var r=f.createRedefinition();r.setRedefiningFeature(parameter);r.setRedefinedFeature(inputs.get(shape.equals("reverse_inputs")?1-i:i));parameter.getOwnedRelationship().add(r);}
    if(shape.equals("two_redefinitions")&&i==0){var r=f.createRedefinition();r.setRedefiningFeature(parameter);r.setRedefinedFeature(inputs.get(1));parameter.getOwnedRelationship().add(r);}
    for(int v=0;v<(shape.equals("two_values")&&i==0?2:1);v++) {
     if(shape.equals("missing_first_value")&&i==0)continue;
     var expression=f.createFeatureReferenceExpression();expression.setElementId("value"+i+"."+v);expression.setIsImpliedIncluded(true);completed(expression);
     if(!(shape.equals("missing_first_result")&&i==0)){var result=f.createFeature();result.setElementId("result"+i+"."+v);result.setIsImpliedIncluded(true);result.setDirection(FeatureDirectionKind.OUT);var membership=f.createReturnParameterMembership();membership.setOwnedMemberFeature(result);expression.getOwnedRelationship().add(membership);}
     FeatureUtil.addFeatureValueTo(parameter,expression);
    }
    parameters.add(parameter);
   }
   if(shape.equals("reverse_features"))Collections.reverse(parameters);for(var p:parameters)TypeUtil.addOwnedFeatureTo(ns,p);
   var arguments=new ArrayList<String>();if(ns instanceof FeatureChainExpression chain)for(var e:chain.getArgument())arguments.add(e.getElementId());else if(((AssignmentActionUsage)ns).getTargetArgument()!=null)arguments.add(((AssignmentActionUsage)ns).getTargetArgument().getElementId());
   var relative=NamespaceUtil.getRelativeNamespaceFor(ns);boolean generated=relative!=null&&!relative.getElementId().startsWith("result");if(generated)relative.setElementId("generated."+relative.getOwningNamespace().getElementId());var row=new TreeMap<String,Object>();row.put("result_generated",generated);row.put("parameter_is_implied_included",false);row.put("namespace_is_implied_included",true);row.put("has_callable_typing",ns instanceof FeatureChainExpression);row.put("kind",kind);row.put("shape",shape);row.put("direction",direction);row.put("arguments",arguments);row.put("relative",relative==null?null:relative.getElementId());controls.add(row);
  }
  Files.writeString(Path.of(args[1]),new GsonBuilder().serializeNulls().setPrettyPrinting().create().toJson(Map.of("methods",methods,"controls",controls))+"\n");
 }
}
