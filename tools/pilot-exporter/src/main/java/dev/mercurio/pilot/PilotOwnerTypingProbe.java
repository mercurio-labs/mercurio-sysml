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
/** Resolved default-producer footprint and independent fresh receiver typing controls. */
public final class PilotOwnerTypingProbe {
 static final Map<String,String> captures=new LinkedHashMap<>();
 static String stableType(String type){return java.util.regex.Pattern.compile("capture#[0-9]+").matcher(type).replaceAll(m->captures.computeIfAbsent(m.group(),k->"capture#"+captures.size()));}
 static Map<String,Object> resolved(CompilationUnitTree unit,Tree tree,Trees trees) {
  var row=new TreeMap<String,Object>();row.put("kind",tree.getKind().name());row.put("source",tree.toString());
  var path=TreePath.getPath(unit,tree);var type=trees.getTypeMirror(path);if(type!=null)row.put("type",stableType(type.toString()));
  var symbol=trees.getElement(path);if(symbol!=null){row.put("symbol",symbol.toString());row.put("declaring",symbol.getEnclosingElement().toString());}
  var children=new ArrayList<Object>();tree.accept(new TreeScanner<Void,List<Object>>() {
   @Override public Void scan(Tree child,List<Object> list){if(child!=null)list.add(resolved(unit,child,trees));return null;}
  },children);row.put("children",children);return row;
 }
 public static void main(String[] args)throws Exception {
  org.omg.sysml.logic.SysMLLogicStandaloneSetup.doSetup();
  var selected=Map.of("TypeAdapter",List.of("computeImplicitGeneralTypes","addDefaultGeneralType","getImplicitGeneralTypes"),"FeatureAdapter",List.of("computeImplicitGeneralTypes","addDefaultGeneralType","addComputedRedefinitions","addFeatureWriteTypes","addRedefinitions","addOwnedCrossFeatureSpecialization","getSpecializationEClass"),"UsageAdapter",List.of("addDefaultGeneralType","addVariationTyping"),"OccurrenceUsageAdapter",List.of("addDefaultGeneralType"),"ActionUsageAdapter",List.of("addDefaultGeneralType"),"StateUsageAdapter",List.of("getSubactionType"),"FeatureUtil",List.of("isOwnedCrossFeature","getOwnedCrossFeatureOf"),"TypeUtil",List.of("getImplicitGeneralTypesFor"));
  var methods=new TreeMap<String,Object>();var initialization=new TreeMap<String,Object>();var compiler=ToolProvider.getSystemJavaCompiler();var diagnostics=new DiagnosticCollector<JavaFileObject>();
  try(var files=compiler.getStandardFileManager(diagnostics,null,java.nio.charset.StandardCharsets.UTF_8)) {
   var task=(JavacTask)compiler.getTask(null,files,diagnostics,List.of("-proc:none","-classpath",args[0]),null,files.getJavaFileObjectsFromStrings(Arrays.asList(args).subList(2,args.length)));
   var units=new ArrayList<CompilationUnitTree>();task.parse().forEach(units::add);task.analyze();var trees=Trees.instance(task);
   require(diagnostics.getDiagnostics().stream().noneMatch(d->d.getKind()==Diagnostic.Kind.ERROR),"Unresolved owner typing sources");
   for(var unit:units)for(var declaration:unit.getTypeDecls())if(declaration instanceof ClassTree type)for(var member:type.getMembers())if(member instanceof VariableTree v && type.getSimpleName().contentEquals("OccurrenceUsageImpl") && List.of("PORTION_KIND_EDEFAULT","portionKind").contains(v.getName().toString()))initialization.put(v.getName().toString(),resolved(unit,v.getInitializer(),trees));else if(member instanceof MethodTree m && m.getBody()!=null && selected.getOrDefault(type.getSimpleName().toString(),List.of()).contains(m.getName().toString())) {
    String name=m.getName().toString();int count=m.getParameters().size();
    if(name.equals("addDefaultGeneralType") && count!=0 || name.equals("getImplicitGeneralTypes") && count!=1 || name.equals("getImplicitGeneralTypesFor") && count!=2)continue;
    methods.put(trees.getElement(TreePath.getPath(unit,type))+"#"+name,resolved(unit,m.getBody(),trees));
   }
  }
  require(methods.size()==18,"Incomplete resolved producer footprint: "+methods.keySet());
  var f=SysMLFactory.eINSTANCE;var bindings=new TreeMap<String,Object>();var controls=new ArrayList<Object>();
  for(String kind:List.of("Feature","ReferenceUsage","ActionUsage","StateUsage")) {
   var sample=(Feature)f.create((EClass)SysMLPackage.eINSTANCE.getEClassifier(kind));var adapter=ElementUtil.getElementAdapter(sample);var dispatch=new TreeMap<String,Object>();
   for(String name:List.of("computeImplicitGeneralTypes","addDefaultGeneralType","getSpecializationEClass"))dispatch.put(name,key(method(adapter.getClass(),name)));
   bindings.put(kind,dispatch);
   for(String context:List.of("detached","Namespace","Package","Class","ActionDefinition","StateDefinition"))for(int mask=0;mask<8;mask++)for(boolean composite:List.of(false,true))for(String membership:List.of("FeatureMembership","OwningMembership")) {
    var target=(Feature)f.create((EClass)SysMLPackage.eINSTANCE.getEClassifier(kind));target.setIsComposite(composite);
    if(!context.equals("detached")) {var owner=(Namespace)f.create((EClass)SysMLPackage.eINSTANCE.getEClassifier(context));if(owner instanceof Type t)t.setIsImpliedIncluded(true);var member=membership.equals("FeatureMembership")?f.createFeatureMembership():f.createOwningMembership();member.setOwnedMemberElement(target);owner.getOwnedRelationship().add(member);}
    var explicit=new ArrayList<Object>();
    for(int bit=0;bit<3;bit++)if((mask&(1<<bit))!=0) {var type=(Type)f.create((EClass)SysMLPackage.eINSTANCE.getEClassifier(List.of("DataType","Class","Structure").get(bit)));type.setElementId("type."+bit);type.setIsImpliedIncluded(true);var r=f.createFeatureTyping();r.setTypedFeature(target);r.setType(type);target.getOwnedRelationship().add(r);explicit.add(type.getElementId());}
    SysMLLibraryUtil.setProviderLookup(r->(c,n)->{var e=f.createFeature();e.setElementId(n);e.setIsImpliedIncluded(true);return e;});
    var current=(org.omg.sysml.adapter.TypeAdapter)ElementUtil.getElementAdapter(target);
    var before=new ArrayList<Object>();for(var t:current.getImplicitGeneralTypes(SysMLPackage.Literals.FEATURE_TYPING))before.add(t.getElementId());
    current.computeImplicitGeneralTypes();var after=new ArrayList<Object>();for(var t:current.getImplicitGeneralTypes(SysMLPackage.Literals.FEATURE_TYPING))after.add(t.getElementId());
    controls.add(Map.of("kind",kind,"context",context,"membership",membership,"typing_mask",mask,"composite",composite,"explicit",explicit,"portion_default_is_null",!(target instanceof OccurrenceUsage o) || o.getPortionKind()==null,"implicit_before",before,"implicit_after",after,"is_implied_included",target.isImpliedIncluded()));
   }
  }
  Files.writeString(Path.of(args[1]),new GsonBuilder().setPrettyPrinting().create().toJson(Map.of("methods",methods,"bindings",bindings,"controls",controls,"portion_initialization",initialization,"absent_portion_values",Map.of("ActionUsage",f.createActionUsage().getPortionKind()==null,"StateUsage",f.createStateUsage().getPortionKind()==null,"TransitionUsage",f.createTransitionUsage().getPortionKind()==null)))+"\n");
 }
}
