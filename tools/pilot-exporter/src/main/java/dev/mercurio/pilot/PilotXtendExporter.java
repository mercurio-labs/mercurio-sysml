package dev.mercurio.pilot;

import com.google.gson.GsonBuilder;
import com.google.inject.Injector;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.*;
import org.eclipse.emf.common.util.URI;
import org.eclipse.emf.ecore.*;
import org.eclipse.emf.ecore.resource.Resource;
import org.eclipse.emf.ecore.util.EcoreUtil;
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

/** Export selected raw Xtend methods using the actual parser, JVM linker and type system. */
public final class PilotXtendExporter {
    private static final String SOURCE = "org.omg.kerml.xtext/src/org/omg/kerml/xtext/validation/KerMLValidator.xtend";
    private static final Map<String, Set<String>> SELECTION = Map.of(
        SOURCE, Set.of("checkImport", "checkAnnotation"),
        "org.omg.sysml.xtext/src/org/omg/sysml/xtext/validation/SysMLValidator.xtend",
        Set.of("checkReferenceUsage", "checkEnumerationDefinition", "checkEnumerationUsage", "checkAnalysisCaseUsage", "checkVerificationCaseUsage", "checkUseCaseUsage", "checkRenderingUsage", "checkViewpointUsage", "checkMetadataUsage", "checkOneType"));
    private Map<String, Set<String>> selection = SELECTION;
    private final Map<Resource, String> sources = new IdentityHashMap<>();
    private final Map<Resource, IResolvedTypes> resolutions = new IdentityHashMap<>();
    private final Map<EObject, String> identities = new IdentityHashMap<>();
    private final Map<String, Object> symbols = new TreeMap<>();
    private final Map<String, Object> types = new TreeMap<>();
    private final Map<String, Integer> counts = new TreeMap<>();
    private IResolvedTypes resolved;
    private IJvmModelAssociations associations;

