package dev.mercurio.pilot;

import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.Paths;
import java.time.Instant;
import java.util.ArrayList;
import java.util.List;
import java.util.UUID;

import org.eclipse.emf.common.util.TreeIterator;
import org.eclipse.emf.ecore.EObject;
import org.eclipse.emf.ecore.resource.Resource;
import org.eclipse.emf.ecore.resource.ResourceSet;
import org.eclipse.xtext.EcoreUtil2;
import org.omg.sysml.interactive.SysMLInteractive;
import org.omg.sysml.lang.sysml.Element;
import org.omg.sysml.util.ElementUtil;
import org.omg.sysml.util.traversal.Traversal;
import org.omg.sysml.util.traversal.facade.impl.JsonElementProcessingFacade;

import com.google.gson.Gson;
import com.google.gson.GsonBuilder;
import com.google.gson.JsonArray;
import com.google.gson.JsonElement;
import com.google.gson.JsonObject;
import com.google.gson.JsonParser;

public final class PilotJsonInteropProbe {
    private static final Gson JSON = new GsonBuilder().disableHtmlEscaping().setPrettyPrinting().create();

    private PilotJsonInteropProbe() {
    }

    public static void main(String[] args) throws Exception {
        if (args.length >= 1 && "--read-json".equals(args[0])) {
            readJson(args);
            return;
        }
        if (args.length >= 1 && "--export-api-json".equals(args[0])) {
            exportApiJson(args);
            return;
        }

        System.err.println(
            "Usage: PilotJsonInteropProbe --read-json <input-json> <output-report-json>\n"
                + "   or: PilotJsonInteropProbe --export-api-json <library-root> <output-json> <model-file> [support-file ...]"
        );
        System.exit(2);
    }

    private static void readJson(String[] args) throws Exception {
        if (args.length != 3) {
            System.err.println("Usage: PilotJsonInteropProbe --read-json <input-json> <output-report-json>");
            System.exit(2);
        }

        Path inputPath = Paths.get(args[1]).toAbsolutePath().normalize();
        Path outputPath = Paths.get(args[2]).toAbsolutePath().normalize();
        JsonElement root = JsonParser.parseString(Files.readString(inputPath, StandardCharsets.UTF_8));
        JsonArray elements = elementsArray(root);

        ProbeReport report = new ProbeReport();
        report.status = "ok";
        report.input_json = inputPath.toString();
        report.checked_at_utc = Instant.now().toString();
        report.pilot_version = pilotVersion();
        report.element_count = elements.size();
        report.diagnostics = new ArrayList<>();

        for (int i = 0; i < elements.size(); i += 1) {
            JsonElement rawElement = elements.get(i);
            if (!rawElement.isJsonObject()) {
                report.diagnostics.add(diagnostic("error", "pilot_json.element.shape", "Element must be a JSON object", "elements[" + i + "]"));
                continue;
            }
            validateElement(rawElement.getAsJsonObject(), "elements[" + i + "]", report.diagnostics);
        }

        if (report.diagnostics.stream().anyMatch(diagnostic -> "error".equals(diagnostic.severity))) {
            report.status = "error";
        }
        writeJson(outputPath, report);
    }

    private static JsonArray elementsArray(JsonElement root) {
        if (root.isJsonArray()) {
            return root.getAsJsonArray();
        }
        if (root.isJsonObject()) {
            JsonObject object = root.getAsJsonObject();
            JsonElement elements = object.get("elements");
            if (elements != null && elements.isJsonArray()) {
                return elements.getAsJsonArray();
            }
        }
        throw new IllegalArgumentException("SysML JSON must be an array or an object with an elements array");
    }

    private static void validateElement(JsonObject object, String path, List<ProbeDiagnostic> diagnostics) {
        JsonElement id = object.get("@id");
        if (id == null || !id.isJsonPrimitive() || !id.getAsJsonPrimitive().isString()) {
            diagnostics.add(diagnostic("error", "pilot_json.element.missing_id", "Element is missing string @id", path));
        } else {
            validateUuid(id.getAsString(), path + ".@id", diagnostics);
        }

        JsonElement type = object.get("@type");
        if (type == null || !type.isJsonPrimitive() || !type.getAsJsonPrimitive().isString()) {
            diagnostics.add(diagnostic("error", "pilot_json.element.missing_type", "Element is missing string @type", path));
        }

        for (String key : object.keySet()) {
            if ("@id".equals(key) || "@type".equals(key) || "xMercurio".equals(key)) {
                continue;
            }
            validateReferences(object.get(key), path + "." + key, diagnostics);
        }
    }

