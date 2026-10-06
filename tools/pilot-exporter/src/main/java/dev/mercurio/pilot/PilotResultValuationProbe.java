package dev.mercurio.pilot;
import com.google.gson.GsonBuilder;
import com.sun.source.tree.*;
import com.sun.source.util.*;
import javax.tools.*;
import java.nio.file.*;
import java.nio.charset.StandardCharsets;
import java.util.*;
import org.omg.sysml.lang.sysml.*;
import org.omg.sysml.util.*;
/** Independent bounded result-default effects with owned valuation contexts. */
public final class PilotResultValuationProbe {
 public static void main(String[] args)throws Exception {
  org.omg.sysml.logic.SysMLLogicStandaloneSetup.doSetup();var methods=new TreeMap<String,Object>();
  var selected=Map.of("FeatureAdapter",List.of("addDefaultGeneralType","getDefaultSupertype","addBoundValueSubsetting","getBoundValueResult"),"FeatureUtil",List.of("getValuationFor","addFeatureValueTo"),"FeatureValue_value_SettingDelegate",List.of("basicGet"));
  var compiler=ToolProvider.getSystemJavaCompiler();var diagnostics=new DiagnosticCollector<JavaFileObject>();
  try(var files=compiler.getStandardFileManager(diagnostics,null,StandardCharsets.UTF_8)) {
   var task=(JavacTask)compiler.getTask(null,files,diagnostics,List.of("-proc:none","-classpath",args[0]),null,files.getJavaFileObjectsFromStrings(Arrays.asList(args).subList(2,args.length)));
   var units=new ArrayList<CompilationUnitTree>();task.parse().forEach(units::add);task.analyze();var trees=Trees.instance(task);
   if(diagnostics.getDiagnostics().stream().anyMatch(d->d.getKind()==Diagnostic.Kind.ERROR))throw new IllegalStateException(diagnostics.getDiagnostics().toString());
   for(var u:units)for(var d:u.getTypeDecls())if(d instanceof ClassTree c)for(var member:c.getMembers())if(member instanceof MethodTree m&&m.getBody()!=null&&selected.getOrDefault(c.getSimpleName().toString(),List.of()).contains(m.getName().toString())) methods.put(trees.getElement(TreePath.getPath(u,c))+"#"+m.getName()+"/"+m.getParameters().size(),PilotOwnerTypingProbe.resolved(u,m.getBody(),trees));
  }
  if(methods.size()!=7)throw new IllegalStateException("Missing resolved programs: "+methods.keySet());
  var f=SysMLFactory.eINSTANCE;var controls=new ArrayList<Object>();
  for(String ownerKind:List.of("Expression","FeatureReferenceExpression","Function","Predicate"))for(int mask=0;mask<8;mask++)for(String shape:List.of("Expression","FeatureReferenceExpression","LiteralInteger","OperatorExpression","FeatureChainExpression","InvocationExpression"))for(int mode=0;mode<4;mode++) {
   try {
   var owner=(Type)f.create((org.eclipse.emf.ecore.EClass)SysMLPackage.eINSTANCE.getEClassifier(ownerKind));owner.setIsImpliedIncluded(true);
   var result=f.createFeature();result.setDirection(FeatureDirectionKind.OUT);var member=f.createReturnParameterMembership();member.setOwnedMemberParameter(result);owner.getOwnedRelationship().add(member);
   if((mask&1)!=0)FeatureUtil.addFeatureTypingTo(result).setType(f.createClass());
   if((mask&2)!=0)FeatureUtil.addFeatureTypingTo(result).setType(f.createStructure());
   if((mask&4)!=0)FeatureUtil.addFeatureTypingTo(result).setType(f.createDataType());
   var value=(Expression)f.create((org.eclipse.emf.ecore.EClass)SysMLPackage.eINSTANCE.getEClassifier(shape));value.setIsImpliedIncluded(true);
   var valueResult=f.createFeature();valueResult.setDirection(FeatureDirectionKind.OUT);var valueMember=f.createReturnParameterMembership();valueMember.setOwnedMemberParameter(valueResult);value.getOwnedRelationship().add(valueMember);
   var valuation=FeatureUtil.addFeatureValueTo(result,value);valuation.setIsDefault((mode&1)!=0);valuation.setIsInitial((mode&2)!=0);
   var library=new HashMap<String,Element>();SysMLLibraryUtil.setProviderLookup(r -> (c,n)->library.computeIfAbsent(n,key->{var feature=f.createFeature();feature.setElementId(key);feature.setIsImpliedIncluded(true);return feature;}));
   var adapter=(org.omg.sysml.adapter.FeatureAdapter)ElementUtil.getElementAdapter(result);adapter.forceComputeRedefinitions();adapter.addDefaultGeneralType();var generals=new ArrayList<Object>();adapter.forEachImplicitGeneralType((k,g)->generals.add(Map.of("kind",k.getName(),"target",g.getElementId())));
   controls.add(Map.of("owner_kind",ownerKind,"mask",mask,"shape",shape,"mode",mode,"status","observed_component","canonical_value",valuation.getValue()==value,"generals",generals));
   } catch(RuntimeException failure) {
    var issue=new TreeMap<String,Object>();issue.put("owner_kind",ownerKind);issue.put("mask",mask);issue.put("shape",shape);issue.put("mode",mode);issue.put("status","fixture_dependency_failure");issue.put("error_class",failure.getClass().getName());issue.put("error",failure.getMessage());controls.add(issue);
   }
  }
  Files.writeString(Path.of(args[1]),new GsonBuilder().setPrettyPrinting().create().toJson(Map.of("methods",methods,"controls",controls))+"\n");
 }
}
