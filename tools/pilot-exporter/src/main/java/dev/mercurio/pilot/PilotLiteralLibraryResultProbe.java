package dev.mercurio.pilot;
import com.google.gson.*;
import java.nio.file.*;
import java.nio.charset.StandardCharsets;
import java.io.ByteArrayInputStream;
import java.util.*;
import org.eclipse.emf.common.util.URI;
import org.eclipse.xtext.resource.*;
import org.eclipse.xtext.resource.impl.*;
import org.omg.sysml.lang.sysml.*;
import org.omg.sysml.util.*;
/** Real-library result queries only, no validation or synthetic library declarations. */
public final class PilotLiteralLibraryResultProbe {
 static List<String> path(Element element){
  var result=new ArrayList<String>();
  for(Element cursor=element;cursor!=null;cursor=NamespaceUtil.getParentNamespaceOf(cursor))if(cursor.getDeclaredName()!=null)result.add(ElementUtil.unescapeString(cursor.getDeclaredName()));
  Collections.reverse(result);return result;
 }
 public static void main(String[]args)throws Exception {
  org.omg.sysml.logic.SysMLLogicStandaloneSetup.doSetup();
  org.eclipse.emf.ecore.EPackage.Registry.INSTANCE.put("https://www.omg.org/spec/SysML/20250201",SysMLPackage.eINSTANCE);
  var injector=new org.omg.kerml.xtext.KerMLStandaloneSetup().createInjectorAndDoEMFRegistration();
  var descriptions=injector.getInstance(ResourceDescriptionsProvider.class);
  descriptions.setLiveScopeResourceDescriptions(()->injector.getInstance(ResourceSetBasedResourceDescriptions.class));
  var library=injector.getInstance(org.omg.kerml.xtext.library.KerMLLibraryProvider.class);
  SysMLLibraryUtil.setProviderLookup(resource->library);
  var set=injector.getInstance(XtextResourceSet.class);set.getLoadOptions().put(ResourceDescriptionsProvider.LIVE_SCOPE,Boolean.TRUE);
  var inputs=JsonParser.parseString(Files.readString(Path.of(args[0]))).getAsJsonObject().getAsJsonArray("cases").get(0).getAsJsonObject().getAsJsonArray("input_files");
  for(var input:inputs){var resource=(XtextResource)set.getResource(URI.createFileURI(input.getAsString()),true);if(!resource.getErrors().isEmpty())throw new IllegalStateException(resource.getErrors().toString());}
  var source="package LiteralProbe { feature integerValue = 1; feature rationalValue = 1.5; feature booleanValue = true; feature stringValue = \"text\"; feature infinityValue = *; feature nullValue = null; feature bounds[0..1]; }";
  var resource=(XtextResource)set.createResource(URI.createURI("memory:/literal-results.kerml"));resource.load(new ByteArrayInputStream(source.getBytes(StandardCharsets.UTF_8)),Map.of());
  if(!resource.getErrors().isEmpty())throw new IllegalStateException(resource.getErrors().toString());
  var expressions=new ArrayList<Expression>();var iterator=resource.getAllContents();
  while(iterator.hasNext()){var object=iterator.next();if(object instanceof LiteralExpression || object instanceof NullExpression)expressions.add((Expression)object);}
  var controls=new ArrayList<Object>();
  for(var expression:expressions){
   var row=new TreeMap<String,Object>();row.put("kind",expression.eClass().getName());row.put("owner_path",path(expression));
   var result=TypeUtil.getResultParameterOf(expression);row.put("result_present",result!=null&&!result.eIsProxy());
   if(result!=null&&!result.eIsProxy()){row.put("result_kind",result.eClass().getName());row.put("result_path",path(result));row.put("result_source_uri",result.eResource().getURI().toString());row.put("result_uri",org.eclipse.emf.ecore.util.EcoreUtil.getURI(result).toString());row.put("result_membership_kind",result.getOwningRelationship().eClass().getName());row.put("result_direction",result.getDirection().getLiteral());}
   controls.add(row);
  }
  var errors=new ArrayList<Object>();
  for(var loaded:set.getResources())for(var error:loaded.getErrors())errors.add(Map.of("source_uri",loaded.getURI().toString(),"message",error.getMessage(),"line",error.getLine()));
  Files.writeString(Path.of(args[1]),new GsonBuilder().setPrettyPrinting().create().toJson(Map.of("source",source,"provider",library.getClass().getName(),"controls",controls,"resource_errors",errors))+"\n");
 }
}
