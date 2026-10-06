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
/** Bounded resolved Occurrence/Item/View default contribution semantics. */
public final class PilotOccurrenceContributionProbe {
 public static void main(String[] args)throws Exception {
  org.omg.sysml.logic.SysMLLogicStandaloneSetup.doSetup();var methods=new TreeMap<String,Object>();var initialization=new TreeMap<String,Object>();
  var selected=Map.of("UsageAdapter",List.of("addDefaultGeneralType"),"OccurrenceUsageAdapter",List.of("addDefaultGeneralType","isSuboccurrence","getDefaultSupertype"),"ItemUsageAdapter",List.of("getDefaultSupertype","isSuboccurrence","isSubobject","isSubitem"),"ViewUsageAdapter",List.of("getDefaultSupertype","isSubview","addDefaultGeneralType"),"MetadataUsageAdapter",List.of("getDefaultSupertype"),"FeatureAdapter",List.of("isSuboccurrence","isSubobject","addDefaultGeneralType"));
  var compiler=ToolProvider.getSystemJavaCompiler();var diagnostics=new DiagnosticCollector<JavaFileObject>();
  try(var files=compiler.getStandardFileManager(diagnostics,null,java.nio.charset.StandardCharsets.UTF_8)) {
   var task=(JavacTask)compiler.getTask(null,files,diagnostics,List.of("-proc:none","-classpath",args[0]),null,files.getJavaFileObjectsFromStrings(Arrays.asList(args).subList(2,args.length)));
   var units=new ArrayList<CompilationUnitTree>();task.parse().forEach(units::add);task.analyze();var trees=Trees.instance(task);
   require(diagnostics.getDiagnostics().stream().noneMatch(d->d.getKind()==Diagnostic.Kind.ERROR),"Unresolved contribution sources");
   for(var unit:units)for(var declaration:unit.getTypeDecls())if(declaration instanceof ClassTree type && type.getSimpleName().contentEquals("OccurrenceUsageImpl"))for(var member:type.getMembers()) {
    if(member instanceof VariableTree v && List.of("PORTION_KIND_EDEFAULT","portionKind").contains(v.getName().toString())) {
     var field=trees.getElement(TreePath.getPath(unit,v));var row=new TreeMap<String,Object>();row.put("symbol",field.getEnclosingElement()+"#"+field.getSimpleName());row.put("type",field.asType().toString());row.put("initializer",node(unit,v.getInitializer(),trees));
     if(v.getInitializer() instanceof IdentifierTree){var value=trees.getElement(TreePath.getPath(unit,v.getInitializer()));row.put("initializer_symbol",value.getEnclosingElement()+"#"+value.getSimpleName());}
     initialization.put(v.getName().toString(),row);
    }
    if(member instanceof MethodTree m && (m.getReturnType()==null || m.getName().contentEquals("getPortionKind")))initialization.put(m.getReturnType()==null?"constructor":"getter",node(unit,m.getBody(),trees));
   }
   for(var unit:units)for(var declaration:unit.getTypeDecls())if(declaration instanceof ClassTree type)for(var member:type.getMembers())if(member instanceof MethodTree m && m.getBody()!=null && m.getParameters().isEmpty() && selected.getOrDefault(type.getSimpleName().toString(),List.of()).contains(m.getName().toString()))methods.put(trees.getElement(TreePath.getPath(unit,type))+"#"+m.getName(),node(unit,m.getBody(),trees));
  }
  require(methods.size()==15,"Missing contribution algorithms");var f=SysMLFactory.eINSTANCE;var bindings=new TreeMap<String,Object>();var excluded=new TreeMap<String,Object>();var absent=new TreeMap<String,Object>();var controls=new ArrayList<Object>();
  var contexts=List.of("detached","Package","Class","Structure","OccurrenceDefinition","OccurrenceUsage","ItemDefinition","ItemUsage","ViewDefinition","ViewUsage","Feature_class","Feature_structure");
  for(var classifier:SysMLPackage.eINSTANCE.getEClassifiers()) {
   if(!(classifier instanceof EClass cls)||cls.isAbstract()||!(f.create(cls) instanceof Usage sample))continue;
   var adapter=ElementUtil.getElementAdapter(sample);var dispatch=new TreeMap<String,Object>();
   for(String name:List.of("getDefaultSupertype","addDefaultGeneralType","isSuboccurrence","isSubobject","addAdditionalMembers"))dispatch.put(name,key(method(adapter.getClass(),name)));
   String selector=(String)dispatch.get("getDefaultSupertype");
   if(!List.of("OccurrenceUsageAdapter","ItemUsageAdapter","ViewUsageAdapter","MetadataUsageAdapter").stream().anyMatch(n->selector.equals("org.omg.sysml.adapter."+n+"#getDefaultSupertype"))) {excluded.put(cls.getName(),dispatch);continue;}
   var names=new TreeMap<String,Object>();for(String role:List.of("base","subitem","subview","dataValue","object","subobject","suboccurrence","snapshot","timeslice","subpart"))names.put(role,ImplicitGeneralizationMap.getDefaultSupertypeFor(sample.getClass(),role));
   bindings.put(cls.getName(),Map.of("dispatch",dispatch,"names",names));absent.put(cls.getName(),((OccurrenceUsage)sample).getPortionKind());
   for(String context:contexts)for(boolean composite:List.of(false,true))for(int mask=0;mask<8;mask++)for(String portion:List.of("none","snapshot","timeslice")) {
    var target=(OccurrenceUsage)f.create(cls);target.setIsComposite(composite);target.setPortionKind(portion.equals("none")?null:PortionKind.get(portion));
    if(!context.equals("detached")) {
     var owner=(Namespace)f.create((EClass)SysMLPackage.eINSTANCE.getEClassifier(context.startsWith("Feature_")?"Feature":context));
     if(owner instanceof Type type)type.setIsImpliedIncluded(true);
     if(context.startsWith("Feature_")){var type=(Type)f.create((EClass)SysMLPackage.eINSTANCE.getEClassifier(context.equals("Feature_class")?"Class":"Structure"));type.setIsImpliedIncluded(true);var r=f.createFeatureTyping();r.setTypedFeature((Feature)owner);r.setType(type);owner.getOwnedRelationship().add(r);}
     var member=f.createFeatureMembership();member.setOwnedMemberFeature(target);owner.getOwnedRelationship().add(member);
    }
    for(int bit=0;bit<3;bit++)if((mask&(1<<bit))!=0) {var type=(Type)f.create((EClass)SysMLPackage.eINSTANCE.getEClassifier(List.of("DataType","Class","Structure").get(bit)));type.setIsImpliedIncluded(true);var r=f.createFeatureTyping();r.setTypedFeature(target);r.setType(type);target.getOwnedRelationship().add(r);}
    var library=new HashMap<String,Element>();SysMLLibraryUtil.setProviderLookup(r->(c,n)->library.computeIfAbsent(n,k->{var e=f.createFeature();e.setElementId(k);e.setIsImpliedIncluded(true);return e;}));
    var current=(org.omg.sysml.adapter.TypeAdapter)ElementUtil.getElementAdapter(target);current.addDefaultGeneralType();var generals=new ArrayList<Object>();current.forEachImplicitGeneralType((k,g)->generals.add(Map.of("kind",k.getName(),"target",g.getElementId())));
    var row=new TreeMap<String,Object>();row.put("kind",cls.getName());row.put("context",context);row.put("composite",composite);row.put("typing_mask",mask);row.put("portion",portion);row.put("generals",generals);controls.add(row);
   }
  }
  Files.writeString(Path.of(args[1]),new GsonBuilder().serializeNulls().setPrettyPrinting().create().toJson(Map.of("methods",methods,"bindings",bindings,"excluded_bindings",excluded,"controls",controls,"portion_initialization",initialization,"absent_portion_values",absent))+"\n");
 }
}
