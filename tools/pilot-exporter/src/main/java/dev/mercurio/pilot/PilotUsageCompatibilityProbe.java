package dev.mercurio.pilot;
import com.google.gson.GsonBuilder;
import com.sun.source.tree.*;
import com.sun.source.util.*;
import javax.tools.*;
import java.nio.file.*;
import java.util.*;
import java.io.ByteArrayInputStream;
import java.nio.charset.StandardCharsets;
import org.eclipse.emf.common.util.URI;
import org.eclipse.xtext.resource.*;
import org.omg.sysml.lang.sysml.*;
import org.omg.sysml.util.*;
/** Build-time variability/compatibility component controls; not complete model qualification. */
public final class PilotUsageCompatibilityProbe {
 public static void main(String[] args)throws Exception {
  SysMLPackage.eINSTANCE.eClass();org.omg.sysml.logic.SysMLLogicStandaloneSetup.doSetup();var methods=new TreeMap<String,Object>();
  var compiler=ToolProvider.getSystemJavaCompiler();var diagnostics=new DiagnosticCollector<JavaFileObject>();
  try(var files=compiler.getStandardFileManager(diagnostics,null,StandardCharsets.UTF_8)) {
   var task=(JavacTask)compiler.getTask(null,files,diagnostics,List.of("-proc:none","-classpath",args[0]),null,files.getJavaFileObjectsFromStrings(Arrays.asList(args).subList(2,args.length)));
   var units=new ArrayList<CompilationUnitTree>();task.parse().forEach(units::add);task.analyze();var trees=Trees.instance(task);
   if(diagnostics.getDiagnostics().stream().anyMatch(d->d.getKind()==Diagnostic.Kind.ERROR))throw new IllegalStateException(diagnostics.getDiagnostics().toString());
   for(var u:units)for(var d:u.getTypeDecls())if(d instanceof ClassTree c)for(var member:c.getMembers())if(member instanceof MethodTree m && m.getBody()!=null) {
    String cls=c.getSimpleName().toString(),name=m.getName().toString();
    if((List.of("FeatureAdapter","TypeAdapter","UsageAdapter").contains(cls)&&name.equals("doTransform")) || cls.equals("FeatureAdapter")&&List.of("computeFeaturingType","computeValueConnector","forceComputeRedefinitions").contains(name) || cls.equals("UsageImpl")&&name.equals("isVariable") || cls.equals("ReferenceUsageAdapter")&&name.equals("addDefaultGeneralType"))
     methods.put(trees.getElement(TreePath.getPath(u,c))+"#"+name,PilotOwnerTypingProbe.resolved(u,m.getBody(),trees));
   }
  }
  if(methods.size()!=8)throw new IllegalStateException("Missing resolved lifecycle methods");
  var f=SysMLFactory.eINSTANCE;var controls=new ArrayList<Object>();var bindings=new TreeMap<String,Object>();
  var kinds=new TreeMap<String,org.eclipse.emf.ecore.EClass>();
  for(var c:SysMLPackage.eINSTANCE.getEClassifiers())if(c instanceof org.eclipse.emf.ecore.EClass k && !k.isAbstract() && !k.isInterface() && SysMLPackage.eINSTANCE.getUsage().isSuperTypeOf(k))kinds.put(k.getName(),k);
  for(var entry:kinds.entrySet()) {
   var probe=(Usage)f.create(entry.getValue());bindings.put(entry.getKey(),probe.getClass().getMethod("isVariable").getDeclaringClass().getName()+"#isVariable");
   for(String placement:List.of("super","first_chain"))for(String mode:List.of("fixed","variable","portion","composite_action")) {
    var nodes=new LinkedHashMap<String,Type>();
    String receiver=placement.equals("super")?"sup":"first";
    for(String id:List.of("sub","sup","base","first"))nodes.put(id,id.equals(receiver)?(Usage)f.create(entry.getValue()):f.createFeature());
    for(String id:List.of("a","c"))nodes.put(id,f.createClass());
    nodes.forEach((id,t)->{t.setElementId(id);t.setIsImpliedIncluded(true);});
    var usage=(Usage)nodes.get(receiver);usage.setIsPortion(mode.equals("portion"));usage.setIsComposite(mode.equals("composite_action"));
    var library=new TreeMap<String,Type>();library.put("Occurrences::Occurrence",f.createClass());library.put("Links::SelfLink",f.createAssociation());library.put("Occurrences::HappensLink",f.createAssociation());library.put("Actions::Action",f.createBehavior());
    library.forEach((name,t)->{t.setElementId(name);t.setIsImpliedIncluded(true);});SysMLLibraryUtil.setProviderLookup(r->(c,n)->library.get(n));
    var edges=new ArrayList<List<String>>();edges.add(List.of("Redefinition","sub","base"));edges.add(List.of("Redefinition","sup","base"));edges.add(List.of("TypeFeaturing","sub","a"));edges.add(List.of("TypeFeaturing","sup","c"));edges.add(List.of("FeatureMembership","a",receiver));
    if(placement.equals("first_chain"))edges.add(List.of("FeatureChaining","sup","first"));
    for(var e:edges) {
     var source=nodes.get(e.get(1));var target=nodes.get(e.get(2));
     switch(e.get(0)) {
      case "FeatureMembership"->TypeUtil.addOwnedFeatureTo(source,(Feature)target);
      case "TypeFeaturing"->{var r=f.createTypeFeaturing();r.setFeatureOfType((Feature)source);r.setFeaturingType(target);source.getOwnedRelationship().add(r);}
      case "FeatureChaining"->{var r=f.createFeatureChaining();r.setChainingFeature((Feature)target);source.getOwnedRelationship().add(r);}
      default->{var r=f.createRedefinition();r.setSpecific(source);r.setGeneral(target);source.getOwnedRelationship().add(r);}
     }
    }
    var libraryEdges=new ArrayList<List<String>>();
    if(!mode.equals("fixed")){var r=f.createSpecialization();r.setSpecific(nodes.get("a"));r.setGeneral(library.get("Occurrences::Occurrence"));nodes.get("a").getOwnedRelationship().add(r);libraryEdges.add(List.of("Specialization","a","Occurrences::Occurrence"));}
    if(mode.equals("composite_action")){var r=f.createFeatureTyping();r.setTypedFeature(usage);r.setType(library.get("Actions::Action"));usage.getOwnedRelationship().add(r);libraryEdges.add(List.of("FeatureTyping",receiver,"Actions::Action"));}
    var nodeKinds=new TreeMap<String,String>();nodes.forEach((id,t)->nodeKinds.put(id,t.eClass().getName()));
    var row=new TreeMap<String,Object>();row.put("kind",entry.getKey());row.put("placement",placement);row.put("shape",mode);row.put("nodes",nodeKinds);row.put("edges",edges);row.put("library_edges",libraryEdges);row.put("receiver",receiver);row.put("variable","");row.put("is_portion",usage.isPortion());row.put("is_composite",usage.isComposite());row.put("may_time_vary",usage.isMayTimeVary());row.put("is_variable",usage.isVariable());row.put("can_access",FeatureUtil.canAccess((Feature)nodes.get("sub"),(Feature)nodes.get("sup")));row.put("compatible",TypeUtil.isCompatible(nodes.get("sub"),nodes.get("sup")));controls.add(row);
   }
  }
  Files.writeString(Path.of(args[1]),new GsonBuilder().serializeNulls().setPrettyPrinting().create().toJson(Map.of("methods",methods,"bindings",bindings,"controls",controls))+"\n");
 }
}
