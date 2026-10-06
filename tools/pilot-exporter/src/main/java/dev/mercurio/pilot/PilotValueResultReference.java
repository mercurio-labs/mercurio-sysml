package dev.mercurio.pilot;

import java.lang.reflect.Method;
import java.lang.reflect.Field;
import java.lang.reflect.InvocationTargetException;
import java.nio.charset.StandardCharsets;
import java.nio.file.*;
import java.util.*;
import com.google.gson.*;
import org.eclipse.emf.ecore.resource.Resource;
import org.eclipse.xtext.EcoreUtil2;
import org.eclipse.xtext.resource.XtextResource;
import org.eclipse.xtext.validation.CheckMode;
import org.eclipse.xtext.diagnostics.Severity;
import org.eclipse.xtext.util.CancelIndicator;
import org.omg.sysml.interactive.SysMLInteractive;
import org.omg.sysml.lang.sysml.*;
import org.omg.sysml.util.*;

/** Isolated case contexts, real source libraries and full CheckMode.ALL.
 * No transformation-completion flags are prepared. Not native qualification. */
public final class PilotValueResultReference {
    private static final Gson JSON = new GsonBuilder().serializeNulls().disableHtmlEscaping().setPrettyPrinting().create();
    // Build-time evidence only. Raw EMF fragments/attributes are retained
    // beside structural identities; no lifecycle flag or source model is prepared.
    private static String resourceName(org.eclipse.emf.ecore.EObject value) {
        if(value.eResource()==null) return "<detached>";
        var uri=value.eResource().getURI();
        String name=uri.isFile()?uri.toFileString().replace((char)92,'/'):uri.toString();
        int index=name.indexOf("/sysml.library/");
        return index>=0?name.substring(index+1):Path.of(name).getFileName().toString();
    }
    private static String path(org.eclipse.emf.ecore.EObject value) {
        var parts=new ArrayDeque<String>();
        var current=value;
        while(current.eContainer()!=null) {
            var parent=current.eContainer();var feature=current.eContainmentFeature();
            Object contents=parent.eGet(feature,false);
            int index=feature.isMany()?((List<?>)contents).indexOf(current):0;
            parts.addFirst(feature.getName()+"["+index+"]");current=parent;
        }
        int root=current.eResource()==null?0:current.eResource().getContents().indexOf(current);
        return resourceName(value)+"#contents["+root+"]"+(parts.isEmpty()?"":"/"+String.join("/",parts));
    }
    private static JsonObject identity(org.eclipse.emf.ecore.EObject value) {
        var result=new JsonObject();result.addProperty("id",path(value));
        result.addProperty("kind",value.eClass().getName());result.addProperty("resource",resourceName(value));
        if(value.eResource()!=null) result.addProperty("emf_fragment",value.eResource().getURIFragment(value));
        if(value instanceof Element element) result.addProperty("qualified_name",element.getQualifiedName());
        return result;
    }
    private static JsonObject stored(org.eclipse.emf.ecore.EObject value) {
        var result=identity(value);var attributes=new JsonObject();var references=new JsonObject();var isSet=new JsonObject();
        for(var feature:value.eClass().getEAllStructuralFeatures()) {
            if(feature.isDerived()||feature.isVolatile())continue;
            Object actual=value.eGet(feature,false);isSet.addProperty(feature.getName(),value.eIsSet(feature));
            if(feature instanceof org.eclipse.emf.ecore.EReference) {
                var ids=new JsonArray();
                if(actual instanceof Collection<?> collection) {
                    for(var item:collection)ids.add(identity((org.eclipse.emf.ecore.EObject)item));
                } else if(actual!=null)ids.add(identity((org.eclipse.emf.ecore.EObject)actual));
                references.add(feature.getName(),ids);
            } else {
                if(actual instanceof org.eclipse.emf.common.util.Enumerator literal)attributes.addProperty(feature.getName(),literal.getLiteral());
                else attributes.add(feature.getName(),JSON.toJsonTree(actual));
            }
        }
        result.add("stored_attributes",attributes);result.add("reference_sequences",references);result.add("is_set",isSet);
        return result;
    }
    private static void witness(Map<String,JsonObject> records,org.eclipse.emf.ecore.EObject value) {
        if(value!=null)records.computeIfAbsent(path(value),key->stored(value));
    }
    private static JsonObject observeResults(XtextResource resource) {
        var rows=new JsonArray();var witnesses=new LinkedHashMap<String,JsonObject>();
        var values=new ArrayList<Element>();
        var it=resource.getAllContents();while(it.hasNext())if(it.next() instanceof Element e)values.add(e);
        for(var element:values) {
            if(!(element instanceof Function)&&!(element instanceof Expression))continue;
            var type=(Type)element;var row=identity(element);
            Feature result=element instanceof Function function?function.getResult():((Expression)element).getResult();
            Feature owned=TypeUtil.getOwnedResultParameterOf(type);
            row.add("result",result==null?JsonNull.INSTANCE:identity(result));
            row.add("owned_result",owned==null?JsonNull.INSTANCE:identity(owned));
            var ownReturns=new JsonArray();
            for(var relation:element.getOwnedRelationship())if(relation instanceof ReturnParameterMembership) {
                ownReturns.add(identity(relation));witness(witnesses,relation);
                for(var member:relation.getOwnedRelatedElement())witness(witnesses,member);
            }
            row.add("owned_return_memberships",ownReturns);
            var types=new JsonArray();
            if(element instanceof Feature feature)for(var prototype:feature.getType()) {
                types.add(identity(prototype));witness(witnesses,prototype);
            }
            row.add("types",types);
            if(result!=null) {
                witness(witnesses,result);
                var membership=result.eContainer();witness(witnesses,membership);
                if(membership!=null)witness(witnesses,membership.eContainer());
                // Canonical multiplicity bounds/flags and return memberships
                // are represented through their stored fields, not OCL claims.
                for(var relation:result.getOwnedRelationship()) {
                    witness(witnesses,relation);
                    for(var child:relation.getOwnedRelatedElement()) {
                        witness(witnesses,child);
                        for(var nested:child.getOwnedRelationship()) {
                            witness(witnesses,nested);
                            for(var leaf:nested.getOwnedRelatedElement())witness(witnesses,leaf);
                        }
                    }
                }
            }
            witness(witnesses,element);rows.add(row);
        }
        var result=new JsonObject();result.add("queries",rows);
        var records=new JsonArray();witnesses.values().forEach(records::add);result.add("stored_witnesses",records);
        result.addProperty("meaning","Direct resolved result/type getters and canonical stored ownership/reference/flag witnesses. This is independent evidence, not native implementation or complete dependency closure.");
        return result;
    }
    public static void main(String[] args) throws Exception {
        Path library = Path.of(args[0]).toAbsolutePath().normalize();
        var spec = JsonParser.parseString(Files.readString(Path.of(args[1]))).getAsJsonObject();
        var out = Path.of(args[2]); Files.createDirectories(out.getParent());
        System.setProperty("org.eclipse.emf.common.util.ReferenceClearingQueue", "false");
        var interactive = SysMLInteractive.getInstance();
        interactive.getLibraryIndexCache().setIndexDisabled(true);
        interactive.setVerbose(false);
        interactive.loadLibrary(library.toString().replace('\\','/'));
        var set = interactive.getResourceSet();
        Method export = PilotModelExporter.class.getDeclaredMethod("exportDocument", Path.class, List.class, org.eclipse.emf.ecore.resource.ResourceSet.class);
        export.setAccessible(true);
        Field sourceField = PilotModelExporter.class.getDeclaredField("assessmentSources");
        sourceField.setAccessible(true);
        @SuppressWarnings("unchecked")
        Set<Resource> assessmentSources = (Set<Resource>)sourceField.get(null);
        var rows = new JsonArray();
        for (var item : spec.getAsJsonArray("cases")) {
            var c = item.getAsJsonObject(); var row = new JsonObject();
            row.addProperty("id", c.get("id").getAsString());
            row.addProperty("observation_kind", c.has("mutation") ? "parsed_source_with_explicit_ecore_mutation" : "isolated_parsed_source");
            XtextResource resource = null;
            System.err.println("Reference case " + c.get("id").getAsString());
            try {
                resource = (XtextResource)interactive.readResource(c.get("path").getAsString().replace('\\','/'));
                interactive.addInputResource(resource);
                row.addProperty("parse_ok", resource.getParseResult()!=null && !resource.getParseResult().hasSyntaxErrors());
                if(c.has("mutation")) {
                    Feature owner=null;
                    var it=resource.getAllContents();
                    while(it.hasNext()) if(it.next() instanceof Feature f && "v".equals(f.getDeclaredName())) {owner=f;break;}
                    if(owner==null) throw new IllegalStateException("Missing parsed mutation owner v");
                    var value=SysMLFactory.eINSTANCE.createLiteralInteger(); value.setValue(2);
                    var fv=FeatureUtil.addFeatureValueTo(owner,value);
                    fv.setIsDefault(c.get("mutation").getAsString().equals("append_default_valuation"));
                    row.addProperty("mutation",c.get("mutation").getAsString());
                }
                EcoreUtil2.resolveLazyCrossReferences(resource, null);
                var issues=resource.getResourceServiceProvider().getResourceValidator().validate(resource,CheckMode.ALL,CancelIndicator.NullImpl);
                var diagnostics=new JsonArray();
                for(var issue:issues) {
                    var d=new JsonObject();
                    d.addProperty("severity",issue.getSeverity().toString());d.addProperty("code",issue.getCode());
                    d.addProperty("message",issue.getMessage());d.addProperty("syntax",issue.isSyntaxError());
                    d.addProperty("line",issue.getLineNumber());d.addProperty("column",issue.getColumn());
                    diagnostics.add(d);
                }
                row.add("diagnostics",diagnostics);
                boolean rejected=issues.stream().anyMatch(i->i.getSeverity()==Severity.ERROR);
                row.addProperty("validation_status",rejected?"rejected":"accepted");
                // Rejected models are diagnostic controls, not published successful models.
                if(!rejected) {
                    ElementUtil.transformAll(resource,true);
                    EcoreUtil2.resolveLazyCrossReferences(resource,null);
                    assessmentSources.add(resource);
                    var model=JSON.toJsonTree(export.invoke(null,library,List.of(resource),set)).getAsJsonObject();
                    model.getAsJsonObject("metadata").remove("exported_at_utc");
                    row.add("result_observations",observeResults(resource));
                    row.add("model",model);
                    row.addProperty("model_status","exported");
                } else row.addProperty("model_status","not_published");
            } catch(Throwable failure) {
                if(failure instanceof InvocationTargetException e && e.getCause()!=null) failure=e.getCause();
                if(failure instanceof VirtualMachineError vm && !(failure instanceof StackOverflowError)) throw vm;
                row.addProperty("validation_status","infrastructure_error");
                row.addProperty("error_class",failure.getClass().getName());
                row.addProperty("error",failure.getMessage());
            } finally {
                if(resource!=null) {
                    assessmentSources.remove(resource);
                    interactive.getInputResources().remove(resource);
                    resource.unload();set.getResources().remove(resource);
                }
            }
            rows.add(row);
            // Preserve completed observations when a later independent case fails.
            Files.writeString(out,JSON.toJson(rows)+"\n",StandardCharsets.UTF_8);
        }
    }
}