    private static void validateReferences(JsonElement value, String path, List<ProbeDiagnostic> diagnostics) {
        if (value == null || value.isJsonNull() || value.isJsonPrimitive()) {
            return;
        }
        if (value.isJsonArray()) {
            JsonArray array = value.getAsJsonArray();
            for (int i = 0; i < array.size(); i += 1) {
                validateReferences(array.get(i), path + "[" + i + "]", diagnostics);
            }
            return;
        }
        JsonObject object = value.getAsJsonObject();
        JsonElement id = object.get("@id");
        if (id != null) {
            if (!id.isJsonPrimitive() || !id.getAsJsonPrimitive().isString()) {
                diagnostics.add(diagnostic("error", "pilot_json.reference.invalid_id", "Reference @id must be a string", path));
            } else {
                validateUuid(id.getAsString(), path + ".@id", diagnostics);
            }
            return;
        }
        for (String key : object.keySet()) {
            validateReferences(object.get(key), path + "." + key, diagnostics);
        }
    }

    private static void validateUuid(String value, String path, List<ProbeDiagnostic> diagnostics) {
        try {
            UUID.fromString(value);
        } catch (IllegalArgumentException ex) {
            diagnostics.add(diagnostic("error", "pilot_json.uuid.invalid", "Value is not a Java UUID: " + value, path));
        }
    }

    private static void exportApiJson(String[] args) throws Exception {
        if (args.length < 4) {
            System.err.println("Usage: PilotJsonInteropProbe --export-api-json <library-root> <output-json> <model-file> [support-file ...]");
            System.exit(2);
        }

        Path libraryRoot = Paths.get(args[1]).toAbsolutePath().normalize();
        Path outputPath = Paths.get(args[2]).toAbsolutePath().normalize();
        List<Path> inputFiles = new ArrayList<>();
        for (int i = 3; i < args.length; i += 1) {
            inputFiles.add(Paths.get(args[i]).toAbsolutePath().normalize());
        }

        System.setProperty("org.eclipse.emf.common.util.ReferenceClearingQueue", "false");
        SysMLInteractive interactive = SysMLInteractive.getInstance();
        interactive.getLibraryIndexCache().setIndexDisabled(true);
        interactive.loadLibrary(libraryRoot.toString());
        interactive.setVerbose(false);

        List<Resource> inputResources = new ArrayList<>();
        for (Path inputFile : inputFiles) {
            Resource resource = interactive.readResource(inputFile.toString());
            interactive.addInputResource(resource);
            inputResources.add(resource);
        }

        ResourceSet resourceSet = interactive.getResourceSet();
        resourceSet.getResources().forEach(resource -> EcoreUtil2.resolveLazyCrossReferences(resource, null));
        interactive.resolveAllInputResources();
        ElementUtil.transformAll(resourceSet, true);
        resourceSet.getResources().forEach(resource -> EcoreUtil2.resolveLazyCrossReferences(resource, null));

        JsonElementProcessingFacade facade = new JsonElementProcessingFacade();
        facade.setTraversal(new Traversal(facade));
        for (Resource resource : inputResources) {
            TreeIterator<EObject> iterator = resource.getAllContents();
            while (iterator.hasNext()) {
                EObject object = iterator.next();
                if (object instanceof Element element && element.eContainer() == null) {
                    facade.getTraversal().visit(element);
                }
            }
        }

        writeJson(outputPath, facade.toJsonTree(false));
    }

    private static ProbeDiagnostic diagnostic(String severity, String code, String message, String path) {
        ProbeDiagnostic diagnostic = new ProbeDiagnostic();
        diagnostic.severity = severity;
        diagnostic.code = code;
        diagnostic.message = message;
        diagnostic.path = path;
        return diagnostic;
    }

    private static void writeJson(Path outputPath, Object value) throws Exception {
        if (outputPath.getParent() != null) {
            Files.createDirectories(outputPath.getParent());
        }
        Files.writeString(outputPath, JSON.toJson(value), StandardCharsets.UTF_8);
    }

    private static String pilotVersion() {
        Package pkg = SysMLInteractive.class.getPackage();
        if (pkg == null) {
            return null;
        }
        String implementationVersion = pkg.getImplementationVersion();
        if (implementationVersion != null && !implementationVersion.isBlank()) {
            return implementationVersion;
        }
        String specificationVersion = pkg.getSpecificationVersion();
        if (specificationVersion != null && !specificationVersion.isBlank()) {
            return specificationVersion;
        }
        return null;
    }

    private static final class ProbeReport {
        private String status;
        private String input_json;
        private String checked_at_utc;
        private String pilot_version;
        private int element_count;
        private List<ProbeDiagnostic> diagnostics;
    }

    private static final class ProbeDiagnostic {
        private String severity;
        private String code;
        private String message;
        private String path;
    }
}
