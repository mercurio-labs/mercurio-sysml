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
/** Common owning-Type producer controls; no complete transformation claim. */
public final class PilotNoOwningTypeFeaturing {
 public static void main(String[] args) throws Exception {
  org.omg.sysml.logic.SysMLLogicStandaloneSetup.doSetup();
  var definitions=new TreeMap<String,Object>();var compiler=ToolProvider.getSystemJavaCompiler();var diagnostics=new DiagnosticCollector<JavaFileObject>();
  try(var files=compiler.getStandardFileManager(diagnostics,null,null)) {
   var task=(JavacTask)compiler.getTask(null,files,diagnostics,List.of("-proc:none","-classpath",args[0]),null,files.getJavaFileObjects(args[2]));
   var units=new ArrayList<CompilationUnitTree>();task.parse().forEach(units::add);task.analyze();
   if(diagnostics.getDiagnostics().stream().anyMatch(d->d.getKind()==Diagnostic.Kind.ERROR))throw new IllegalStateException(diagnostics.getDiagnostics().toString());
   var trees=Trees.instance(task);
   for(var unit:units)for(var decl:unit.getTypeDecls())if(decl instanceof ClassTree cls)
    for(var member:cls.getMembers())if(member instanceof MethodTree m && Set.of("computeFeaturingType","addImplicitFeaturingTypesIfNecessary","addImplicitFeaturingTypes","addOwnedCrossFeatureTypeFeaturing").contains(m.getName().toString()))
     definitions.put(cls.getSimpleName()+"#"+m.getName(),PilotResultConstructionExporter.tree(unit,m.getBody(),trees));
  }
  var f=SysMLFactory.eINSTANCE;var bindings=new TreeMap<String,Object>();var controls=new ArrayList<Object>();
  for(var classifier:SysMLPackage.eINSTANCE.getEClassifiers()) {
   if(!(classifier instanceof EClass cls)||cls.isAbstract()||cls.isInterface()||!SysMLPackage.eINSTANCE.getFeature().isSuperTypeOf(cls))continue;
   var prototype=(Feature)f.create(cls);var method=PilotFeatureRedefinitionExporter.method(ElementUtil.getElementAdapter(prototype).getClass(),"computeFeaturingType");method.setAccessible(true);
   boolean specialized=SysMLPackage.eINSTANCE.getConnector().isSuperTypeOf(cls)||SysMLPackage.eINSTANCE.getExpression().isSuperTypeOf(cls);
   bindings.put(cls.getName(),Map.of("owning_type_provider",PilotFeatureRedefinitionExporter.key(method),"specialized_context",specialized,"ancestry",cls.getEAllSuperTypes().stream().map(EClass::getName).sorted().toList()));
   for(String context:List.of("detached","package","package_feature","library_package","namespace","namespace_feature"))for(boolean variable:List.of(false,true))for(String shape:List.of("empty","explicit","duplicate","first_chain")) {
    var feature=(Feature)f.create(cls);feature.setElementId("source");if(feature instanceof Usage usage)usage.setIsConstant(variable);else feature.setIsVariable(variable);
    if(!context.equals("detached")){Namespace owner=context.startsWith("namespace")?f.createNamespace():context.equals("library_package")?f.createLibraryPackage():f.createPackage();OwningMembership member=context.endsWith("_feature")?f.createFeatureMembership():f.createOwningMembership();member.getOwnedRelatedElement().add(feature);owner.getOwnedRelationship().add(member);}
    var explicit=f.createType();explicit.setElementId("explicit");explicit.setIsImpliedIncluded(true);
    if(!shape.equals("empty"))for(int i=0;i<(shape.equals("duplicate")?2:1);i++){var r=f.createTypeFeaturing();r.setFeatureOfType(feature);r.setFeaturingType(explicit);feature.getOwnedRelationship().add(r);}
    if(shape.equals("first_chain"))for(String id:List.of("first","second")){var endpoint=f.createFeature();endpoint.setElementId(id);endpoint.setIsImpliedIncluded(true);var type=f.createType();type.setElementId(id+".type");var tf=f.createTypeFeaturing();tf.setFeaturingType(type);tf.setFeatureOfType(endpoint);endpoint.getOwnedRelationship().add(tf);var chain=f.createFeatureChaining();chain.setChainingFeature(endpoint);feature.getOwnedRelationship().add(chain);}
    if(feature.getOwningType()!=null)throw new IllegalStateException("Owning type in no-owner control");
    Object contribution=method.invoke(ElementUtil.getElementAdapter(feature));if(contribution!=null)throw new IllegalStateException("Unexpected owning contribution");
    var before=feature.getFeaturingType().stream().map(Type::getElementId).toList();FeatureUtil.insertImplicitTypeFeaturings(feature);var after=feature.getFeaturingType().stream().map(Type::getElementId).toList();
    if(!before.equals(after)||feature.isImpliedIncluded())throw new IllegalStateException("Mutated complete state");
    controls.add(Map.of("kind",cls.getName(),"context",context,"flag_field",feature instanceof Usage?"is_constant":"is_variable","flag_value",variable,"shape",shape,"query",before,"after",after));
   }
  }
  Files.writeString(Path.of(args[1]),new GsonBuilder().setPrettyPrinting().create().toJson(Map.of("definitions",definitions,"bindings",bindings,"controls",controls))+"\n");
 }
}
