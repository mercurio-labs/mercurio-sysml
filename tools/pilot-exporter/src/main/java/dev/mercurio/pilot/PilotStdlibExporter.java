package dev.mercurio.pilot;

import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.Paths;
import java.time.Instant;
import java.util.ArrayList;
import java.util.Collection;
import java.util.Comparator;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Locale;
import java.util.Map;
import java.util.Objects;
import java.util.Set;
import java.util.TreeSet;
import java.util.UUID;

import org.eclipse.emf.common.util.TreeIterator;
import org.eclipse.emf.ecore.EObject;
import org.eclipse.emf.ecore.EAttribute;
import org.eclipse.emf.common.util.Enumerator;
import org.eclipse.emf.ecore.resource.Resource;
import org.eclipse.emf.ecore.resource.ResourceSet;
import org.eclipse.xtext.EcoreUtil2;
import org.eclipse.xtext.nodemodel.ICompositeNode;
import org.eclipse.xtext.nodemodel.util.NodeModelUtils;
import org.omg.sysml.interactive.SysMLInteractive;
import org.omg.sysml.lang.sysml.Documentation;
import org.omg.sysml.lang.sysml.Membership;
import org.omg.sysml.lang.sysml.VisibilityKind;
import org.omg.sysml.lang.sysml.Element;
import org.omg.sysml.lang.sysml.Feature;
import org.omg.sysml.lang.sysml.Namespace;
import org.omg.sysml.lang.sysml.Relationship;
import org.omg.sysml.lang.sysml.Specialization;
import org.omg.sysml.lang.sysml.Type;
import org.omg.sysml.util.ElementUtil;
import org.omg.sysml.util.TypeUtil;
import org.omg.sysml.util.FeatureUtil;

import com.google.gson.GsonBuilder;

public final class PilotStdlibExporter {
    private static final String KERNEL_LIBRARIES = "Kernel Libraries";
    private static final String SYSTEMS_LIBRARY = "Systems Library";
    private static final String DOMAIN_LIBRARIES = "Domain Libraries";

    private PilotStdlibExporter() {
    }

    public static void main(String[] args) throws Exception {
        if (args.length < 2) {
            System.err.println("Usage: PilotStdlibExporter <library-root> <output-json>");
            System.exit(2);
        }

        Path libraryRoot = Paths.get(args[0]).toAbsolutePath().normalize();
        Path outputPath = Paths.get(args[1]).toAbsolutePath().normalize();

        System.setProperty("org.eclipse.emf.common.util.ReferenceClearingQueue", "false");

        SysMLInteractive interactive = SysMLInteractive.getInstance();
        interactive.getLibraryIndexCache().setIndexDisabled(true);
        interactive.loadLibrary(libraryRoot.toString());

        ResourceSet resourceSet = interactive.getResourceSet();
        resourceSet.getResources().forEach(resource -> EcoreUtil2.resolveLazyCrossReferences(resource, null));
        ElementUtil.transformAll(resourceSet, false);

        for (Resource resource : resourceSet.getResources()) {
            if (!resource.getErrors().isEmpty()) {
                throw new IllegalStateException("Pilot library parse errors in " + resource.getURI()
                    + ": " + resource.getErrors().subList(0, Math.min(3, resource.getErrors().size())));
            }
        }

        ExportDocument document = exportDocument(libraryRoot, resourceSet);
        if (outputPath.getParent() != null) {
            Files.createDirectories(outputPath.getParent());
        }
        Files.writeString(
            outputPath,
            new GsonBuilder().disableHtmlEscaping().setPrettyPrinting().create().toJson(document),
            StandardCharsets.UTF_8
        );
    }

