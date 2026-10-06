package dev.mercurio.pilot;
import java.nio.file.*;
import java.nio.charset.StandardCharsets;
import java.util.*;
import com.google.gson.GsonBuilder;
import org.omg.sysml.lang.sysml.*;
import org.omg.sysml.util.*;
import org.eclipse.xtext.resource.*;
import org.eclipse.xtext.resource.impl.*;
/** Genuine parsed source/library valuation observations, not validation. */
public final class PilotParsedResultValuationProbe {
 static String path(Element element) {
  var parts=new ArrayList<String>();
  for(org.eclipse.emf.ecore.EObject e=element;e!=null;e=e.eContainer()) {
   if(e instanceof Namespace n && n.getDeclaredName()==null && e.eContainer()!=null) return null;
   if(e instanceof Element item && item.getDeclaredName()!=null) parts.add(ElementUtil.unescapeString(item.getDeclaredName()));
  }
  Collections.reverse(parts);return String.join("::",parts);
 }
 public static void main(String[] args)throws Exception {
  org.omg.sysml.logic.SysMLLogicStandaloneSetup.doSetup();org.eclipse.emf.ecore.EPackage.Registry.INSTANCE.put("https://www.omg.org/spec/SysML/20250201",SysMLPackage.eINSTANCE);
  var kerml=new org.omg.kerml.xtext.KerMLStandaloneSetup().createInjectorAndDoEMFRegistration();var sysml=new org.omg.sysml.xtext.SysMLStandaloneSetup().createInjectorAndDoEMFRegistration();
  for(var injector:List.of(kerml,sysml)) injector.getInstance(ResourceDescriptionsProvider.class).setLiveScopeResourceDescriptions(()->injector.getInstance(ResourceSetBasedResourceDescriptions.class));
  var set=kerml.getInstance(XtextResourceSet.class);set.getLoadOptions().put(ResourceDescriptionsProvider.LIVE_SCOPE,Boolean.TRUE);
  var symbols=new HashMap<String,Element>();var names=new IdentityHashMap<Element,String>();
  for(String filename:Files.readAllLines(Path.of(args[0]))) {
   var r=set.createResource(org.eclipse.emf.common.util.URI.createFileURI(filename));try(var input=Files.newInputStream(Path.of(filename))){r.load(input,Map.of());}
   if(!r.getErrors().isEmpty())throw new IllegalStateException("Library parse: "+r.getErrors());
   var it=r.getAllContents();while(it.hasNext())if(it.next() instanceof Element e && e.getDeclaredName()!=null) {
    String n=path(e);if(n==null)continue;var previous=symbols.putIfAbsent(n,e);if(previous!=null && previous!=e)throw new IllegalStateException("Ambiguous library declaration: "+n);names.put(e,n);
   }
  }
  var converter=kerml.getInstance(org.eclipse.xtext.naming.IQualifiedNameConverter.class);var lookups=new TreeMap<String,Object>();
  SysMLLibraryUtil.setProviderLookup(r->(context,name)->{String key=String.join("::",converter.toQualifiedName(name).getSegments());var e=symbols.get(key);lookups.put(name,e==null?"absent":e.eClass().getName());return e;});
  var cases=List.of(
   "function f { return : ScalarValues::Integer = 1; }",
   "function f { return : ScalarValues::Boolean = true; }",
   "function f { return : ScalarValues::Integer = 1 + 2; }",
   "function f { return : ScalarValues::Integer = if true ? 1 else 2; }",
   "function f { in x: ScalarValues::Integer; return : ScalarValues::Integer = x; }",
   "function f { in e1: Collections::KeyValuePair; return : Base::Anything = e1.key; }",
   "function f { in col: Collections::Collection; return : Base::Anything = col.elements; }",
   "function f { in col: Collections::Collection; return : ScalarValues::Natural = SequenceFunctions::size(col.elements); }",
   "function f { in indexes: ScalarValues::Positive[1..*]; return : Base::Anything = indexes#(1); }",
   "function f { in e1: Collections::KeyValuePair; e1.key }",
   "function f { in indexes: ScalarValues::Positive[1..*]; indexes#(1) }",
   "function f { in x: ScalarValues::Integer; if x <= 1 ? x else x + 1 }");
  var rows=new ArrayList<Object>();int index=0;
  for(String body:cases) {
   String source="package ValuationProbe"+index+" { "+body+" }";var row=new TreeMap<String,Object>();row.put("case",index++);row.put("source",source);lookups.clear();
   var r=set.createResource(org.eclipse.emf.common.util.URI.createURI("memory:/valuation-"+index+".kerml"));
   try {
    r.load(new java.io.ByteArrayInputStream(source.getBytes(StandardCharsets.UTF_8)),Map.of());
    if(!r.getErrors().isEmpty())throw new IllegalStateException("Source parse: "+r.getErrors());
    Function callable=null;var it=r.getAllContents();while(it.hasNext())if(it.next() instanceof Function f && "f".equals(f.getDeclaredName())) {callable=f;break;}
    if(callable==null)throw new IllegalStateException("Missing callable");
    ElementUtil.transform(callable);var result=callable.getResult();if(result==null)throw new IllegalStateException("Missing return parameter");
    var adapter=(org.omg.sysml.adapter.FeatureAdapter)ElementUtil.getElementAdapter(result);adapter.forceComputeRedefinitions();adapter.addDefaultGeneralType();var effects=new ArrayList<Object>();
    adapter.forEachImplicitGeneralType((kind,g)->{var effect=new TreeMap<String,Object>();effect.put("kind",kind.getName());effect.put("target_kind",g.eClass().getName());String name=names.get(g);effect.put("target_name",name==null?path(g):name);effects.add(effect);});
    row.put("status","observed_component");row.put("result_kind",result.eClass().getName());
    var owning=result.getOwningType();row.put("result_owner_kind",owning==null?null:owning.eClass().getName());row.put("result_owned_by_callable",owning==callable);
    row.put("owned_return_parameter_memberships",callable.getOwnedRelationship().stream().filter(ReturnParameterMembership.class::isInstance).count());row.put("direction",result.getDirection()==null?null:result.getDirection().getLiteral());row.put("valuation_present",FeatureUtil.getValuationFor(result)!=null);row.put("effects",effects);row.put("resource_errors",r.getErrors().stream().map(Object::toString).toList());
   } catch(RuntimeException failure) {row.put("status","dependency_failure");row.put("error_class",failure.getClass().getName());row.put("error",failure.getMessage());}
   row.put("library_lookups",new TreeMap<>(lookups));rows.add(row);r.unload();set.getResources().remove(r);
  }
  Files.writeString(Path.of(args[1]),new GsonBuilder().serializeNulls().setPrettyPrinting().create().toJson(rows)+"\n");
 }
}
