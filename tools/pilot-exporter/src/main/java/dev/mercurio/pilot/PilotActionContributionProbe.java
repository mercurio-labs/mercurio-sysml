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
public final class PilotActionContributionProbe {
 public static void main(String[] args)throws Exception {
  org.omg.sysml.logic.SysMLLogicStandaloneSetup.doSetup();var methods=new TreeMap<String,Object>();var initialization=new TreeMap<String,Object>();
  var selected=Map.of("UsageAdapter",List.of("addDefaultGeneralType","isActionOwnedComposite","isPartOwnedComposite","isEntryExitAction"),"OccurrenceUsageAdapter",List.of("addDefaultGeneralType","isSuboccurrence"),"ActionUsageAdapter",List.of("addDefaultGeneralType","getDefaultSupertype","isSuboccurrence","getSubactionType"),"TransitionUsageAdapter",List.of("getDefaultSupertype","isActionTransition","isStateTransition"),"FeatureAdapter",List.of("isSuboccurrence","isSubobject","addDefaultGeneralType","isStructureOwnedComposite","isBehaviorOwnedComposite","isBehaviorOwned"),"FeatureUtil",List.of("isPerformanceFeature"));
  var compiler=ToolProvider.getSystemJavaCompiler();var diagnostics=new DiagnosticCollector<JavaFileObject>();
  try(var files=compiler.getStandardFileManager(diagnostics,null,java.nio.charset.StandardCharsets.UTF_8)) {
   var task=(JavacTask)compiler.getTask(null,files,diagnostics,List.of("-proc:none","-classpath",args[0]),null,files.getJavaFileObjectsFromStrings(Arrays.asList(args).subList(2,args.length)));
   var units=new ArrayList<CompilationUnitTree>();task.parse().forEach(units::add);task.analyze();var trees=Trees.instance(task);
   require(diagnostics.getDiagnostics().stream().noneMatch(d->d.getKind()==Diagnostic.Kind.ERROR),"Unresolved contribution sources");
   for(var unit:units)for(var declaration:unit.getTypeDecls())if(declaration instanceof ClassTree type)for(var member:type.getMembers())if(member instanceof MethodTree m && m.getBody()!=null && (m.getParameters().isEmpty() || m.getName().contentEquals("isPerformanceFeature")) && selected.getOrDefault(type.getSimpleName().toString(),List.of()).contains(m.getName().toString()))methods.put(trees.getElement(TreePath.getPath(unit,type))+"#"+m.getName(),node(unit,m.getBody(),trees));
  }
  require(methods.size()==20,"Missing contribution algorithms: "+methods.keySet());var f=SysMLFactory.eINSTANCE;var bindings=new TreeMap<String,Object>();var excluded=new TreeMap<String,Object>();var absent=new TreeMap<String,Object>();var controls=new ArrayList<Object>();
  var contexts=List.of("detached","Package","Class","Structure","Behavior","Step","ActionDefinition","ActionUsage","StateDefinition","StateUsage","PartDefinition","PartUsage","Feature_class","Feature_structure");
  for(var classifier:SysMLPackage.eINSTANCE.getEClassifiers()) {
   if(!(classifier instanceof EClass cls)||!List.of("ActionUsage","TransitionUsage").contains(cls.getName())||cls.isAbstract()||!(f.create(cls) instanceof Usage sample))continue;
   var adapter=ElementUtil.getElementAdapter(sample);var dispatch=new TreeMap<String,Object>();
   for(String name:List.of("getDefaultSupertype","addDefaultGeneralType","isSuboccurrence","isSubobject","addAdditionalMembers","isActionOwnedComposite","isPartOwnedComposite","isStructureOwnedComposite","isBehaviorOwnedComposite","isBehaviorOwned","getSubactionType"))dispatch.put(name,key(method(adapter.getClass(),name)));
   String selector=(String)dispatch.get("getDefaultSupertype");
   var names=new TreeMap<String,Object>();for(String role:List.of("base","subitem","subview","dataValue","object","subobject","suboccurrence","snapshot","timeslice","subaction","ownedAction","ownedPerformance","subperformance","enclosedPerformance","stateTransition","actionTransition"))names.put(role,ImplicitGeneralizationMap.getDefaultSupertypeFor(sample.getClass(),role));
   bindings.put(cls.getName(),Map.of("dispatch",dispatch,"names",names));absent.put(cls.getName(),((OccurrenceUsage)sample).getPortionKind());
   for(String context:contexts)for(boolean composite:List.of(false,true))for(int mask=0;mask<8;mask++)for(String portion:List.of("none","snapshot","timeslice"))for(String sourceKind:cls.getName().equals("TransitionUsage")?List.of("none","Feature","StateUsage"):List.of("none")) {
    var target=(OccurrenceUsage)f.create(cls);target.setIsComposite(composite);target.setPortionKind(portion.equals("none")?null:PortionKind.get(portion));
    if(!context.equals("detached")) {
     var owner=(Namespace)f.create((EClass)SysMLPackage.eINSTANCE.getEClassifier(context.startsWith("Feature_")?"Feature":context));
     if(owner instanceof Type type)type.setIsImpliedIncluded(true);
     if(context.startsWith("Feature_")){var type=(Type)f.create((EClass)SysMLPackage.eINSTANCE.getEClassifier(context.equals("Feature_class")?"Class":"Structure"));type.setIsImpliedIncluded(true);var r=f.createFeatureTyping();r.setTypedFeature((Feature)owner);r.setType(type);owner.getOwnedRelationship().add(r);}
     var member=f.createFeatureMembership();member.setOwnedMemberFeature(target);owner.getOwnedRelationship().add(member);
    }
    for(int bit=0;bit<3;bit++)if((mask&(1<<bit))!=0) {var type=(Type)f.create((EClass)SysMLPackage.eINSTANCE.getEClassifier(List.of("DataType","Class","Structure").get(bit)));type.setIsImpliedIncluded(true);var r=f.createFeatureTyping();r.setTypedFeature(target);r.setType(type);target.getOwnedRelationship().add(r);}
    if(!sourceKind.equals("none")){var source=f.create((EClass)SysMLPackage.eINSTANCE.getEClassifier(sourceKind));((Type)source).setIsImpliedIncluded(true);var alias=f.createMembership();alias.setMemberElement((Feature)source);target.getOwnedRelationship().add(0,alias);}
    var library=new HashMap<String,Element>();SysMLLibraryUtil.setProviderLookup(r->(c,n)->library.computeIfAbsent(n,k->{var e=f.createFeature();e.setElementId(k);e.setIsImpliedIncluded(true);return e;}));
    var current=(org.omg.sysml.adapter.TypeAdapter)ElementUtil.getElementAdapter(target);current.addDefaultGeneralType();var generals=new ArrayList<Object>();current.forEachImplicitGeneralType((k,g)->generals.add(Map.of("kind",k.getName(),"target",g.getElementId())));
    var row=new TreeMap<String,Object>();row.put("kind",cls.getName());row.put("context",context);row.put("composite",composite);row.put("typing_mask",mask);row.put("portion",portion);row.put("source_kind",sourceKind);row.put("generals",generals);controls.add(row);
   }
  }
  var declared=new ArrayList<Object>();
  for(String kind:List.of("ActionUsage","TransitionUsage"))for(String childKind:List.of("Class","DataType","ReferenceUsage","FeatureReferenceExpression")) {
   var target=(Usage)f.create((EClass)SysMLPackage.eINSTANCE.getEClassifier(kind));target.setIsComposite(false);
   var child=f.create((EClass)SysMLPackage.eINSTANCE.getEClassifier(childKind));var membership=f.createOwningMembership();membership.setOwnedMemberElement((Element)child);target.getOwnedRelationship().add(membership);
   SysMLLibraryUtil.setProviderLookup(r->(c,n)->{var e=f.createFeature();e.setElementId(n);e.setIsImpliedIncluded(true);return e;});
   var adapter=(org.omg.sysml.adapter.TypeAdapter)ElementUtil.getElementAdapter(target);adapter.addDefaultGeneralType();var generals=new ArrayList<Object>();adapter.forEachImplicitGeneralType((k,g)->generals.add(Map.of("kind",k.getName(),"target",g.getElementId())));
   declared.add(Map.of("kind",kind,"child_kind",childKind,"generals",generals));
  }
  Files.writeString(Path.of(args[1]),new GsonBuilder().serializeNulls().setPrettyPrinting().create().toJson(Map.of("owned_member_controls",declared,"methods",methods,"bindings",bindings,"excluded_bindings",excluded,"controls",controls,"portion_initialization",initialization,"absent_portion_values",absent))+"\n");
 }
}