    private static ExportDocument exportDocument(Path libraryRoot, ResourceSet resourceSet) {
        Map<String, ExportElement> elements = new LinkedHashMap<>();
        List<RelationshipKey> relationships = new ArrayList<>();
        Set<String> libraryFiles = new TreeSet<>();
        boolean hasModeledContent = false;

        for (Resource resource : resourceSet.getResources()) {
            Path resourcePath = resourcePath(resource);
            if (resourcePath == null || !resourcePath.startsWith(libraryRoot)) {
                continue;
            }

            String relativePath = normalizeRelativePath(libraryRoot.relativize(resourcePath));
            String libraryGroup = libraryGroup(relativePath);
            if (libraryGroup == null) {
                continue;
            }
            libraryFiles.add(relativePath);

            TreeIterator<EObject> iterator = resource.getAllContents();
            while (iterator.hasNext()) {
                EObject object = iterator.next();
                if (!(object instanceof Element element)) {
                    continue;
                }

                String qualifiedName = exportIdentityOf(element, libraryRoot);
                if (qualifiedName == null) {
                    continue;
                }

                // Parsing a missing or damaged library can leave only unnamed
                // resource Namespace wrappers, which are not library content.
                if (!(element instanceof Namespace && element.eContainer() == null
                    && clean(element.getDeclaredName()) == null
                    && clean(element.getDeclaredShortName()) == null)) {
                    hasModeledContent = true;
                }
                elements.computeIfAbsent(
                    qualifiedName,
                    ignored -> toExportElement(element, qualifiedName, libraryGroup, relativePath)
                );
                collectRelationships(element, qualifiedName, relationships, libraryRoot);
            }
        }

        List<ExportElement> exportElements = new ArrayList<>(elements.values());
        exportElements.sort(Comparator.comparing(element -> element.qualified_name));

        List<ExportRelationship> exportRelationships = relationships.stream()
            .filter(relationship -> elements.containsKey(relationship.source))
            .filter(relationship -> elements.containsKey(relationship.target))
            .map(relationship -> new ExportRelationship(relationship.source, relationship.relation, relationship.target))
            .toList();

        // Check the final projection, after dangling relationship filtering.
        // This is a structural minimum, not a version-specific size threshold
        // or a claim that every expected standard-library declaration exists.
        if (!hasModeledContent || exportRelationships.isEmpty()) {
            throw new IllegalStateException("Incomplete Pilot library export: expected modeled content beyond "
                + "anonymous resource roots and retained relationships; observed " + libraryFiles.size()
                + " library resources, " + exportElements.size() + " elements, modeled content="
                + hasModeledContent + ", " + exportRelationships.size() + " relationships. "
                + "Check that the library sources were loaded and transformed successfully.");
        }

        ExportMetadata metadata = new ExportMetadata();
        metadata.element_count = exportElements.size();
        metadata.relationship_count = exportRelationships.size();
        metadata.library_root = libraryRoot.toString();
        metadata.library_files = new ArrayList<>(libraryFiles);
        metadata.exported_at_utc = Instant.now().toString();
        metadata.pilot_version = pilotVersion();
        metadata.observed_ecore_defaults_v1 = true;

        ExportDocument document = new ExportDocument();
        document.metadata = metadata;
        document.elements = exportElements;
        document.relationships = exportRelationships;
        return document;
    }

    private static ExportElement toExportElement(
        Element element,
        String qualifiedName,
        String libraryGroup,
        String relativePath
    ) {
        ExportElement export = new ExportElement();
        export.qualified_name = qualifiedName;
        export.kind = element.eClass().getName();
        export.library_group = libraryGroup;
        export.source = new ExportSource(relativePath, startLineOf(element), endLineOf(element));
        export.documentation = documentationOf(element);
        export.properties = propertiesOf(element);
        return export;
    }

    private static List<ExportDocumentationBlock> documentationOf(Element element) {
        List<ExportDocumentationBlock> docs = new ArrayList<>();
        for (Documentation documentation : element.getDocumentation()) {
            String body = clean(documentation.getBody());
            if (body != null) {
                docs.add(new ExportDocumentationBlock("comment", body));
            }
        }
        docs.sort(Comparator.comparing(block -> block.text));
        return docs;
    }

