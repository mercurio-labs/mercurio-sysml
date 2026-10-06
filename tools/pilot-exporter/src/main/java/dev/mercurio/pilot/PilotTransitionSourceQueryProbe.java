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
public final class PilotTransitionSourceQueryProbe {
 public static void main(String[] args)throws Exception {
  org.omg.sysml.logic.SysMLLogicStandaloneSetup.doSetup();var methods=new TreeMap<String,Object>();
  var compiler=ToolProvider.getSystemJavaCompiler();var diagnostics=new DiagnosticCollector<JavaFileObject>();
  try(var files=compiler.getStandardFileManager(diagnostics,null,java.nio.charset.StandardCharsets.UTF_8)) {
   var task=(JavacTask)compiler.getTask(null,files,diagnostics,List.of("-proc:none","-classpath",args[0]),null,files.getJavaFileObjectsFromStrings(Arrays.asList(args).subList(2,args.length)));
   var units=new ArrayList<CompilationUnitTree>();task.parse().forEach(units::add);task.analyze();var trees=Trees.instance(task);
   require(diagnostics.getDiagnostics().stream().noneMatch(d->d.getKind()==Diagnostic.Kind.ERROR),"Unresolved source query algorithms");
   for(var u:units)for(var d:u.getTypeDecls())if(d instanceof ClassTree c)for(var member:c.getMembers())if(member instanceof MethodTree m && m.getBody()!=null) {
    String cls=c.getSimpleName().toString(),name=m.getName().toString();
    if(cls.equals("TransitionUsageAdapter")&&List.of("computeSource","computeTransitionLinkConnectors","addAdditionalMembers").contains(name) || cls.equals("UsageUtil")&&List.of("getSourceFeatureOf","getTransitionLinkFeatureOf").contains(name) || cls.equals("FeatureUtil")&&name.equals("getBasicFeatureOf") || cls.equals("TransitionUsage_source_SettingDelegate")&&name.equals("basicGet") || cls.equals("TransitionUsage_sourceFeature_InvocationDelegate")&&name.equals("dynamicInvoke"))
     methods.put(trees.getElement(TreePath.getPath(u,c))+"#"+name,PilotTransitionSourceProbe.resolved(u,m.getBody(),trees));
   }
  }
  require(methods.size()==8,"Missing source query algorithms");var f=SysMLFactory.eINSTANCE;var controls=new ArrayList<Object>();var kinds=new ArrayList<String>();
  for(var classifier:SysMLPackage.eINSTANCE.getEClassifiers())if(classifier instanceof EClass cls&&!cls.isAbstract()&&f.create(cls) instanceof Feature)kinds.add(cls.getName());Collections.sort(kinds);
  for(String kind:kinds)for(String membership:List.of("Membership","OwningMembership","FeatureMembership"))for(String chain:List.of("none","action_last","feature_last"))for(boolean link:List.of(false,true)) {
   var t=f.createTransitionUsage();var candidate=(Feature)f.create((EClass)SysMLPackage.eINSTANCE.getEClassifier(kind));candidate.setDeclaredName("candidate");
   if(!chain.equals("none")){var endpoint=chain.equals("action_last")?f.createStateUsage():f.createFeature();endpoint.setDeclaredName("last");var r=f.createFeatureChaining();r.setChainingFeature(endpoint);candidate.getOwnedRelationship().add(r);}
   var m=(Membership)f.create((EClass)SysMLPackage.eINSTANCE.getEClassifier(membership));if(m instanceof OwningMembership om)om.setOwnedMemberElement(candidate);else m.setMemberElement(candidate);t.getOwnedRelationship().add(m);
   var fallback=f.createStateUsage();fallback.setDeclaredName("fallback");var alias=f.createMembership();alias.setMemberElement(fallback);t.getOwnedRelationship().add(alias);
   if(link){var owned=f.createFeatureMembership();owned.setOwnedMemberFeature(f.createReferenceUsage());t.getOwnedRelationship().add(owned);}
   int before=t.getOwnedRelationship().size();var raw=t.sourceFeature();var source=t.getSource();int after=t.getOwnedRelationship().size();var repeated=t.getSource();
   require(source==repeated && after==t.getOwnedRelationship().size(),"Unstable source replay");
   var result=new TreeMap<String,Object>();result.put("noop",before==after);result.put("kind",kind);result.put("membership",membership);result.put("chain",chain);result.put("link",link);result.put("raw",raw==null?null:raw.getDeclaredName());result.put("source",source==null?null:source.getDeclaredName());result.put("members_before",before);result.put("members_after",after);controls.add(result);
  }
  Files.writeString(Path.of(args[1]),new GsonBuilder().serializeNulls().setPrettyPrinting().create().toJson(Map.of("methods",methods,"feature_kinds",kinds,"controls",controls))+"\n");
 }
}
