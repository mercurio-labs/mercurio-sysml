package dev.mercurio.pilot;
import com.google.gson.GsonBuilder;
import com.sun.source.tree.*;
import com.sun.source.util.*;
import javax.tools.*;
import javax.lang.model.element.ExecutableElement;
import java.nio.file.*;
import java.util.*;
import org.eclipse.emf.ecore.*;
import org.omg.sysml.lang.sysml.*;
import org.omg.sysml.util.*;
import static dev.mercurio.pilot.PilotStepStrategyProbe.*;

/** Source membership stage only. Connector transformation is deliberately not invoked. */
public final class PilotTransitionSourceProbe {
 static Object resolved(CompilationUnitTree unit, Tree root, Trees trees) {
  var roots=new ArrayList<Object>();
  new TreeScanner<Void,List<Object>>() {
   @Override public Void scan(Tree tree,List<Object> parent) {
    if(tree==null)return null;
    var row=new TreeMap<String,Object>();row.put("kind",tree.getKind().name());
    var path=TreePath.getPath(unit,tree);var type=trees.getTypeMirror(path);
    if(type!=null)row.put("type",type.toString());
    var symbol=trees.getElement(path);
    if(symbol instanceof ExecutableElement e)row.put("symbol",e.getEnclosingElement()+"#"+e.getSimpleName());
    if(tree instanceof IdentifierTree t)row.put("name",t.getName().toString());
    if(tree instanceof MemberSelectTree t)row.put("name",t.getIdentifier().toString());
    if(tree instanceof LiteralTree t)row.put("value",t.getValue());
    var children=new ArrayList<Object>();row.put("children",children);parent.add(row);
    return super.scan(tree,children);
   }
  }.scan(root,roots);return roots.get(0);
 }
 static Membership own(Namespace owner,Element child,String kind) {
  var f=SysMLFactory.eINSTANCE;var m=(OwningMembership)f.create((EClass)SysMLPackage.eINSTANCE.getEClassifier(kind));
  m.setOwnedMemberElement(child);owner.getOwnedRelationship().add(m);return m;
 }
 static Feature make(String name) {
  var f=SysMLFactory.eINSTANCE;
  String kind=switch(name){case "directed"->"Feature";case "message","end_flow"->"FlowUsage";case "nonfeature"->"Comment";default->name;};
  var element=f.create((EClass)SysMLPackage.eINSTANCE.getEClassifier(kind));
  if(!(element instanceof Feature feature))return null;
  feature.setDeclaredName("previous");if(name.equals("directed"))feature.setDirection(FeatureDirectionKind.IN);
  if(name.equals("end_flow")){var end=f.createFeature();end.setIsEnd(true);own(feature,end,"FeatureMembership");}
  return feature;
 }
 static Object signature(TransitionUsage t) {
  var rows=new ArrayList<Object>();
  for(var m:t.getOwnedMembership()) {
   var row=new TreeMap<String,Object>();row.put("membership",m.eClass().getName());
   row.put("member",m.getMemberElement()==null?null:m.getMemberElement().getDeclaredName());rows.add(row);
  }
  return rows;
 }
 public static void main(String[] args)throws Exception {
  org.omg.sysml.logic.SysMLLogicStandaloneSetup.doSetup();var methods=new TreeMap<String,Object>();
  var compiler=ToolProvider.getSystemJavaCompiler();var diagnostics=new DiagnosticCollector<JavaFileObject>();
  try(var files=compiler.getStandardFileManager(diagnostics,null,java.nio.charset.StandardCharsets.UTF_8)) {
   var task=(JavacTask)compiler.getTask(null,files,diagnostics,List.of("-proc:none","-classpath",args[0]),null,files.getJavaFileObjectsFromStrings(Arrays.asList(args).subList(2,args.length)));
   var units=new ArrayList<CompilationUnitTree>();task.parse().forEach(units::add);task.analyze();var trees=Trees.instance(task);
   require(diagnostics.getDiagnostics().stream().noneMatch(d->d.getKind()==Diagnostic.Kind.ERROR),"Unresolved transition source algorithms");
   for(var u:units)for(var d:u.getTypeDecls())if(d instanceof ClassTree c)for(var member:c.getMembers())if(member instanceof MethodTree m && m.getBody()!=null) {
    String cls=c.getSimpleName().toString(),name=m.getName().toString();
    if(cls.equals("TransitionUsageAdapter")&&name.equals("computeSource") || cls.equals("UsageUtil")&&List.of("getPreviousFeature","isMessage").contains(name) || cls.equals("FeatureUtil")&&name.equals("isParameter"))
     methods.put(trees.getElement(TreePath.getPath(u,c))+"#"+name,resolved(u,m.getBody(),trees));
   }
  }
  require(methods.size()==4,"Missing source algorithms");var f=SysMLFactory.eINSTANCE;var controls=new ArrayList<Object>();
  for(String ownerKind:List.of("detached","Package","Class","ActionDefinition","StateDefinition"))
   for(String previous:List.of("none","Feature","StateUsage","TransitionUsage","BindingConnector","message","end_flow","directed","nonfeature"))
    for(String first:List.of("none","parameter","empty_alias","explicit_alias")) {
     var target=f.createTransitionUsage();target.setDeclaredName("transition");
     if(!ownerKind.equals("detached")) {
      var owner=(Namespace)f.create((EClass)SysMLPackage.eINSTANCE.getEClassifier(ownerKind));
      if(!previous.equals("none")) {
       var candidate=make(previous);Element e=candidate==null?f.createComment():candidate;
       own(owner,e,candidate==null?"OwningMembership":"FeatureMembership");
      }
      own(owner,target,"FeatureMembership");
     }
     if(first.equals("parameter")){var p=f.createReferenceUsage();p.setDeclaredName("parameter");own(target,p,"ParameterMembership");}
     else if(first.endsWith("alias")){var m=f.createMembership();if(first.equals("explicit_alias")){var e=f.createStateUsage();e.setDeclaredName("explicit");m.setMemberElement(e);}target.getOwnedRelationship().add(m);}
     var action=method(ElementUtil.getElementAdapter(target).getClass(),"computeSource");action.setAccessible(true);
     var before=signature(target);action.invoke(ElementUtil.getElementAdapter(target));var after=signature(target);
     action.invoke(ElementUtil.getElementAdapter(target));var replay=signature(target);
     controls.add(Map.of("owner",ownerKind,"previous",previous,"first",first,"before",before,"after",after,"replay",replay));
    }
  Files.writeString(Path.of(args[1]),new GsonBuilder().serializeNulls().setPrettyPrinting().create().toJson(Map.of("methods",methods,"controls",controls))+"\n");
 }
}