    private static Map<String, Object> propertiesOf(Element element) {
        Map<String, Object> properties = new LinkedHashMap<>();
        putIfPresent(properties, "declared_name", clean(element.getDeclaredName()));
        putIfPresent(properties, "declared_short_name", clean(element.getDeclaredShortName()));
        putIfPresent(properties, "name", clean(element.getName()));
        putIfPresent(properties, "short_name", clean(element.getShortName()));
        properties.put("is_library_element", element.isLibraryElement());
        if (element.eResource() != null && qualifiedNameOf(element) == null) {
            properties.put("resource_fragment", element.eResource().getURIFragment(element));
        }
        if (element instanceof Membership membership) {
            properties.put("member_element_present", membership.getMemberElement() != null);
            properties.put("membership_owning_namespace_present",
                membership.getMembershipOwningNamespace() != null);
            putIfPresent(properties, "member_name", clean(membership.getMemberName()));
            putIfPresent(properties, "member_short_name", clean(membership.getMemberShortName()));
            if (membership.getVisibility() != null) {
                properties.put("visibility", membership.getVisibility().toString().toLowerCase(Locale.ROOT));
            }
        }
        Membership owningMembership = element.getOwningMembership();
        if (owningMembership != null && owningMembership.getVisibility() != null) {
            properties.put("owning_membership_visibility",
                owningMembership.getVisibility().toString().toLowerCase(Locale.ROOT));
        }

        if (element instanceof Feature feature) {
            properties.put("is_abstract", feature.isAbstract());
            properties.put("is_derived", feature.isDerived());
            properties.put("is_end", feature.isEnd());
            properties.put("is_ordered", feature.isOrdered());
            properties.put("is_unique", feature.isUnique());
            properties.put("is_variable", feature.isVariable());
            if (feature.getDirection() != null) {
                properties.put("direction", feature.getDirection().toString().toLowerCase(Locale.ROOT));
            }
        } else if (element instanceof Type type) {
            properties.put("is_abstract", type.isAbstract());
        } else if (element instanceof Relationship relationship) {
            properties.put("is_implied", relationship.isImplied());
        }

        // The effective Ecore contract identifies explicit defaults, while
        // generated Pilot getters may override them. Export their observed
        // values from attached library objects; the native importer promotes
        // these values instead of guessing from Ecore literals.
        for (EAttribute attribute : element.eClass().getEAllAttributes()) {
            if (attribute.getDefaultValueLiteral() == null) { continue; }
            Object value;
            try {
                value = element.eGet(attribute);
            } catch (RuntimeException error) {
                throw new IllegalStateException("Cannot observe " + qualifiedNameOf(element)
                    + " (" + element.eClass().getName() + ")." + attribute.getName(), error);
            }
            if (value instanceof Enumerator enumerator) value = enumerator.getLiteral();
            if (!(value instanceof String || value instanceof Number || value instanceof Boolean)) {
                throw new IllegalStateException("Unsupported explicit-default attribute "
                    + element.eClass().getName() + "." + attribute.getName());
            }
            String name = normalizeAttributeName(attribute.getName());
            Object previous = properties.put(name, value);
            if (previous != null && !Objects.equals(previous, value)) {
                throw new IllegalStateException("Conflicting Pilot attribute projection "
                    + element.eClass().getName() + "." + name);
            }
        }

        if (element instanceof Namespace namespace) {
            Map<String, String> visible = new java.util.TreeMap<>();
            List<Membership> candidates = new ArrayList<>(namespace.getOwnedMembership());
            if (!namespace.getOwnedImport().isEmpty()) {
                candidates.addAll(new ArrayList<>(namespace.getImportedMembership()));
            }
            for (Membership membership : candidates) {
                if (namespace.visibilityOf(membership) != VisibilityKind.PUBLIC) {
                    continue;
                }
                String target = qualifiedNameOf(membership.getMemberElement());
                if (target == null) { continue; }
                for (String name : new String[] {membership.getMemberName(), membership.getMemberShortName()}) {
                    name = clean(name);
                    // Direct owned names already have canonical IDs. Export only
                    // aliases and imported names, after Pilot checks visibility.
                    if (name != null && !target.equals(qualifiedNameOf(namespace) + "::" + name)
                        && namespace.resolveVisible(name) == membership) {
                        visible.put(name, target);
                    }
                }
            }
            if (!visible.isEmpty()) { properties.put("public_memberships", visible); }
        }

        properties.values().removeIf(Objects::isNull);
        return properties;
    }

