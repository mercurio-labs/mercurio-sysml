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
public final class PilotFeatureChainMembersProbe {
 static void completed(Element e)throws Exception {var a=ElementUtil.getElementAdapter(e);var g=org.omg.sysml.adapter.ElementAdapter.class.getDeclaredField("isTransformed");g.setAccessible(true);g.setBoolean(a,true);}
 public static void main(String[] args)throws Exception {
  org.omg.sysml.logic.SysMLLogicStandaloneSetup.doSetup();var methods=new TreeMap<String,Object>();
  var selected=Map.of("Type_ownedFeature_SettingDelegate",List.of("basicGet"),"FeatureChainExpressionAdapter",List.of("addAdditionalMembers"),"InvocationExpressionAdapter",List.of("addAdditionalMembers"),"TypeUtil",List.of("addResultParameterTo","getOwnedParameterOf","getOwnedParametersOf","addOwnedFeatureTo"));
  var compiler=ToolProvider.getSystemJavaCompiler();var diagnostics=new DiagnosticCollector<JavaFileObject>();
  try(var files=compiler.getStandardFileManager(diagnostics,null,StandardCharsets.UTF_8)) {
   var task=(JavacTask)compiler.getTask(null,files,diagnostics,List.of("-proc:none","-classpath",args[0]),null,files.getJavaFileObjectsFromStrings(Arrays.asList(args).subList(2,args.length)));
   var units=new ArrayList<CompilationUnitTree>();task.parse().forEach(units::add);task.analyze();var trees=Trees.instance(task);
   if(diagnostics.getDiagnostics().stream().anyMatch(d->d.getKind()==Diagnostic.Kind.ERROR))throw new IllegalStateException(diagnostics.getDiagnostics().toString());
   for(var u:units)for(var d:u.getTypeDecls())if(d instanceof ClassTree c)for(var member:c.getMembers())if(member instanceof MethodTree m&&m.getBody()!=null&&selected.getOrDefault(c.getSimpleName().toString(),List.of()).contains(m.getName().toString())) methods.put(trees.getElement(TreePath.getPath(u,c))+"#"+m.getName()+"/"+m.getParameters().size(),PilotOwnerTypingProbe.resolved(u,m.getBody(),trees));
  }
  if(methods.size()!=8)throw new IllegalStateException("Missing resolved member methods: "+methods.keySet());
  var f=SysMLFactory.eINSTANCE;var controls=new ArrayList<Object>();
  for(String shape:List.of("empty","input","multiple_inputs","nondirected","existing_subfeature","existing_return","empty_return"))for(String direction:List.of("none","in","out","inout"))for(boolean included:List.of(false,true)) {
   var owner=f.createFeatureChainExpression();owner.setIsImpliedIncluded(included);var features=new ArrayList<Feature>();
   if(shape.equals("existing_return")||shape.equals("empty_return")){var member=f.createReturnParameterMembership();if(shape.equals("existing_return")){var result=f.createFeature();result.setDirection(FeatureDirectionKind.OUT);member.setOwnedMemberParameter(result);}owner.getOwnedRelationship().add(member);}
   if(!List.of("empty","existing_return","empty_return").contains(shape))for(int i=0;i<(shape.equals("multiple_inputs")?2:1);i++) {
    var p=f.createFeature();p.setDeclaredName("parameter"+i);if(!direction.equals("none")&&!shape.equals("nondirected"))p.setDirection(FeatureDirectionKind.get(direction));TypeUtil.addOwnedFeatureTo(owner,p);features.add(p);
    if(shape.equals("existing_subfeature")){var child=f.createFeature();child.setDeclaredName("existing");TypeUtil.addOwnedFeatureTo(p,child);}
   }
   var adapter=ElementUtil.getElementAdapter(owner);var before=node(owner);((org.omg.sysml.adapter.FeatureChainExpressionAdapter)adapter).addAdditionalMembers();var after=node(owner);((org.omg.sysml.adapter.FeatureChainExpressionAdapter)adapter).addAdditionalMembers();var replay=node(owner);
   controls.add(Map.of("kind","FeatureChainExpression","shape",shape,"direction",direction,"is_implied_included",included,"before",before,"after",after,"idempotent",after.equals(replay)));
  }
  Files.writeString(Path.of(args[1]),new GsonBuilder().setPrettyPrinting().create().toJson(Map.of("methods",methods,"controls",controls,"bindings",Map.of("FeatureChainExpression",f.createFeatureChainExpression().eClass().getName()+"->"+ElementUtil.getElementAdapter(f.createFeatureChainExpression()).getClass().getMethod("addAdditionalMembers").getDeclaringClass().getName()+"#addAdditionalMembers")))+"\n");
 }
 static Map<String,Object> node(Element e) {
  var row=new TreeMap<String,Object>();row.put("kind",e.eClass().getName());if(e.getDeclaredName()!=null)row.put("name",e.getDeclaredName());
  if(e instanceof Type t)row.put("is_implied_included",t.isImpliedIncluded());if(e instanceof Feature f&&f.getDirection()!=null)row.put("direction",f.getDirection().getLiteral());
  if(e instanceof Membership m)row.put("visibility",m.getVisibility().getLiteral());
  row.put("relationships",e.getOwnedRelationship().stream().map(PilotFeatureChainMembersProbe::node).toList());
  if(e instanceof Relationship r)row.put("elements",r.getOwnedRelatedElement().stream().map(PilotFeatureChainMembersProbe::node).toList());
  return row;
 }
}
