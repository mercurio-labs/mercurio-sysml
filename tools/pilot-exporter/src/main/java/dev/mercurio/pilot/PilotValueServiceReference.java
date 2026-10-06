package dev.mercurio.pilot;

import java.lang.reflect.*;
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
public final class PilotValueServiceReference {
    private static final Gson JSON = new GsonBuilder().serializeNulls().disableHtmlEscaping().setPrettyPrinting().create();
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