    private static void collectRelationships(
        Element element,
        String sourceQualifiedName,
        List<RelationshipKey> relationships,
        Path libraryRoot
    ) {
        addRelationship(relationships, sourceQualifiedName, "owner", exportIdentityOf(element.getOwner(), libraryRoot));
        addRelationship(relationships, sourceQualifiedName, "owning_membership", exportIdentityOf(element.getOwningMembership(), libraryRoot));

        if (element instanceof Membership membership) {
            addRelationship(relationships, sourceQualifiedName, "member_element", exportIdentityOf(membership.getMemberElement(), libraryRoot));
            addRelationship(relationships, sourceQualifiedName, "membership_owning_namespace", exportIdentityOf(membership.getMembershipOwningNamespace(), libraryRoot));
        }

        if (element instanceof Namespace namespace) {
            addRelationships(relationships, sourceQualifiedName, "members", namespace.getOwnedMember(), libraryRoot);
            for (Membership membership : namespace.getOwnedMembership()) {
                addRelationship(relationships, sourceQualifiedName, "owned_membership", exportIdentityOf(membership, libraryRoot));
            }
        }

        if (element instanceof Type type) {
            // Implicit supertypes may live only in the Pilot adapter cache.
            // getOwnedSpecialization alone silently drops those relationships.
            addRelationships(relationships, sourceQualifiedName, "specializes", TypeUtil.getSupertypesOf(type, false), libraryRoot);
            addRelationships(relationships, sourceQualifiedName, "features", type.getOwnedFeature(), libraryRoot);
        }

        if (element instanceof Feature feature) {
            addRelationships(relationships, sourceQualifiedName, "type", FeatureUtil.getAllTypesOf(feature), libraryRoot);
            addRelationships(relationships, sourceQualifiedName, "featuring_type", feature.getFeaturingType(), libraryRoot);
            addRelationships(relationships, sourceQualifiedName, "chaining_feature", feature.getChainingFeature(), libraryRoot);
        }
    }

    private static void addRelationships(
        List<RelationshipKey> relationships,
        String source,
        String relation,
        Collection<? extends Element> targets,
        Path libraryRoot
    ) {
        for (Element target : targets) {
            addRelationship(relationships, source, relation, exportIdentityOf(target, libraryRoot));
        }
    }

    private static void addRelationship(
        List<RelationshipKey> relationships,
        String source,
        String relation,
        String target
    ) {
        if (source == null || target == null || source.equals(target)) {
            return;
        }
        relationships.add(new RelationshipKey(source, relation, target));
    }

    private static String qualifiedNameOf(Element element) {
        return element == null ? null : clean(element.getQualifiedName());
    }

    private static String exportIdentityOf(Element element, Path libraryRoot) {
        if (element == null) { return null; }
        String qualifiedName = qualifiedNameOf(element);
        if (qualifiedName != null) { return qualifiedName; }
        if (element.eResource() == null) { return null; }
        Path resource = resourcePath(element.eResource());
        if (resource == null || !resource.startsWith(libraryRoot)) { return null; }
        String fragment = element.eResource().getURIFragment(element);
        String prefix = element instanceof Membership ? "LibraryMembership::" : "LibraryAnonymous::";
        return prefix + normalizeRelativePath(libraryRoot.relativize(resource))
            + "::" + UUID.nameUUIDFromBytes(fragment.getBytes(StandardCharsets.UTF_8));
    }

