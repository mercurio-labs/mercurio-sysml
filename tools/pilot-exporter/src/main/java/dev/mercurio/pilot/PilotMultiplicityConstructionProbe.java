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

/** Resolved Usage admission and canonical multiplicity constructor, separate from bound semantics. */
public final class PilotMultiplicityConstructionProbe {
 public static void main(String[] args) throws Exception {
  org.omg.sysml.logic.SysMLLogicStandaloneSetup.doSetup();var methods=new TreeMap<String,Object>();
  var compiler=ToolProvider.getSystemJavaCompiler();var diagnostics=new DiagnosticCollector<JavaFileObject>();
  try(var files=compiler.getStandardFileManager(diagnostics,null,java.nio.charset.StandardCharsets.UTF_8)) {
   var task=(JavacTask)compiler.getTask(null,files,diagnostics,List.of("-proc:none","-classpath",args[0]),null,files.getJavaFileObjectsFromStrings(Arrays.asList(args).subList(2,args.length)));
   var units=new ArrayList<CompilationUnitTree>();task.parse().forEach(units::add);task.analyze();var trees=Trees.instance(task);
   require(diagnostics.getDiagnostics().stream().noneMatch(d->d.getKind()==Diagnostic.Kind.ERROR),"Unresolved multiplicity sources");
   for(var unit:units)for(var declaration:unit.getTypeDecls())if(declaration instanceof ClassTree type)for(var member:type.getMembers())if(member instanceof MethodTree m && m.getBody()!=null) {
    String name=m.getName().toString();String cls=type.getSimpleName().toString();
    if(cls.equals("UsageAdapter") && List.of("isAddMultiplicity","isAddDefaultMultiplicity","addAdditionalMembers").contains(name)
      || List.of("ItemUsageAdapter","PortUsageAdapter","AttributeUsageAdapter","ConnectionUsageAdapter").contains(cls) && name.equals("isAddMultiplicity")
      || cls.equals("TypeUtil") && name.equals("addMultiplicityTo") || cls.equals("FeatureUtil") && name.equals("getBasicFeatureOf"))
     methods.put(trees.getElement(TreePath.getPath(unit,type))+"#"+name,node(unit,m.getBody(),trees));
   }
  }
  require(methods.size()==9,"Missing multiplicity algorithms");var f=SysMLFactory.eINSTANCE;var bindings=new TreeMap<String,Object>();var excluded=new TreeMap<String,Object>();var controls=new ArrayList<Object>();
  for(var classifier:SysMLPackage.eINSTANCE.getEClassifiers()) {
   if(!(classifier instanceof EClass cls)||cls.isAbstract()||!(f.create(cls) instanceof Usage))continue;
   var sample=(Usage)f.create(cls);var adapter=ElementUtil.getElementAdapter(sample);
   if(!key(method(adapter.getClass(),"addAdditionalMembers")).equals("org.omg.sysml.adapter.UsageAdapter#addAdditionalMembers")){excluded.put(cls.getName(),key(method(adapter.getClass(),"addAdditionalMembers")));continue;}
   var predicate=method(adapter.getClass(),"isAddMultiplicity");predicate.setAccessible(true);
   String dispatch=key(predicate);require(List.of("UsageAdapter","ItemUsageAdapter","PortUsageAdapter","AttributeUsageAdapter","ConnectionUsageAdapter","ConnectionUsageAdapter").stream().anyMatch(n->dispatch.equals("org.omg.sysml.adapter."+n+"#isAddMultiplicity")),"Unassessed multiplicity predicate");
   bindings.put(cls.getName(),dispatch);
   for(boolean end:List.of(false,true))for(String ownerKind:List.of("detached","Package","Class"))for(String subset:List.of("none","detached","owned","chain_detached","chain_owned"))for(String existing:List.of("none","owned_range","alias")) {
    var target=(Usage)f.create(cls);target.setIsEnd(end);
    if(!ownerKind.equals("detached")) {var owner=(Namespace)f.create((EClass)SysMLPackage.eINSTANCE.getEClassifier(ownerKind));var m=f.createFeatureMembership();m.setOwnedMemberFeature(target);owner.getOwnedRelationship().add(m);}
    if(!subset.equals("none")) {
     var general=f.createFeature();if(subset.startsWith("chain_")) {var basic=f.createFeature();if(subset.equals("chain_owned")){var owner=f.createClass();var m=f.createFeatureMembership();m.setOwnedMemberFeature(basic);owner.getOwnedRelationship().add(m);}var c=f.createFeatureChaining();c.setFeatureChained(general);c.setChainingFeature(basic);general.getOwnedRelationship().add(c);}
     else if(subset.equals("owned")){var owner=f.createClass();var m=f.createFeatureMembership();m.setOwnedMemberFeature(general);owner.getOwnedRelationship().add(m);}
     var relation=f.createSubsetting();relation.setSubsettingFeature(target);relation.setSubsettedFeature(general);target.getOwnedRelationship().add(relation);
    }
    if(existing.equals("owned_range")){var m=f.createOwningMembership();m.setOwnedMemberElement(f.createMultiplicityRange());target.getOwnedRelationship().add(m);}
    else if(existing.equals("alias")){var m=f.createMembership();m.setMemberElement(f.createMultiplicity());target.getOwnedRelationship().add(m);}
    boolean admitted=(Boolean)predicate.invoke(ElementUtil.getElementAdapter(target));
    int before=(int)NamespaceUtil.getOwnedMembersOf(target).filter(Multiplicity.class::isInstance).count();
    if(admitted)TypeUtil.addMultiplicityTo(target);
    int after=(int)NamespaceUtil.getOwnedMembersOf(target).filter(Multiplicity.class::isInstance).count();
    if(admitted)TypeUtil.addMultiplicityTo(target);
    int replay=(int)NamespaceUtil.getOwnedMembersOf(target).filter(Multiplicity.class::isInstance).count();
    controls.add(Map.of("kind",cls.getName(),"is_end",end,"owner",ownerKind,"subset",subset,"existing",existing,"admitted",admitted,"before",before,"after",after,"replay",replay));
   }
  }
  Files.writeString(Path.of(args[1]),new GsonBuilder().serializeNulls().setPrettyPrinting().create().toJson(Map.of("methods",methods,"bindings",bindings,"excluded_bindings",excluded,"controls",controls))+"\n");
 }
}
