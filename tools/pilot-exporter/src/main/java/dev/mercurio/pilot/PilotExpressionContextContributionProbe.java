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
public final class PilotExpressionContextContributionProbe {
 public static void main(String[] args)throws Exception {
  org.omg.sysml.logic.SysMLLogicStandaloneSetup.doSetup();
  var definitions=new TreeMap<String,Object>();
  var compiler=ToolProvider.getSystemJavaCompiler();var diagnostics=new DiagnosticCollector<JavaFileObject>();
  var selected=Map.of("TypeAdapter",List.of("addImplicitGeneralType","addDefaultGeneralType","isImplicitSpecializationFor","areEquivalentImplicitGeneralTypes"),"OperatorExpressionAdapter",List.of("computeImplicitGeneralTypes"),"TriggerInvocationExpressionAdapter",List.of("computeImplicitGeneralTypes"));
  try(var files=compiler.getStandardFileManager(diagnostics,null,java.nio.charset.StandardCharsets.UTF_8)) {
   var task=(JavacTask)compiler.getTask(null,files,diagnostics,List.of("-proc:none","-classpath",args[0]),null,files.getJavaFileObjectsFromStrings(Arrays.asList(args).subList(2,args.length)));
   var units=new ArrayList<CompilationUnitTree>();task.parse().forEach(units::add);task.analyze();var trees=Trees.instance(task);
   require(diagnostics.getDiagnostics().stream().noneMatch(d->d.getKind()==Diagnostic.Kind.ERROR),"Unresolved library definitions: "+diagnostics.getDiagnostics());
   for(var unit:units)for(var declaration:unit.getTypeDecls())if(declaration instanceof ClassTree cls)for(var member:cls.getMembers())if(member instanceof MethodTree m && m.getBody()!=null && selected.getOrDefault(cls.getSimpleName().toString(),List.of()).contains(m.getName().toString()) && (cls.getSimpleName().toString().equals("TypeAdapter")?m.getParameters().size()==2:m.getParameters().isEmpty()))definitions.put(trees.getElement(TreePath.getPath(unit,cls))+"#"+m.getName()+"/"+m.getParameters().size(),PilotResultConstructionExporter.tree(unit,m.getBody(),trees));
  }
  require(definitions.size()==6,"Missing contribution definitions: "+definitions.keySet());
  var f=SysMLFactory.eINSTANCE;var controls=new ArrayList<Object>();var bindings=new TreeMap<String,Object>();
  for(String kind:List.of("BooleanExpression","CollectExpression","ConstructorExpression","Expression","FeatureChainExpression","FeatureReferenceExpression","IndexExpression","Invariant","InvocationExpression","LiteralBoolean","LiteralExpression","LiteralInfinity","LiteralInteger","LiteralRational","LiteralString","MetadataAccessExpression","NullExpression","OperatorExpression","SelectExpression","TriggerInvocationExpression")) {
   var cls=(EClass)SysMLPackage.eINSTANCE.getEClassifier(kind);
   var probe=ElementUtil.getElementAdapter((Element)f.create(cls));bindings.put(kind,probe.getClass().getMethod("computeImplicitGeneralTypes").getDeclaringClass().getName()+"#computeImplicitGeneralTypes/0");
   for(String direction:List.of("null","in","out","inout"))
   for(String context:List.of("detached","package","feature","parameter","result","value"))
   for(boolean composite:List.of(false,true)) {
    String variant=kind.equals("TriggerInvocationExpression")?"when":"default";String value="Function";
    var target=(Expression)f.create(cls);target.setElementId("expression");
    if(target instanceof TriggerInvocationExpression trigger)trigger.setKind(TriggerKind.WHEN);
    target.setDirection(direction.equals("null")?null:FeatureDirectionKind.get(direction));target.setIsComposite(composite);
    if(context.equals("package")){var pkg=f.createPackage();pkg.setElementId("context");var member=f.createOwningMembership();member.setOwnedMemberElement(target);pkg.getOwnedRelationship().add(member);}
    if(List.of("feature","parameter","result").contains(context)){var owner=f.createFunction();owner.setElementId("owner");owner.setIsImpliedIncluded(true);FeatureMembership member=context.equals("parameter")?f.createParameterMembership():context.equals("result")?f.createReturnParameterMembership():f.createFeatureMembership();member.setOwnedMemberFeature(target);owner.getOwnedRelationship().add(member);}
    if(context.equals("value")){var owner=f.createFeature();owner.setElementId("owner");owner.setIsImpliedIncluded(true);var member=f.createFeatureValue();member.setValue(target);owner.getOwnedRelationship().add(member);}
    String selector=target instanceof OperatorExpression op?op.getOperator():target instanceof TriggerInvocationExpression trigger?trigger.getKind().toString():null;
    String[] candidates=selector==null?new String[0]:target instanceof OperatorExpression?ExpressionUtil.getOperatorQualifiedNames(selector):new String[]{ImplicitGeneralizationMap.getDefaultSupertypeFor(target.getClass(),selector)};
    Element lookup=value.equals("absent")?null:value.equals("self")?target:(Element)f.create((EClass)SysMLPackage.eINSTANCE.getEClassifier(value));
    if(lookup!=null && lookup!=target){lookup.setElementId("target");if(lookup instanceof Type type)type.setIsImpliedIncluded(true);}
    var fallback=new TreeMap<String,Feature>();
    SysMLLibraryUtil.setProviderLookup(r->(c,n)->{
     if(Arrays.asList(candidates).contains(n))return lookup;
     return fallback.computeIfAbsent(n,key->{var feature=f.createFeature();feature.setElementId(key);feature.setIsImpliedIncluded(true);return feature;});
    });
    var adapter=(org.omg.sysml.adapter.TypeAdapter)ElementUtil.getElementAdapter(target);var generals=new ArrayList<Object>();
    var row=new TreeMap<String,Object>();row.put("kind",kind);row.put("variant",variant);row.put("selector",selector);row.put("candidates",Arrays.asList(candidates));row.put("value",value);row.put("context",context);row.put("direction",direction);row.put("composite",composite);row.put("is_end",target.isEnd());row.put("is_implied_included",target.isImpliedIncluded());
    try {adapter.computeImplicitGeneralTypes();adapter.forEachImplicitGeneralType((k,g)->generals.add(Map.of("kind",k.getName(),"target",g.getElementId())));row.put("status","ready");}
    catch(ClassCastException e){row.put("status","wrong_return_type");}
    row.put("generals",generals);controls.add(row);
   }
  }
  var equivalence=new ArrayList<Object>();
  var compare=org.omg.sysml.adapter.TypeAdapter.class.getDeclaredMethod("areEquivalentImplicitGeneralTypes",Type.class,Type.class);compare.setAccessible(true);
  for(String shape:List.of("identity","empty","one_same","one_different","two_same","two_reverse","different_length","non_feature")) {
   var a=shape.equals("non_feature")?f.createClass():f.createFeature();
   Type b=shape.equals("identity")?a:shape.equals("non_feature")?f.createClass():f.createFeature();
   var ac=switch(shape){case "one_same","one_different","different_length"->List.of("f0");case "two_same","two_reverse"->List.of("f0","f1");default->List.<String>of();};
   var bc=switch(shape){case "one_same"->List.of("f0");case "one_different"->List.of("f1");case "two_same","different_length"->List.of("f0","f1");case "two_reverse"->List.of("f1","f0");default->List.<String>of();};
   var ends=new TreeMap<String,Feature>();for(String id:List.of("f0","f1")){var feature=f.createFeature();feature.setElementId(id);ends.put(id,feature);}
   for(String id:ac){var chain=f.createFeatureChaining();chain.setChainingFeature(ends.get(id));a.getOwnedRelationship().add(chain);}
   for(String id:bc){var chain=f.createFeatureChaining();chain.setChainingFeature(ends.get(id));b.getOwnedRelationship().add(chain);}
   equivalence.add(Map.of("shape",shape,"first_kind",a.eClass().getName(),"second_kind",b.eClass().getName(),"first_chain",ac,"second_chain",bc,"equivalent",compare.invoke(null,a,b)));
  }
  Files.writeString(Path.of(args[1]),new GsonBuilder().serializeNulls().setPrettyPrinting().create().toJson(Map.of("definitions",definitions,"bindings",bindings,"controls",controls,"equivalence_controls",equivalence))+"\n");
 }
}
