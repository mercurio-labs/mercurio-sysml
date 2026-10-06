package dev.mercurio.pilot;
import com.google.gson.GsonBuilder;
import java.nio.file.*;
import java.util.*;
import org.eclipse.xtext.scoping.*;
import org.omg.sysml.lang.sysml.*;
/** Actual Pilot scope-origin selection; global lookup is excluded from this projection. */
public final class PilotTransitionScopeProbe {
 static java.lang.reflect.Field field(java.lang.Class<?> c,String name)throws Exception {while(c!=null){try {var f=c.getDeclaredField(name);f.setAccessible(true);return f;}catch(NoSuchFieldException e){c=c.getSuperclass();}}throw new IllegalStateException(name);}
 public static void main(String[] args)throws Exception {
  org.omg.sysml.logic.SysMLLogicStandaloneSetup.doSetup();var injector=new org.omg.sysml.xtext.SysMLStandaloneSetup().createInjectorAndDoEMFRegistration();
  var provider=injector.getInstance(org.omg.sysml.xtext.scoping.SysMLScopeProvider.class);
  field(provider.getClass(),"globalScope").set(provider,(IGlobalScopeProvider)(r,f,p)->IScope.NULLSCOPE);
  var f=SysMLFactory.eINSTANCE;var rows=new ArrayList<Object>();
  for(String parentKind:List.of("Package","Class","StateUsage"))for(String ownerKind:List.of("TransitionUsage","ActionUsage"))for(String memberKind:List.of("Membership","OwningMembership","FeatureMembership"))for(String preceding:List.of("none","FeatureMembership","Membership")) {
   var parent=(Namespace)f.create((org.eclipse.emf.ecore.EClass)SysMLPackage.eINSTANCE.getEClassifier(parentKind));parent.setElementId("parent");if(parent instanceof Type t)t.setIsImpliedIncluded(true);
   var owner=(Feature)f.create((org.eclipse.emf.ecore.EClass)SysMLPackage.eINSTANCE.getEClassifier(ownerKind));owner.setElementId("owner");owner.setIsImpliedIncluded(true);var own=f.createFeatureMembership();own.setOwnedMemberFeature(owner);parent.getOwnedRelationship().add(own);
   if(!preceding.equals("none")){var before=(Membership)f.create((org.eclipse.emf.ecore.EClass)SysMLPackage.eINSTANCE.getEClassifier(preceding));if(before instanceof OwningMembership o){var feature=f.createFeature();feature.setIsImpliedIncluded(true);o.setOwnedMemberElement(feature);}owner.getOwnedRelationship().add(before);}
   var member=(Membership)f.create((org.eclipse.emf.ecore.EClass)SysMLPackage.eINSTANCE.getEClassifier(memberKind));if(member instanceof OwningMembership o){var feature=f.createFeature();feature.setIsImpliedIncluded(true);o.setOwnedMemberElement(feature);}owner.getOwnedRelationship().add(member);
   var scope=provider.getScope(member,SysMLPackage.Literals.MEMBERSHIP__MEMBER_ELEMENT);var origin=(Namespace)field(scope.getClass(),"ns").get(scope);
   rows.add(Map.of("parent_kind",parentKind,"owner_kind",ownerKind,"membership",memberKind,"preceding",preceding,"scope_origin",origin.getElementId()));
  }
  Files.writeString(Path.of(args[0]),new GsonBuilder().setPrettyPrinting().create().toJson(rows)+"\n");
 }
}
