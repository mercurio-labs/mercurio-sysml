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
/** Build-time resolved instantiation dispatch and independent membership-order controls. */
public final class PilotInstantiationStrategyProbe {
 public static void main(String[] args)throws Exception {
  org.omg.sysml.logic.SysMLLogicStandaloneSetup.doSetup();
  var definitions=new TreeMap<String,Object>();
  var compiler=ToolProvider.getSystemJavaCompiler();var diagnostics=new DiagnosticCollector<JavaFileObject>();
  var selected=Map.of("TypeAdapter",List.of("getImplicitGeneralTypeKinds","forEachImplicitGeneralType"),"InvocationExpressionAdapter",List.of("addDefaultGeneralType"),"ExpressionAdapter",List.of("addDefaultGeneralType","getDefaultSupertype"),"InstantiationExpression_instantiatedType_InvocationDelegate",List.of("dynamicInvoke"),"OperatorExpression_instantiatedType_InvocationDelegate",List.of("dynamicInvoke"),"TriggerInvocationExpression_instantiatedType_InvocationDelegate",List.of("dynamicInvoke"),"InstantiationExpression_instantiatedType_SettingDelegate",List.of("basicGet"));
  try(var files=compiler.getStandardFileManager(diagnostics,null,java.nio.charset.StandardCharsets.UTF_8)) {
   var task=(JavacTask)compiler.getTask(null,files,diagnostics,List.of("-proc:none","-classpath",args[0]),null,files.getJavaFileObjectsFromStrings(Arrays.asList(args).subList(2,args.length)));
   var units=new ArrayList<CompilationUnitTree>();task.parse().forEach(units::add);task.analyze();var trees=Trees.instance(task);
   require(diagnostics.getDiagnostics().stream().noneMatch(d->d.getKind()==Diagnostic.Kind.ERROR),"Unresolved instantiation definitions: "+diagnostics.getDiagnostics());
   for(var unit:units)for(var declaration:unit.getTypeDecls())if(declaration instanceof ClassTree cls)for(var member:cls.getMembers())if(member instanceof MethodTree m && m.getBody()!=null && selected.getOrDefault(cls.getSimpleName().toString(),List.of()).contains(m.getName().toString()))definitions.put(trees.getElement(TreePath.getPath(unit,cls))+"#"+m.getName()+"/"+m.getParameters().size(),PilotResultConstructionExporter.tree(unit,m.getBody(),trees));
  }
  require(definitions.size()==9,"Missing resolved methods: "+definitions.keySet());
  var f=SysMLFactory.eINSTANCE;var bindings=new TreeMap<String,Object>();var controls=new ArrayList<Object>();var references=new ArrayList<Object>();
  for(var classifier:SysMLPackage.eINSTANCE.getEClassifiers()) {
   if(!(classifier instanceof EClass cls)||cls.isAbstract()||!SysMLPackage.eINSTANCE.getInstantiationExpression().isSuperTypeOf(cls))continue;
   var sample=(InstantiationExpression)f.create(cls);var adapter=ElementUtil.getElementAdapter(sample);
   var operation=cls.getEAllOperations().stream().filter(o->o.getName().equals("instantiatedType") && o.getEParameters().isEmpty()).findFirst().orElseThrow();
   var selector=((EOperation.Internal)operation).getInvocationDelegate();
   var resolve=selector.getClass().getDeclaredMethod("calculateInvocationDelegateRecursive",EClass.class);resolve.setAccessible(true);
   var delegate=resolve.invoke(selector,cls);String provider=delegate.getClass().getName();
   var names=new TreeMap<String,Object>();for(String role:List.of("base","ownedPerformance","subperformance","enclosedPerformance"))names.put(role,ImplicitGeneralizationMap.getDefaultSupertypeFor(sample.getClass(),role));
   bindings.put(cls.getName(),Map.of("operation_uri",org.eclipse.emf.ecore.util.EcoreUtil.getURI(operation).toString(),"selected_delegate",provider,"setting_delegate","org.omg.sysml.delegate.setting.InstantiationExpression_instantiatedType_SettingDelegate","add_default_general_type",key(method(adapter.getClass(),"addDefaultGeneralType")),"compute_implicit_general_types",key(method(adapter.getClass(),"computeImplicitGeneralTypes")),"role_bindings",names));
   if(!provider.endsWith(".InstantiationExpression_instantiatedType_InvocationDelegate")) {
    references.add(Map.of("kind",cls.getName(),"selected_delegate",provider,"required_dependency",provider.contains("OperatorExpression")?"operator qualified-name conversion and global Function lookup":"TriggerKind role mapping and global Type lookup"));continue;
   }
   for(String shape:List.of("empty","empty_then_class","feature_membership_then_class","class","feature","package_then_class","class_then_feature","feature_then_class","owning_class","empty_only")) {
    var target=(InstantiationExpression)f.create(cls);
    if(shape.equals("empty_then_class") || shape.equals("empty_only")){var member=f.createMembership();target.getOwnedRelationship().add(member);}
    if(shape.startsWith("feature_membership")){var member=f.createFeatureMembership();var feature=f.createFeature();feature.setElementId("ignored");member.setOwnedMemberFeature(feature);target.getOwnedRelationship().add(member);}
    if(shape.startsWith("package")){var member=f.createMembership();var pkg=f.createPackage();pkg.setElementId("package");member.setMemberElement(pkg);target.getOwnedRelationship().add(member);}
    var kinds=switch(shape){case "empty","empty_only" -> List.<String>of();case "feature","feature_then_class" -> shape.equals("feature")?List.of("Feature"):List.of("Feature","Class");case "class_then_feature" -> List.of("Class","Feature");default -> List.of("Class");};
    int i=0;for(String kind:kinds){var value=(Type)f.create((EClass)SysMLPackage.eINSTANCE.getEClassifier(kind));value.setElementId("target"+i++);value.setIsImpliedIncluded(true);if(shape.equals("owning_class")){var member=f.createOwningMembership();member.setOwnedMemberElement(value);target.getOwnedRelationship().add(member);}else{var member=f.createMembership();member.setMemberElement(value);target.getOwnedRelationship().add(member);}}
    Object direct=target.instantiatedType();Object invoked=target.eInvoke(operation,new org.eclipse.emf.common.util.BasicEList<Object>());Object setting=target.getInstantiatedType();require(direct==invoked && direct==setting,"Setting/operation dispatch disagreement");
    SysMLLibraryUtil.setProviderLookup(r->(c,n)->{var feature=f.createFeature();feature.setElementId(n);feature.setIsImpliedIncluded(true);return feature;});
    var current=(org.omg.sysml.adapter.TypeAdapter)ElementUtil.getElementAdapter(target);current.addDefaultGeneralType();var generals=new ArrayList<Object>();current.forEachImplicitGeneralType((k,g)->generals.add(Map.of("kind",k.getName(),"target",g.getElementId())));
    var row=new TreeMap<String,Object>();row.put("kind",cls.getName());row.put("shape",shape);row.put("instantiated_type",direct==null?null:((Type)direct).getElementId());row.put("generals",generals);controls.add(row);
   }
  }
  Files.writeString(Path.of(args[1]),new GsonBuilder().serializeNulls().setPrettyPrinting().create().toJson(Map.of("definitions",definitions,"bindings",bindings,"membership_controls",controls,"reference_only_dependencies",references,"generalization_classifier_ids",Map.of("FeatureTyping",SysMLPackage.eINSTANCE.getFeatureTyping().getClassifierID(),"Subsetting",SysMLPackage.eINSTANCE.getSubsetting().getClassifierID())))+"\n");
 }
}
