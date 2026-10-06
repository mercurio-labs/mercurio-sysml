package dev.mercurio.pilot;

import com.google.gson.GsonBuilder;
import java.nio.file.*;
import java.nio.charset.StandardCharsets;
import java.util.*;
import org.eclipse.emf.common.util.URI;
import org.eclipse.emf.ecore.*;
import org.eclipse.emf.ecore.resource.impl.ResourceSetImpl;
import org.eclipse.emf.ecore.util.EcoreUtil;
import org.eclipse.emf.ecore.xmi.impl.EcoreResourceFactoryImpl;

/** Build-time effective Ecore metadata. Does not execute SysML delegates. */
public final class PilotEcoreExporter {
    private static String id(EObject value) {
        if (value == null) return null;
        if (value.eIsProxy()) throw new IllegalStateException("Unresolved Ecore reference");
        if (value instanceof EPackage p) return p.getNsURI();
        if (value instanceof EClassifier c) return id(c.getEPackage()) + "#//" + c.getName();
        if (value instanceof EStructuralFeature f) return id(f.getEContainingClass()) + "/" + f.getName();
        throw new IllegalStateException("Unsupported identity " + value.eClass().getName());
    }
    private static Map<String,Object> typed(ETypedElement e) {
        Map<String,Object> r = new TreeMap<>();
        r.put("name", e.getName()); r.put("type", id(e.getEType()));
        r.put("lower_bound", e.getLowerBound()); r.put("upper_bound", e.getUpperBound());
        r.put("ordered", e.isOrdered()); r.put("unique", e.isUnique());
        return r;
    }
    public static void main(String[] args) throws Exception {
        if (args.length != 2) throw new IllegalArgumentException("MODEL OUTPUT");
        var set = new ResourceSetImpl();
        set.getResourceFactoryRegistry().getExtensionToFactoryMap().put("ecore", new EcoreResourceFactoryImpl());
        set.getPackageRegistry().put(EcorePackage.eNS_URI, EcorePackage.eINSTANCE);
        var resource = set.getResource(URI.createFileURI(Path.of(args[0]).toAbsolutePath().toString()), true);
        EcoreUtil.resolveAll(set);
        for (var r : set.getResources()) if (!r.getErrors().isEmpty()) throw new IllegalStateException(r.getErrors().toString());
        if (!EcoreUtil.UnresolvedProxyCrossReferencer.find(set).isEmpty()) throw new IllegalStateException("Unresolved proxies");
        var classes = new ArrayList<Object>(); var features = new ArrayList<Object>(); var operations = new ArrayList<Object>();
        var objects = resource.getAllContents();
        while (objects.hasNext()) {
            EObject obj = objects.next();
            if (obj instanceof EClass c) {
                var row = new TreeMap<String,Object>(); row.put("id", id(c)); row.put("name", c.getName());
                row.put("abstract", c.isAbstract()); row.put("interface", c.isInterface());
                row.put("super_types", c.getESuperTypes().stream().map(PilotEcoreExporter::id).toList());
                classes.add(row);
            }
            if (obj instanceof EStructuralFeature f) {
                var row = typed(f); row.put("id", id(f)); row.put("owner", id(f.getEContainingClass()));
                row.put("kind", f instanceof EReference ? "reference" : "attribute");
                row.put("changeable", f.isChangeable()); row.put("unsettable", f.isUnsettable());
                row.put("derived", f.isDerived()); row.put("transient", f.isTransient()); row.put("volatile", f.isVolatile());
                row.put("default_literal", f.getDefaultValueLiteral());
                Object value = f.getDefaultValue();
                if (value == null || value instanceof String || value instanceof Number || value instanceof Boolean) row.put("default_value", value);
                else if (value instanceof org.eclipse.emf.common.util.Enumerator e) row.put("default_value", e.getLiteral());
                else throw new IllegalStateException("Unsupported default value " + id(f) + ": " + value.getClass());
                row.put("containment", f instanceof EReference r && r.isContainment());
                row.put("container", f instanceof EReference r && r.isContainer());
                row.put("opposite", f instanceof EReference r ? id(r.getEOpposite()) : null);
                row.put("resolve_proxies", f instanceof EReference r ? r.isResolveProxies() : null);
                row.put("is_id", f instanceof EAttribute a && a.isID());
                features.add(row);
            }
            if (obj instanceof EOperation op) {
                var row = typed(op); row.put("owner", id(op.getEContainingClass()));
                row.put("source_fragment", resource.getURIFragment(op));
                row.put("parameters", op.getEParameters().stream().map(PilotEcoreExporter::typed).toList());
                row.put("exceptions", op.getEExceptions().stream().map(PilotEcoreExporter::id).toList());
                operations.add(row);
            }
        }
        var result = new TreeMap<String,Object>(); result.put("classes", classes); result.put("features", features);
        result.put("operations", operations); result.put("schema", "dev.mercurio.emf-effective.v1");
        result.put("behavior", "metadata only; delegates and model algorithms not executed");
        Files.writeString(Path.of(args[1]), new GsonBuilder().serializeNulls().disableHtmlEscaping().setPrettyPrinting().create().toJson(result) + "\n", StandardCharsets.UTF_8);
    }
}
