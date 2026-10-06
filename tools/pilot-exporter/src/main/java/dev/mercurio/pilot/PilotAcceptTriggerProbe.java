package dev.mercurio.pilot;
import java.nio.file.*;
import java.util.*;
import com.google.gson.GsonBuilder;
import org.omg.sysml.lang.sysml.*;
import org.omg.sysml.util.*;
/** Bounded lifecycle controls with supplied complete owner/library stages. */
public final class PilotAcceptTriggerProbe {
 public static void main(String[] args) throws Exception {
  org.omg.sysml.logic.SysMLLogicStandaloneSetup.doSetup();
  var f=SysMLFactory.eINSTANCE;var cases=new ArrayList<Object>();
  for(int generals=0;generals<=2;generals++)for(int explicit=0;explicit<=2;explicit++) {
   var target=f.createAcceptActionUsage();var owner=f.createTransitionUsage();owner.setIsImpliedIncluded(true);
   SysMLLibraryUtil.setProviderLookup(r->(c,n)->null);
   ((org.omg.sysml.adapter.TypeAdapter)ElementUtil.getElementAdapter(owner)).computeImplicitGeneralTypes();
   var membership=f.createTransitionFeatureMembership();membership.setKind(TransitionFeatureKind.TRIGGER);membership.setOwnedMemberFeature(target);owner.getOwnedRelationship().add(membership);
   var ids=new IdentityHashMap<Type,String>();
   for(int i=0;i<generals;i++){var g=f.createFeature();g.setIsImpliedIncluded(true);var relation=f.createSubsetting();relation.setSubsettedFeature(g);relation.setSubsettingFeature(owner);owner.getOwnedRelationship().add(relation);}
   for(int i=0;i<explicit;i++){var g=f.createFeature();g.setIsImpliedIncluded(true);ids.put(g,"explicit"+i);var relation=f.createRedefinition();relation.setRedefinedFeature(g);relation.setRedefiningFeature(target);target.getOwnedRelationship().add(relation);}
   var accepter=f.createFeature();accepter.setIsImpliedIncluded(true);ids.put(accepter,"accepter");var lookups=new ArrayList<String>();
   SysMLLibraryUtil.setProviderLookup(r->(c,n)->{lookups.add(n);if(!n.equals("Actions::TransitionAction::accepter"))throw new IllegalStateException("Unexpected default "+n);return accepter;});
   var first=TypeUtil.getGeneralTypesOf(target,false).stream().map(t->Objects.requireNonNull(ids.get(t))).toList();
   var again=TypeUtil.getGeneralTypesOf(target,false).stream().map(t->Objects.requireNonNull(ids.get(t))).toList();
   if(!first.equals(again))throw new IllegalStateException("Non-idempotent Accept query");
   cases.add(Map.of("owner_generals",generals,"explicit",explicit,"general_types",first,"library_lookups",lookups,"is_implied_included",target.isImpliedIncluded()));
  }
  Files.writeString(Path.of(args[0]),new GsonBuilder().serializeNulls().setPrettyPrinting().create().toJson(Map.of("controls",cases))+"\n");
 }
}