    private static Integer startLineOf(Element element) {
        ICompositeNode node = NodeModelUtils.findActualNodeFor(element);
        return node == null ? null : node.getStartLine();
    }

    private static Integer endLineOf(Element element) {
        ICompositeNode node = NodeModelUtils.findActualNodeFor(element);
        return node == null ? null : node.getEndLine();
    }

    private static Path resourcePath(Resource resource) {
        if (resource.getURI() == null || !resource.getURI().isFile()) {
            return null;
        }
        return Paths.get(resource.getURI().toFileString()).toAbsolutePath().normalize();
    }

    private static String normalizeRelativePath(Path path) {
        return path.toString().replace('\\', '/');
    }

    private static String libraryGroup(String relativePath) {
        if (relativePath.startsWith(KERNEL_LIBRARIES.replace('\\', '/'))) {
            return KERNEL_LIBRARIES;
        }
        if (relativePath.startsWith(SYSTEMS_LIBRARY.replace('\\', '/'))) {
            return SYSTEMS_LIBRARY;
        }
        if (relativePath.startsWith(DOMAIN_LIBRARIES.replace('\\', '/'))) {
            return DOMAIN_LIBRARIES;
        }
        return null;
    }

    private static void putIfPresent(Map<String, Object> properties, String key, String value) {
        if (value != null) {
            properties.put(key, value);
        }
    }

    private static String normalizeAttributeName(String name) {
        StringBuilder normalized = new StringBuilder();
        for (int i = 0; i < name.length(); i++) {
            char current = name.charAt(i);
            if (Character.isUpperCase(current)) normalized.append('_');
            normalized.append(Character.toLowerCase(current));
        }
        return normalized.toString();
    }

    private static String clean(String value) {
        if (value == null) {
            return null;
        }
        String normalized = value.replace("\r\n", "\n").trim();
        return normalized.isEmpty() ? null : normalized;
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

    private static final class RelationshipKey {
        private final String source;
        private final String relation;
        private final String target;

        private RelationshipKey(String source, String relation, String target) {
            this.source = source;
            this.relation = relation;
            this.target = target;
        }

        @Override
        public boolean equals(Object other) {
            if (this == other) {
                return true;
            }
            if (!(other instanceof RelationshipKey key)) {
                return false;
            }
            return source.equals(key.source) && relation.equals(key.relation) && target.equals(key.target);
        }

        @Override
        public int hashCode() {
            return Objects.hash(source, relation, target);
        }
    }

    private static final class ExportDocument {
        private ExportMetadata metadata;
        private List<ExportElement> elements;
        private List<ExportRelationship> relationships;
    }

    private static final class ExportMetadata {
        private boolean observed_ecore_defaults_v1;
        private int element_count;
        private int relationship_count;
        private String library_root;
        private List<String> library_files;
        private String exported_at_utc;
        private String pilot_version;
    }

    private static final class ExportElement {
        private String qualified_name;
        private String kind;
        private String library_group;
        private ExportSource source;
        private List<ExportDocumentationBlock> documentation;
        private Map<String, Object> properties;
    }

    private static final class ExportSource {
        private final String file;
        private final Integer start_line;
        private final Integer end_line;

        private ExportSource(String file, Integer startLine, Integer endLine) {
            this.file = file;
            this.start_line = startLine;
            this.end_line = endLine;
        }
    }

    private static final class ExportDocumentationBlock {
        private final String kind;
        private final String text;

        private ExportDocumentationBlock(String kind, String text) {
            this.kind = kind;
            this.text = text;
        }
    }

    private static final class ExportRelationship {
        private final String source;
        private final String relation;
        private final String target;

        private ExportRelationship(String source, String relation, String target) {
            this.source = source;
            this.relation = relation;
            this.target = target;
        }
    }
}
