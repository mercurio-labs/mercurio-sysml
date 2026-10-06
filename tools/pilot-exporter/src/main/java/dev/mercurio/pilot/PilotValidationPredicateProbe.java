package dev.mercurio.pilot;

import com.google.gson.GsonBuilder;
import java.lang.reflect.Proxy;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.IdentityHashMap;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Locale;
import java.util.Map;
import org.eclipse.emf.ecore.EObject;
import org.eclipse.emf.ecore.EStructuralFeature;
import org.eclipse.xtext.validation.ValidationMessageAcceptor;
import org.omg.kerml.xtext.validation.KerMLValidator;
import org.omg.sysml.xtext.validation.SysMLValidator;
import org.omg.sysml.lang.sysml.ReferenceUsage;
import org.omg.sysml.lang.sysml.EnumerationDefinition;
import org.omg.sysml.lang.sysml.SysMLFactory;
import org.omg.sysml.lang.sysml.Feature;
import org.omg.sysml.lang.sysml.Type;
import org.omg.sysml.adapter.FeatureAdapter;
import org.eclipse.emf.common.util.BasicEList;
import org.eclipse.emf.common.util.EList;
import org.omg.sysml.lang.sysml.AnnotatingElement;
import org.omg.sysml.lang.sysml.Annotation;
import org.omg.sysml.lang.sysml.Element;
import org.omg.sysml.lang.sysml.Import;
import org.omg.sysml.lang.sysml.Namespace;
import org.omg.sysml.lang.sysml.VisibilityKind;

/**
 * Executes unmodified compiled Pilot predicates with controlled getter values.
 * These proxies deliberately do not assert that each input is a valid EMF graph.
 * Every unconfigured getter fails so a changed predicate cannot gain fake defaults.
 */
public final class PilotValidationPredicateProbe {
    private PilotValidationPredicateProbe() {}

    private static Map<String, Object> object(Object... entries) {
        Map<String, Object> result = new LinkedHashMap<>();
        for (int i = 0; i < entries.length; i += 2) {
            result.put((String) entries[i], entries[i + 1]);
        }
        return result;
    }

    private static <T> T controlled(Class<T> type, String id, Map<String, Object> getters) {
        return type.cast(Proxy.newProxyInstance(type.getClassLoader(), new Class<?>[] { type },
            (self, method, arguments) -> {
                if (method.getDeclaringClass() == Object.class) {
                    return switch (method.getName()) {
                        case "toString" -> id;
                        case "hashCode" -> System.identityHashCode(self);
                        case "equals" -> self == arguments[0];
                        default -> throw new AssertionError(method);
                    };
                }
                if (method.getParameterCount() == 0 && getters.containsKey(method.getName())) {
                    return getters.get(method.getName());
                }
                throw new AssertionError("Unexpected controlled-model access: " + id + "." + method);
            }));
    }

    private static final class Capture implements ValidationMessageAcceptor {
        private final IdentityHashMap<EObject, String> subjects = new IdentityHashMap<>();
        private final List<Map<String, Object>> diagnostics = new ArrayList<>();

        Capture(EObject subject, String id) {
            subjects.put(subject, id);
        }

        private void record(String severity, String message, EObject subject,
                EStructuralFeature feature, int index, String code, String... data) {
            if (!subjects.containsKey(subject)) {
                throw new AssertionError("Unexpected diagnostic subject");
            }
            String featureName = feature == null ? null
                : feature.getEContainingClass().getName() + "::" + feature.getName();
            diagnostics.add(object("severity", severity, "code", code, "message", message,
                "subject", subjects.get(subject), "feature", featureName, "index", index,
                "data", data == null ? null : Arrays.asList(data)));
        }

        @Override public void acceptError(String message, EObject subject, EStructuralFeature feature,
                int index, String code, String... data) {
            record("error", message, subject, feature, index, code, data);
        }
        @Override public void acceptWarning(String message, EObject subject, EStructuralFeature feature,
                int index, String code, String... data) {
            record("warning", message, subject, feature, index, code, data);
        }
        @Override public void acceptInfo(String message, EObject subject, EStructuralFeature feature,
                int index, String code, String... data) {
            record("info", message, subject, feature, index, code, data);
        }
        @Override public void acceptError(String message, EObject subject, int offset, int length,
                String code, String... data) {
            throw new AssertionError("Unexpected offset diagnostic overload");
        }
        @Override public void acceptWarning(String message, EObject subject, int offset, int length,
                String code, String... data) {
            throw new AssertionError("Unexpected offset diagnostic overload");
        }
        @Override public void acceptInfo(String message, EObject subject, int offset, int length,
                String code, String... data) {
            throw new AssertionError("Unexpected offset diagnostic overload");
        }
    }

