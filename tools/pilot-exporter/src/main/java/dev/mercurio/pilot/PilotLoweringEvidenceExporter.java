package dev.mercurio.pilot;

import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.Paths;
import java.time.Instant;
import java.util.ArrayList;
import java.util.Comparator;
import java.util.LinkedHashSet;
import java.util.List;
import java.util.Set;

import org.eclipse.emf.ecore.EAttribute;
import org.eclipse.emf.ecore.EClass;
import org.eclipse.emf.ecore.EPackage;
import org.eclipse.emf.ecore.EReference;
import org.eclipse.emf.ecore.EStructuralFeature;
import org.omg.sysml.interactive.SysMLInteractive;

import com.google.gson.GsonBuilder;
import com.google.gson.annotations.SerializedName;

public final class PilotLoweringEvidenceExporter {
    private PilotLoweringEvidenceExporter() {
    }

    public static void main(String[] args) throws Exception {
        if (args.length < 1) {
            System.err.println("Usage: PilotLoweringEvidenceExporter <output-json>");
            System.exit(2);
        }

        Path outputPath = Paths.get(args[0]).toAbsolutePath().normalize();

        System.setProperty("org.eclipse.emf.common.util.ReferenceClearingQueue", "false");
        SysMLInteractive.getInstance();

        EvidenceDocument document = exportEvidence();
        if (outputPath.getParent() != null) {
            Files.createDirectories(outputPath.getParent());
        }
        Files.writeString(
            outputPath,
            new GsonBuilder().disableHtmlEscaping().setPrettyPrinting().create().toJson(document),
            StandardCharsets.UTF_8
        );
    }

    private static EvidenceDocument exportEvidence() {
        EvidenceDocument document = new EvidenceDocument();
        document.source = new EvidenceSource();
        document.source.exporter_version = PilotLoweringEvidenceExporter.class.getPackage().getImplementationVersion();
        document.source.captured_at_utc = Instant.now().toString();
        document.grammar_rules = new ArrayList<>();
        document.ecore_classes = exportEcoreClasses();
        document.transform_observations = new ArrayList<>();
        return document;
    }

    private static List<EcoreClassEvidence> exportEcoreClasses() {
        List<EcoreClassEvidence> classes = new ArrayList<>();
        Set<String> seenPackages = new LinkedHashSet<>();

        for (Object value : EPackage.Registry.INSTANCE.values()) {
            if (!(value instanceof EPackage ePackage)) {
                continue;
            }
            if (!isPilotLanguagePackage(ePackage)) {
                continue;
            }
            if (!seenPackages.add(packageKey(ePackage))) {
                continue;
            }

            for (Object classifier : ePackage.getEClassifiers()) {
                if (classifier instanceof EClass eClass) {
                    classes.add(toClassEvidence(ePackage, eClass));
                }
            }
        }

        classes.sort(
            Comparator.comparing((EcoreClassEvidence entry) -> entry.packageName)
                .thenComparing(entry -> entry.name)
        );
        return classes;
    }

    private static boolean isPilotLanguagePackage(EPackage ePackage) {
        String name = valueOrEmpty(ePackage.getName()).toLowerCase();
        String nsUri = valueOrEmpty(ePackage.getNsURI()).toLowerCase();
        return name.contains("sysml")
            || name.contains("kerml")
            || nsUri.contains("sysml")
            || nsUri.contains("kerml");
    }

    private static String packageKey(EPackage ePackage) {
        return valueOrEmpty(ePackage.getNsURI()) + "#" + valueOrEmpty(ePackage.getName());
    }

    private static EcoreClassEvidence toClassEvidence(EPackage ePackage, EClass eClass) {
        EcoreClassEvidence evidence = new EcoreClassEvidence();
        evidence.packageName = displayPackageName(ePackage);
        evidence.name = eClass.getName();
        evidence.abstract_class = eClass.isAbstract();
        evidence.supertypes = new ArrayList<>();
        for (EClass supertype : eClass.getESuperTypes()) {
            evidence.supertypes.add(qualifiedClassName(supertype));
        }
        evidence.supertypes.sort(String::compareTo);

        evidence.structural_features = new ArrayList<>();
        for (EStructuralFeature feature : eClass.getEStructuralFeatures()) {
            evidence.structural_features.add(toFeatureEvidence(feature));
        }
        evidence.structural_features.sort(Comparator.comparing(feature -> feature.name));
        return evidence;
    }

    private static EcoreFeatureEvidence toFeatureEvidence(EStructuralFeature feature) {
        EcoreFeatureEvidence evidence = new EcoreFeatureEvidence();
        evidence.name = feature.getName();
        evidence.kind = feature instanceof EReference ? "reference" : "attribute";
        evidence.lower_bound = feature.getLowerBound();
        evidence.upper_bound = feature.getUpperBound();
        evidence.derived = feature.isDerived();
        evidence.transientFeature = feature.isTransient();
        evidence.volatileFeature = feature.isVolatile();

        if (feature instanceof EReference reference) {
            evidence.containment = reference.isContainment();
            evidence.target = qualifiedClassName(reference.getEReferenceType());
        } else if (feature instanceof EAttribute attribute) {
            evidence.containment = false;
            evidence.target = attribute.getEAttributeType() == null
                ? null
                : attribute.getEAttributeType().getName();
        }

        return evidence;
    }

    private static String qualifiedClassName(EClass eClass) {
        if (eClass == null) {
            return null;
        }
        EPackage ePackage = eClass.getEPackage();
        return displayPackageName(ePackage) + "::" + eClass.getName();
    }

    private static String displayPackageName(EPackage ePackage) {
        if (ePackage == null) {
            return "";
        }
        String name = valueOrEmpty(ePackage.getName());
        if (name.equalsIgnoreCase("sysml")) {
            return "SysML";
        }
        if (name.equalsIgnoreCase("kerml")) {
            return "KerML";
        }
        return name;
    }

    private static String valueOrEmpty(String value) {
        return value == null ? "" : value;
    }

    private static final class EvidenceDocument {
        private EvidenceSource source;
        private List<Object> grammar_rules;
        private List<EcoreClassEvidence> ecore_classes;
        private List<Object> transform_observations;
    }

    private static final class EvidenceSource {
        private String pilot_source_id;
        private String exporter_version;
        private String captured_at_utc;
    }

    private static final class EcoreClassEvidence {
        @SerializedName("package")
        private String packageName;
        private String name;
        private List<String> supertypes;
        private List<EcoreFeatureEvidence> structural_features;
        private boolean abstract_class;
    }

    private static final class EcoreFeatureEvidence {
        private String name;
        private String kind;
        private String target;
        private int lower_bound;
        private int upper_bound;
        private boolean containment;
        private boolean derived;
        @SerializedName("transient")
        private boolean transientFeature;
        @SerializedName("volatile")
        private boolean volatileFeature;
    }
}
