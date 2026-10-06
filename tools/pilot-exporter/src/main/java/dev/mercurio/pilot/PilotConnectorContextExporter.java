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
/** Resolved Connector lifecycle/context programs and concrete runtime dispatch. */
public final class PilotConnectorContextExporter {
 public static void main(String[] args) throws Exception {
  org.omg.sysml.logic.SysMLLogicStandaloneSetup.doSetup();
  var definitions=new TreeMap<String,Object>();var compiler=ToolProvider.getSystemJavaCompiler();var diagnostics=new DiagnosticCollector<JavaFileObject>();
  try(var files=compiler.getStandardFileManager(diagnostics,null,null)) {
   var sources=Arrays.asList(args).subList(2,args.length);
   var task=(JavacTask)compiler.getTask(null,files,diagnostics,List.of("-proc:none","-classpath",args[0]),null,files.getJavaFileObjectsFromStrings(sources));
   var units=new ArrayList<CompilationUnitTree>();task.parse().forEach(units::add);task.analyze();
   if(diagnostics.getDiagnostics().stream().anyMatch(d->d.getKind()==Diagnostic.Kind.ERROR))throw new IllegalStateException(diagnostics.getDiagnostics().toString());
   var trees=Trees.instance(task);
   for(var unit:units)for(var decl:unit.getTypeDecls())if(decl instanceof ClassTree cls)
    for(var member:cls.getMembers())if(member instanceof MethodTree m && Set.of("doTransform","addContextFeaturingType","addFeaturingTypeIfNecessary","addFeaturingTypes","getContextTypeFor","basicGet").contains(m.getName().toString()))
     definitions.put(unit.getPackageName()+"."+cls.getSimpleName()+"#"+m.getName(),PilotResultConstructionExporter.tree(unit,m.getBody(),trees));
  }
  var controls=new ArrayList<Object>();var bindings=new TreeMap<String,Object>();var f=SysMLFactory.eINSTANCE;
  for(var classifier:SysMLPackage.eINSTANCE.getEClassifiers()) {
   if(!(classifier instanceof EClass cls)||cls.isAbstract()||cls.isInterface()||!SysMLPackage.eINSTANCE.getConnector().isSuperTypeOf(cls))continue;
   var node=(Connector)f.create(cls);var adapter=ElementUtil.getElementAdapter(node);var row=new TreeMap<String,Object>();
   row.put("adapter",adapter.getClass().getName());row.put("transform",PilotFeatureRedefinitionExporter.key(PilotFeatureRedefinitionExporter.method(adapter.getClass(),"doTransform")));
   try {row.put("context",PilotFeatureRedefinitionExporter.key(PilotFeatureRedefinitionExporter.method(adapter.getClass(),"addContextFeaturingType")));}
   catch(IllegalStateException missing){row.put("context",null);}
   row.put("ancestry",cls.getEAllSuperTypes().stream().map(EClass::getName).sorted().toList());bindings.put(cls.getName(),row);
   var shapes=new LinkedHashMap<String,List<List<String>>>();
   shapes.put("empty",List.of());shapes.put("single_empty",List.of(List.of()));shapes.put("single",List.of(List.of("a")));
   shapes.put("shared_two",List.of(List.of("a"),List.of("a")));shapes.put("shared_three",List.of(List.of("a"),List.of("a"),List.of("a")));
   shapes.put("disjoint_two",List.of(List.of("a"),List.of("b")));shapes.put("disjoint_three",List.of(List.of("a"),List.of("a"),List.of("b")));
   shapes.put("ordered_two",List.of(List.of("b","a"),List.of("a","b")));
   shapes.put("direct_two",List.of(List.of("a"),List.of("f0")));
   shapes.put("direct_all_three",List.of(List.of("a"),List.of("f0"),List.of("f0")));
   shapes.put("direct_partial_three",List.of(List.of("a"),List.of("f0"),List.of("b")));
   for(var shape:shapes.entrySet()) {
    var connector=(Connector)f.create(cls);connector.setIsImpliedIncluded(true);
    var types=new TreeMap<String,Type>();for(String id:List.of("a","b")){var type=f.createClass();type.setElementId(id);type.setIsImpliedIncluded(true);types.put(id,type);}
    var features=new ArrayList<Feature>();for(int i=0;i<shape.getValue().size();i++){var feature=f.createFeature();feature.setElementId("f"+i);feature.setIsImpliedIncluded(true);features.add(feature);types.put(feature.getElementId(),feature);}
    for(int i=0;i<features.size();i++) {
     var feature=features.get(i);for(var id:shape.getValue().get(i)){var r=f.createTypeFeaturing();r.setFeatureOfType(feature);r.setFeaturingType(types.get(id));feature.getOwnedRelationship().add(r);}
     var end=f.createFeature();end.setIsEnd(true);end.setIsImpliedIncluded(true);var reference=f.createReferenceSubsetting();reference.setReferencingFeature(end);reference.setReferencedFeature(feature);end.getOwnedRelationship().add(reference);var member=f.createFeatureMembership();member.getOwnedRelatedElement().add(end);connector.getOwnedRelationship().add(member);
    }
    var related=connector.getRelatedFeature().stream().map(Feature::getElementId).toList();if(related.size()!=features.size())throw new IllegalStateException("Missing related features "+cls.getName());
    var context=ConnectorUtil.getContextTypeFor(connector);var control=new TreeMap<String,Object>();control.put("kind",cls.getName());control.put("shape",shape.getKey());control.put("featuring",shape.getValue());control.put("related",related);control.put("result",context==null?null:context.getElementId());control.put("complete_inputs_supplied",true);control.put("partial_shortcut_observation",shape.getKey().equals("direct_partial_three"));controls.add(control);
   }

  }
  Files.writeString(Path.of(args[1]),new GsonBuilder().serializeNulls().setPrettyPrinting().create().toJson(Map.of("definitions",definitions,"bindings",bindings,"controls",controls))+"\n");
 }
}