    public static void main(String[] args) throws Exception {
        if (args.length != 1) {
            throw new IllegalArgumentException("Usage: PilotValidationPredicateProbe <output-json>");
        }
        List<Map<String, Object>> cases = new ArrayList<>();
        for (boolean namespacePresent : new boolean[] { false, true }) {
            for (boolean ownerPresent : new boolean[] { false, true }) {
                for (VisibilityKind visibility : new VisibilityKind[] {
                        VisibilityKind.PUBLIC, VisibilityKind.PROTECTED, VisibilityKind.PRIVATE }) {
                    Element owner = controlled(Element.class, "namespace-owner", object());
                    Namespace namespace = controlled(Namespace.class, "namespace",
                        object("getOwner", ownerPresent ? owner : null));
                    Import subject = controlled(Import.class, "import",
                        object("getImportOwningNamespace", namespacePresent ? namespace : null,
                            "getVisibility", visibility));
                    Capture capture = new Capture(subject, "import");
                    KerMLValidator validator = new KerMLValidator();
                    validator.setMessageAcceptor(capture);
                    validator.checkImport(subject);
                    String visibilityName = visibility.name().toLowerCase(Locale.ROOT);
                    cases.add(object("id", "import-namespace-" + namespacePresent + "-owner-"
                        + ownerPresent + "-" + visibilityName, "method", "checkImport", "inputs",
                        object("namespace_present", namespacePresent, "namespace_owner_present",
                            ownerPresent, "visibility", visibilityName), "diagnostics", capture.diagnostics));
                }
            }
        }
        for (int mask = 0; mask < 8; mask++) {
            AnnotatingElement owned = controlled(AnnotatingElement.class, "owned-annotating", object());
            AnnotatingElement owning = controlled(AnnotatingElement.class, "owning-annotating", object());
            Element annotated = controlled(Element.class, "owning-annotated", object());
            Annotation subject = controlled(Annotation.class, "annotation",
                object("getOwnedAnnotatingElement", (mask & 1) != 0 ? owned : null,
                    "getOwningAnnotatingElement", (mask & 2) != 0 ? owning : null,
                    "getOwningAnnotatedElement", (mask & 4) != 0 ? annotated : null));
            Capture capture = new Capture(subject, "annotation");
            KerMLValidator validator = new KerMLValidator();
            validator.setMessageAcceptor(capture);
            validator.checkAnnotation(subject);
            cases.add(object("id", "annotation-mask-" + mask, "method", "checkAnnotation", "inputs",
                object("owned_annotating_element_present", (mask & 1) != 0,
                    "owning_annotating_element_present", (mask & 2) != 0,
                    "owning_annotated_element_present", (mask & 4) != 0),
                "diagnostics", capture.diagnostics));
        }
        for (boolean flag : new boolean[] {false, true}) {
            ReferenceUsage reference = controlled(ReferenceUsage.class, "reference", object("isReference", flag));
            Capture capture = new Capture(reference, "reference");
            SysMLValidator validator = new SysMLValidator();
            validator.setMessageAcceptor(capture);
            validator.checkReferenceUsage(reference);
            cases.add(object("id", "reference-" + flag, "method", "checkReferenceUsage",
                "inputs", object("is_reference", flag), "diagnostics", capture.diagnostics));
            EnumerationDefinition enumeration = controlled(EnumerationDefinition.class, "enumeration", object("isVariation", flag));
            capture = new Capture(enumeration, "enumeration");
            validator.setMessageAcceptor(capture);
            validator.checkEnumerationDefinition(enumeration);
            cases.add(object("id", "enumeration-" + flag, "method", "checkEnumerationDefinition",
                "inputs", object("is_variation", flag), "diagnostics", capture.diagnostics));
        }
        for (String context : List.of("EnumerationUsage", "AnalysisCaseUsage", "VerificationCaseUsage",
                "UseCaseUsage", "RenderingUsage", "ViewpointUsage", "MetadataUsage")) {
            String required = context.equals("MetadataUsage") ? "Metaclass" : context.replace("Usage", "Definition");
            for (String combination : List.of("empty", "matching", "wrong", "multiple")) {
                Feature subject = (Feature) SysMLFactory.class.getMethod("create" + context).invoke(SysMLFactory.eINSTANCE);
                EList<Type> controlledTypes = new BasicEList<>();
                List<String> kinds = new ArrayList<>();
                if (!combination.equals("empty")) {
                    String type = combination.equals("wrong") ? "Class" : required;
                    controlledTypes.add((Type) SysMLFactory.class.getMethod("create" + type).invoke(SysMLFactory.eINSTANCE));
                    kinds.add(type);
                }
                if (combination.equals("multiple")) {
                    controlledTypes.add((Type) SysMLFactory.class.getMethod("create" + required).invoke(SysMLFactory.eINSTANCE));
                    kinds.add(required);
                }
                subject.eAdapters().clear();
                subject.eAdapters().add(new FeatureAdapter(subject) {
                    @Override public EList<Type> getAllTypes() { return controlledTypes; }
                });
                Capture capture = new Capture(subject, "typed-usage");
                SysMLValidator validator = new SysMLValidator();
                validator.setMessageAcceptor(capture).getState().currentObject = subject;
                String method = "check" + context;
                SysMLValidator.class.getMethod(method, Class.forName("org.omg.sysml.lang.sysml." + context)).invoke(validator, subject);
                cases.add(object("id", context + "-" + combination, "method", method,
                    "inputs", object("type_kinds", kinds), "diagnostics", capture.diagnostics));
            }
        }
        // Real model construction independently qualifies the implicit enumeration default.
        if (!SysMLFactory.eINSTANCE.createEnumerationDefinition().isVariation())
            throw new AssertionError("Enumeration variation default changed");
        String json = new GsonBuilder().serializeNulls().disableHtmlEscaping().setPrettyPrinting()
            .create().toJson(object("cases", cases)) + "\n";
        Files.writeString(Path.of(args[0]), json, StandardCharsets.UTF_8);
    }
}
