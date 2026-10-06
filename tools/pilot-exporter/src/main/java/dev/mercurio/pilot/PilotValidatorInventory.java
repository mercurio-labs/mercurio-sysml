package dev.mercurio.pilot;

import com.google.gson.GsonBuilder;
import com.google.inject.Injector;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.*;
import org.eclipse.emf.common.util.URI;
import org.eclipse.emf.ecore.EObject;
import org.eclipse.emf.ecore.resource.Resource;
import org.eclipse.xtend.core.XtendStandaloneSetup;
import org.eclipse.xtend.core.xtend.*;
import org.eclipse.xtext.common.types.*;
import org.eclipse.xtext.nodemodel.INode;
import org.eclipse.xtext.nodemodel.util.NodeModelUtils;
import org.eclipse.xtext.resource.XtextResource;
import org.eclipse.xtext.resource.XtextResourceSet;
import org.eclipse.xtext.xbase.*;
import org.eclipse.xtext.xbase.jvmmodel.IJvmModelAssociations;
import org.eclipse.xtext.xbase.typesystem.*;
import org.eclipse.xtext.xbase.typesystem.references.LightweightTypeReference;

/** Inventory declarations and actual resolved calls; this is not semantic translation. */
public final class PilotValidatorInventory {
    private static Map<String,Object> span(EObject object) {
        INode n=NodeModelUtils.getNode(object);
        if(n==null) return Map.of("synthetic",true,"source_span","unavailable");
        return Map.of("offset",n.getOffset(),"length",n.getLength(),"start_line",n.getStartLine(),"end_line",n.getEndLine());
    }
    private static List<EObject> contents(EObject root) {
        List<EObject> result=new ArrayList<>();
        if(root!=null) { result.add(root); root.eAllContents().forEachRemaining(result::add); }
        return result;
    }
    private static String type(JvmTypeReference reference) {
        return reference==null ? null : reference.getIdentifier();
    }
    private static Map<String,Object> dependency(EObject target) {
        Map<String,Object> row=new TreeMap<>();
        row.put("kind",target.eClass().getName());
        if(target instanceof JvmIdentifiableElement element) row.put("id",element.getIdentifier());
        else { row.put("local",true); row.put("span",span(target)); }
        if(target instanceof JvmFeature feature) row.put("declaring_type",feature.getDeclaringType().getIdentifier());
        if(target instanceof JvmOperation operation) {
            row.put("name",operation.getSimpleName());
            row.put("parameter_types",operation.getParameters().stream().map(p->type(p.getParameterType())).toList());
            row.put("result_type",type(operation.getReturnType()));
            row.put("static",operation.isStatic());
            row.put("varargs",operation.isVarArgs());
        } else if(target instanceof JvmConstructor constructor) {
            row.put("parameter_types",constructor.getParameters().stream().map(p->type(p.getParameterType())).toList());
            row.put("result_type",constructor.getDeclaringType().getIdentifier());
        } else if(target instanceof JvmField field) {
            row.put("name",field.getSimpleName()); row.put("result_type",type(field.getType()));
            row.put("static",field.isStatic()); row.put("final",field.isFinal());
        }
        return row;
    }
    public static void main(String[] args) throws Exception {
        if(args.length!=3) throw new IllegalArgumentException("Usage: PilotValidatorInventory ROOT SOURCES_JSON OUTPUT_JSON");
        Path root=Path.of(args[0]);
        String[] sources=new com.google.gson.Gson().fromJson(Files.readString(Path.of(args[1])),String[].class);
        Injector injector=new XtendStandaloneSetup().createInjectorAndDoEMFRegistration();
        XtextResourceSet resources=injector.getInstance(XtextResourceSet.class);
        resources.setClasspathURIContext(PilotValidatorInventory.class.getClassLoader());
        IJvmModelAssociations associations=injector.getInstance(IJvmModelAssociations.class);
        Map<String,Resource> loaded=new TreeMap<>();
        for(String source:sources) {
            Resource resource=resources.getResource(URI.createFileURI(root.resolve(source).toAbsolutePath().toString()),true);
            if(!(resource instanceof XtextResource xtext) || xtext.getParseResult()==null || xtext.getParseResult().hasSyntaxErrors())
                throw new IllegalStateException("Xtend parse failed for "+source+": "+resource.getErrors());
            loaded.put(source,resource);
        }
        List<Object> files=new ArrayList<>();
        List<Map<String,Object>> methods=new ArrayList<>();
        for(var input:loaded.entrySet()) {
            Resource resource=input.getValue();
            XtendFile file=(XtendFile)resource.getContents().get(0);
            IResolvedTypes resolved=injector.getInstance(IBatchTypeResolver.class).resolveTypes(resource);
            List<Object> fileIssues=new ArrayList<>();
            for(var diagnostic:resolved.getQueuedDiagnostics())
                fileIssues.add(Map.of("offset",diagnostic.getOffset(),"length",diagnostic.getLength(),"message",diagnostic.getMessage()));
            int count=0, checks=0;
            for(EObject object:contents(file)) if(object instanceof XtendFunction method) {
                count++;
                XtendTypeDeclaration declaring=(XtendTypeDeclaration)method.eContainer();
                String owner=file.getPackage()+"."+declaring.getName();
                List<String> annotations=new ArrayList<>();
                for(var annotation:method.getAnnotations()) {
                    JvmType annotationType=annotation.getAnnotationType();
                    if(annotationType==null || annotationType.eIsProxy()) throw new IllegalStateException("Unresolved method annotation in "+input.getKey()+" "+method.getName());
                    annotations.add(annotationType.getIdentifier());
                }
                boolean check=annotations.contains("org.eclipse.xtext.validation.Check");
                if(check)checks++;
                Map<String,Object> row=new TreeMap<>();
                row.put("rule_id",owner+"#"+method.getName());
                row.put("id",owner+"#"+method.getName()+"("+String.join(",",method.getParameters().stream().map(p->type(p.getParameterType())).toList())+")");
                row.put("name",method.getName()); row.put("declaring_type",owner);
                row.put("source",input.getKey()); row.put("span",span(method)); row.put("annotations",annotations); row.put("is_check",check);
                row.put("explicit_annotation_values",method.getAnnotations().stream().filter(a->a.getValue()!=null || !a.getElementValuePairs().isEmpty()).count());
                row.put("generic_parameters",method.getTypeParameters().size());
                row.put("parameters",method.getParameters().stream().map(p->Map.of("name",p.getName(),"type",type(p.getParameterType()))).toList());
                List<String> jvmIds=new ArrayList<>();
                for(EObject inferred:associations.getJvmElements(method)) if(inferred instanceof JvmOperation op) jvmIds.add(op.getIdentifier());
                Collections.sort(jvmIds); row.put("jvm_ids",jvmIds);
                Map<String,Integer> kinds=new TreeMap<>();
                int mutableLocals=0;
                List<Object> calls=new ArrayList<>(), issues=new ArrayList<>();
                for(EObject node:contents(method.getExpression())) {
                    kinds.merge(node.eClass().getName(),1,Integer::sum);
                    if(node instanceof XVariableDeclaration local && local.isWriteable())mutableLocals++;
                    if(node instanceof XExpression expression) {
                        LightweightTypeReference actual=resolved.getActualType(expression);
                        if(actual==null || actual.isUnknown()) issues.add(Map.of("kind","unresolved-expression-type","span",span(node)));
                    }
                    if(node instanceof XAbstractFeatureCall call) {
                        Map<String,Object> callRow=new TreeMap<>(); callRow.put("span",span(node)); callRow.put("node_kind",node.eClass().getName());
                        callRow.put("explicit_arguments",call.getExplicitArguments().size());
                        callRow.put("explicit_type_arguments",call.getTypeArguments().stream().map(PilotValidatorInventory::type).toList());
                        callRow.put("null_safe",call instanceof XMemberFeatureCall member && member.isNullSafe());
                        EObject linked=resolved.getLinkedFeature(call);
                        if(call.isPackageFragment()) {
                            callRow.put("resolved",true); callRow.put("target",Map.of("kind","PackageFragment","name",call.getConcreteSyntaxFeatureName()));
                        } else if(linked==null || linked.eIsProxy() || call.getInvalidFeatureIssueCode()!=null) {
                            callRow.put("resolved",false); callRow.put("issue_code",call.getInvalidFeatureIssueCode());
                            issues.add(Map.of("kind","unresolved-feature","span",span(node)));
                        } else { callRow.put("resolved",true); callRow.put("target",dependency(linked)); }
                        calls.add(callRow);
                    } else if(node instanceof XConstructorCall constructor) {
                        Map<String,Object> callRow=new TreeMap<>(); callRow.put("span",span(node)); callRow.put("node_kind",node.eClass().getName());
                        EObject linked=resolved.getLinkedFeature(constructor);
                        boolean valid=linked!=null && !linked.eIsProxy();
                        callRow.put("resolved",valid);
                        if(valid)callRow.put("target",dependency(linked));
                        else issues.add(Map.of("kind","unresolved-constructor","span",span(node)));
                        calls.add(callRow);
                    }
                }
                INode location=NodeModelUtils.getNode(method);
                for(var diagnostic:resolved.getQueuedDiagnostics()) if(diagnostic.getOffset()>=location.getOffset() && diagnostic.getOffset()<location.getEndOffset())
                    issues.add(Map.of("kind","type-system-diagnostic","offset",diagnostic.getOffset(),"message",diagnostic.getMessage()));
                row.put("mutable_locals",mutableLocals);
                row.put("body_node_kinds",kinds); row.put("calls",calls); row.put("resolution_issues",issues);
                row.put("resolution_status",issues.isEmpty()?"resolved":"incomplete");
                methods.add(row);
            }
            files.add(Map.of("source",input.getKey(),"methods",count,"checks",checks,"type_system_diagnostics",fileIssues));
        }
        methods.sort(Comparator.comparing(m->(String)m.get("id")));
        Map<String,Object> output=new TreeMap<>();
        output.put("schema_version",1); output.put("source_format","resolved-xtend-validator-inventory");
        output.put("parser","org.eclipse.xtend.core.XtendStandaloneSetup"); output.put("span_encoding","utf-16-code-units");
        output.put("files",files); output.put("methods",methods);
        Files.writeString(Path.of(args[2]),new GsonBuilder().serializeNulls().setPrettyPrinting().disableHtmlEscaping().create().toJson(output)+"\n",StandardCharsets.UTF_8);
    }
}
