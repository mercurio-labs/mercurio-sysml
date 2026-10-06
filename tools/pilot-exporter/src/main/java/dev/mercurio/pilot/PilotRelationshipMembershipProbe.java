package dev.mercurio.pilot;
import com.google.gson.GsonBuilder;
import java.nio.file.*;
import java.util.*;
import org.eclipse.emf.ecore.EClass;
import org.omg.sysml.lang.sysml.*;
import org.omg.sysml.util.*;
/** Factory projections only. Never installs synthetic library providers. */
public final class PilotRelationshipMembershipProbe {
 static java.lang.reflect.Method method(java.lang.Class<?> cls,String name)throws Exception {
  while(cls!=null){try{var m=cls.getDeclaredMethod(name);m.setAccessible(true);return m;}catch(NoSuchMethodException ignored){cls=cls.getSuperclass();}}
  throw new IllegalStateException(name);
 }
 public static void main(String[]args)throws Exception {
  org.omg.sysml.logic.SysMLLogicStandaloneSetup.doSetup();
  var f=SysMLFactory.eINSTANCE;var p=SysMLPackage.eINSTANCE;
  var bindings=new ArrayList<String>();var controls=new ArrayList<Object>();
  for(var classifier:p.getEClassifiers()){
   if(!(classifier instanceof EClass cls)||cls.isAbstract()||!(p.getSpecialization().isSuperTypeOf(cls)||p.getDisjoining().isSuperTypeOf(cls)||p.getFeatureInverting().isSuperTypeOf(cls)))continue;
   bindings.add(cls.getName());
   for(String typing:List.of("none","Classifier","Class","Structure","DataType")){
    var owner=f.createFeature();
    if(!typing.equals("none")){var type=(Type)f.create((EClass)p.getEClassifier(typing));var direct=f.createFeatureTyping();direct.setTypedFeature(owner);direct.setType(type);owner.getOwnedRelationship().add(direct);}
    var before=(String)method(ElementUtil.getElementAdapter(owner).getClass(),"getDefaultSupertype").invoke(ElementUtil.getElementAdapter(owner));
    var relation=(Relationship)f.create(cls);var member=f.createOwningMembership();member.getOwnedRelatedElement().add(relation);owner.getOwnedRelationship().add(member);
    var after=(String)method(ElementUtil.getElementAdapter(owner).getClass(),"getDefaultSupertype").invoke(ElementUtil.getElementAdapter(owner));
    controls.add(Map.of("kind",cls.getName(),"typing",typing,"before",before,"after",after,"owned_typing",owner.getOwnedTyping().size(),"owned_specialization",owner.getOwnedSpecialization().size(),"owned_feature",owner.getOwnedFeature().size(),"member_owner",relation.getOwner()==owner));
   }
  }
  Files.writeString(Path.of(args[0]),new GsonBuilder().setPrettyPrinting().create().toJson(Map.of("bindings",bindings,"controls",controls))+"\n");
 }
}