    public static void main(String[] args) throws Exception {
        if (args.length != 2 && args.length != 3) throw new IllegalArgumentException("Usage: PilotXtendExporter PILOT_ROOT OUTPUT_JSON [SELECTION_JSON]");
        PilotXtendExporter exporter = new PilotXtendExporter();
        if (args.length == 3) {
            com.google.gson.JsonObject selected = com.google.gson.JsonParser.parseString(Files.readString(Path.of(args[2]), StandardCharsets.UTF_8)).getAsJsonObject();
            Map<String, Set<String>> selection = new TreeMap<>();
            for (String source : selected.keySet()) {
                Set<String> names = new TreeSet<>();
                for (com.google.gson.JsonElement name : selected.getAsJsonArray(source))
                    if (!names.add(name.getAsString())) throw new IllegalArgumentException("Duplicate selected method");
                if (names.isEmpty()) throw new IllegalArgumentException("Empty method selection");
                selection.put(source, names);
            }
            if (selection.isEmpty()) throw new IllegalArgumentException("Empty source selection");
            exporter.selection = selection;
        }
        exporter.export(Path.of(args[0]), Path.of(args[1]));
    }
    private void export(Path root, Path output) throws Exception {
        Injector injector = new XtendStandaloneSetup().createInjectorAndDoEMFRegistration();
        XtextResourceSet resources = injector.getInstance(XtextResourceSet.class);
        resources.setClasspathURIContext(PilotXtendExporter.class.getClassLoader());
        associations = injector.getInstance(IJvmModelAssociations.class);
        for (String source : new TreeSet<>(selection.keySet())) {
            Resource resource = resources.getResource(URI.createFileURI(root.resolve(source).toAbsolutePath().toString()), true);
            if (!(resource instanceof XtextResource xtext) || xtext.getParseResult() == null || xtext.getParseResult().hasSyntaxErrors())
                throw new IllegalStateException("Xtend parse failed: " + resource.getErrors());
            sources.put(resource, source);
        }
        List<XtendFunction> methods = new ArrayList<>();
        for (String source : new TreeSet<>(selection.keySet())) {
            Resource resource = resources.getResource(URI.createFileURI(root.resolve(source).toAbsolutePath().toString()), false);
            resolutions.put(resource, injector.getInstance(IBatchTypeResolver.class).resolveTypes(resource));
            XtendFile file = (XtendFile) resource.getContents().get(0);
            Set<String> selected = new HashSet<>();
            for (XtendTypeDeclaration type : file.getXtendTypes()) for (XtendMember member : type.getMembers()) {
                if (member instanceof XtendFunction method && selection.get(source).contains(method.getName())) {
                    if (!selected.add(method.getName())) throw new IllegalStateException("Ambiguous selected method");
                    index(method, file.getPackage() + "." + type.getName() + "#" + method.getName());
                    methods.add(method);
                }
            }
            if (!selected.equals(selection.get(source))) throw new IllegalStateException("Incomplete method selection");
        }
        for (XtendFunction method : methods) {
            resolved = resolutions.get(method.eResource());
            for (org.eclipse.xtext.diagnostics.AbstractDiagnostic diagnostic : resolved.getQueuedDiagnostics()) {
                INode location = NodeModelUtils.getNode(method);
                if (diagnostic.getOffset() >= location.getOffset() && diagnostic.getOffset() < location.getEndOffset())
                    throw new IllegalStateException("Selected Xtend method has linking/type diagnostic: " + diagnostic);
            }
            for (XtendParameter parameter : method.getParameters())
                for (EObject inferred : associations.getJvmElements(parameter)) identities.put(inferred, identities.get(parameter));
        }
        List<Object> exported = new ArrayList<>();
        for (XtendFunction method : methods) {
            resolved = resolutions.get(method.eResource());
            XtendFile file = (XtendFile) method.eResource().getContents().get(0);
            Map<String,Object> row = node(method);
            row.put("declaring_type", file.getPackage() + "." + ((XtendTypeDeclaration)method.eContainer()).getName());
            row.put("source", sources.get(method.eResource())); row.put("name", method.getName());
            exported.add(row);
        }
        Map<String,Object> document = new LinkedHashMap<>();
        document.put("schema_version", 1);
        document.put("source_format", "raw-xtend");
        document.put("parser", "org.eclipse.xtend.core.XtendStandaloneSetup");
        document.put("span_encoding", "utf-16-code-units");
        document.put("methods", exported);
        document.put("symbols", symbols);
        document.put("types", types);
        document.put("coverage", Map.of("node_kinds", counts, "nodes", counts.values().stream().mapToInt(Integer::intValue).sum(), "selected_methods", methods.size(), "unresolved_references", 0));
        Files.writeString(output, new GsonBuilder().serializeNulls().setPrettyPrinting().disableHtmlEscaping().create().toJson(document) + "\n", StandardCharsets.UTF_8);
    }
    private void index(EObject object, String identity) {
        if (object.eIsProxy()) throw new IllegalStateException("Unresolved AST node " + identity);
        if (identities.put(object, identity) != null) throw new IllegalStateException("Repeated containment: " + identity + " " + object.eClass().getName());
        for (EStructuralFeature feature : object.eClass().getEAllStructuralFeatures()) if (!feature.isDerived() && feature instanceof EReference ref && ref.isContainment()) {
            Object value = object.eGet(feature, true);
            if (feature.isMany()) { int i=0; for(Object child:(List<?>)value) { if(((EObject)child).eContainingFeature()==feature && ((EObject)child).eContainer()==object) index((EObject)child,identity+"/"+feature.getName()+"/"+i); i++; } }
            else if (value != null && ((EObject)value).eContainingFeature()==feature && ((EObject)value).eContainer()==object) index((EObject)value,identity+"/"+feature.getName());
        }
    }
    private Map<String,Object> node(EObject object) {
        String kind = object.eClass().getName();
        counts.merge(kind,1,Integer::sum);
        Map<String,Object> row = new LinkedHashMap<>();
        row.put("id",identities.get(object)); row.put("kind",kind);
        INode location=NodeModelUtils.getNode(object);
        row.put("span",location==null?null:Map.of("offset",location.getOffset(),"length",location.getLength(),"start_line",location.getStartLine(),"end_line",location.getEndLine()));
        if(object instanceof XtendParameter parameter) { row.put("type", parameter.getParameterType().getIdentifier()); type(parameter.getParameterType().getType()); }
        if(object instanceof XExpression expr) {
            LightweightTypeReference type=resolved.getActualType(expr);
            if(type==null || type.isUnknown()) throw new IllegalStateException("Unresolved expression type: " + identities.get(object));
            row.put("type",type.getIdentifier());
            if(type.getType()!=null)type(type.getType());
        }
        Map<String,Object> fields=new TreeMap<>();
        List<EStructuralFeature> features=new ArrayList<>(object.eClass().getEAllStructuralFeatures());
        features.sort(Comparator.comparing(EStructuralFeature::getName));
        for(EStructuralFeature feature:features) {
            if(feature.isDerived() || feature instanceof EReference ref && ref.isContainer()) continue;
            Object value=object.eGet(feature,true);
            if(object instanceof XAbstractFeatureCall call && feature.getName().equals("feature")) {
                value=resolved.getLinkedFeature(call);
                if(value==null || call.getInvalidFeatureIssueCode()!=null) throw new IllegalStateException("Unresolved or invalid feature: " + identities.get(object));
            }
            if(feature.isMany()) { List<Object> list=new ArrayList<>(); for(Object child:(List<?>)value) list.add(value(object,feature,child)); fields.put(feature.getName(),list); }
            else fields.put(feature.getName(),value(object,feature,value));
        }
        row.put("fields",fields);
        return row;
    }
    private Object value(EObject owner,EStructuralFeature feature,Object value) {
        if(value==null)return null;
        if(feature instanceof EReference ref) {
            EObject target=(EObject)value;
            if(target.eIsProxy()) throw new IllegalStateException("Unresolved " + feature.getName()+": "+EcoreUtil.getURI(target));
            if(ref.isContainment() && target.eContainingFeature()==feature && target.eContainer()==owner)return node(target);
            String id=identities.get(target);
            if(id==null && target instanceof JvmIdentifiableElement symbol) id=symbol(symbol);
            if(id==null) throw new IllegalStateException("Unexported reference " + feature.getName()+": "+EcoreUtil.getURI(target));
            return Map.of("$ref",id);
        }
        if(value instanceof String || value instanceof Boolean || value instanceof Number)return value;
        if(value instanceof org.eclipse.emf.common.util.Enumerator e)return e.getLiteral();
        throw new IllegalStateException("Unsupported attribute " + feature.getName());
    }
    private String symbol(JvmIdentifiableElement symbol) {
        if(symbol.eIsProxy()) throw new IllegalStateException("Unresolved JVM symbol");
        String id=symbol.getIdentifier();
        if(id==null || id.isEmpty()) throw new IllegalStateException("Anonymous JVM symbol");
        if(symbols.containsKey(id))return id;
        Map<String,Object> row=new LinkedHashMap<>(); symbols.put(id,row);
        row.put("kind",symbol.eClass().getName()); row.put("identifier",id); row.put("simple_name",symbol.getSimpleName());
        if(symbol instanceof JvmFeature feature) { row.put("declaring_type",feature.getDeclaringType().getIdentifier()); type(feature.getDeclaringType()); }
        if(symbol instanceof JvmOperation operation) {
            row.put("type",operation.getReturnType().getIdentifier()); row.put("parameter_types",operation.getParameters().stream().map(p->p.getParameterType().getIdentifier()).toList()); row.put("static",operation.isStatic()); row.put("varargs",operation.isVarArgs());
            if (operation.getDeclaringType().getIdentifier().equals("org.omg.sysml.lang.sysml.SysMLPackage")
                && operation.getParameters().isEmpty() && operation.getReturnType().getIdentifier().equals("org.eclipse.emf.ecore.EReference")) {
                try {
                    Class<?> packageClass = Class.forName("org.omg.sysml.lang.sysml.SysMLPackage");
                    Object singleton = packageClass.getField("eINSTANCE").get(null);
                    EStructuralFeature feature = (EStructuralFeature) packageClass.getMethod(operation.getSimpleName()).invoke(singleton);
                    row.put("ecore_feature", feature.getEContainingClass().getName() + "::" + feature.getName());
                } catch (ReflectiveOperationException failure) { throw new IllegalStateException("Cannot resolve Ecore feature", failure); }
            }
        } else if(symbol instanceof JvmField field) {
            row.put("type",field.getType().getIdentifier()); row.put("static",field.isStatic());
            row.put("final",field.isFinal()); row.put("constant",field.isConstant());
            EObject original=associations.getPrimarySourceElement(field);
            if(original instanceof XtendField declaration) {
                row.put("source",sources.get(declaration.eResource()));
                row.put("source_immutable",declaration.isFinal());
                row.put("source_static",declaration.isStatic());
                row.put("source_modifiers",new ArrayList<>(declaration.getModifiers()));
                row.put("initializer",nodeConstant(declaration.getInitialValue()));
                if(field.isStatic() && field.isFinal() && declaration.isStatic() && declaration.isFinal()
                    && declaration.getInitialValue() instanceof XStringLiteral literal)
                    row.put("constant_value",literal.getValue());
            } else if(field.isStatic() && field.isFinal() && field.isConstant()) row.put("constant_value",field.getConstantValue());
        } else if(symbol instanceof JvmFormalParameter parameter) row.put("type",parameter.getParameterType().getIdentifier());
        else if(symbol instanceof JvmType jvmType) type(jvmType);
        else throw new IllegalStateException("Unsupported JVM symbol " + id);
        return id;
    }
    private Object nodeConstant(XExpression expression) {
        if(!identities.containsKey(expression))index(expression,sources.get(expression.eResource())+"#constant@"+NodeModelUtils.getNode(expression).getOffset());
        IResolvedTypes previous = resolved;
        resolved = resolutions.get(expression.eResource());
        if (resolved == null) throw new IllegalStateException("Constant source not resolved");
        try { return node(expression); } finally { resolved = previous; }
    }
    private void type(JvmType type) {
        if(type.eIsProxy())throw new IllegalStateException("Unresolved type");
        String id=type.getIdentifier();
        if(types.containsKey(id))return;
        Map<String,Object> row=new LinkedHashMap<>(); types.put(id,row);
        row.put("kind",type.eClass().getName());
        if(type instanceof JvmDeclaredType declared) {
            row.put("supertypes",declared.getSuperTypes().stream().map(JvmTypeReference::getIdentifier).toList());
            for(JvmTypeReference parent:declared.getSuperTypes())type(parent.getType());
        } else row.put("supertypes",List.of());
    }
}
