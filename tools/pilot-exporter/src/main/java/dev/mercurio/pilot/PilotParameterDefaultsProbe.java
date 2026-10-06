package dev.mercurio.pilot;
import com.google.gson.GsonBuilder;
import java.nio.file.*;
import java.util.*;
import org.eclipse.emf.ecore.EClass;
import org.omg.sysml.lang.sysml.*;
import org.omg.sysml.util.*;
/** Fixed Feature parameter selector controls only; no synthetic library provider. */
public final class PilotParameterDefaultsProbe {
 static java.lang.reflect.Method method(java.lang.Class<?> cls,String name)throws Exception {
  while(cls!=null){try{var m=cls.getDeclaredMethod(name);m.setAccessible(true);return m;}catch(NoSuchMethodException ignored){cls=cls.getSuperclass();}}
  throw new IllegalStateException(name);
 }
 public static void main(String[]args)throws Exception {
  org.omg.sysml.logic.SysMLLogicStandaloneSetup.doSetup();var f=SysMLFactory.eINSTANCE;var p=SysMLPackage.eINSTANCE;var controls=new ArrayList<Object>();
  for(String ownerKind:List.of("Behavior","Function","Step","Expression","OperatorExpression","InvocationExpression"))
   for(String membership:List.of("ParameterMembership","FeatureMembership"))for(String direction:List.of("in","out","inout"))
    for(String typing:List.of("none","Classifier","Class","Structure","DataType"))for(String valuation:List.of("absent","value","default")){
     var owner=(Type)f.create((EClass)p.getEClassifier(ownerKind));var parameter=f.createFeature();parameter.setDirection(FeatureDirectionKind.get(direction));
     var member=(FeatureMembership)f.create((EClass)p.getEClassifier(membership));member.getOwnedRelatedElement().add(parameter);owner.getOwnedRelationship().add(member);
     if(!typing.equals("none")){var type=(Type)f.create((EClass)p.getEClassifier(typing));var relation=f.createFeatureTyping();relation.setTypedFeature(parameter);relation.setType(type);parameter.getOwnedRelationship().add(relation);}
     if(!valuation.equals("absent")){var value=f.createFeatureValue();value.setIsDefault(valuation.equals("default"));value.getOwnedRelatedElement().add(f.createLiteralInteger());parameter.getOwnedRelationship().add(value);}
     var adapter=ElementUtil.getElementAdapter(parameter);var name=(String)method(adapter.getClass(),"getDefaultSupertype").invoke(adapter);
     controls.add(Map.of("owner",ownerKind,"membership",membership,"direction",direction,"typing",typing,"valuation",valuation,"default_name",name,"is_parameter",FeatureUtil.isParameter(parameter)));
    }
  Files.writeString(Path.of(args[0]),new GsonBuilder().setPrettyPrinting().create().toJson(Map.of("controls",controls))+"\n");
 }
}
