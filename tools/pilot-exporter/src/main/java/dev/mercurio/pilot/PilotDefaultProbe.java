package dev.mercurio.pilot;

import com.google.gson.GsonBuilder;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.Comparator;
import java.util.List;
import java.util.Map;
import java.util.TreeMap;
import org.eclipse.emf.common.util.Enumerator;
import org.eclipse.emf.ecore.EAttribute;
import org.eclipse.emf.ecore.EClass;
import org.eclipse.emf.ecore.EClassifier;
import org.eclipse.emf.ecore.EObject;
import org.omg.sysml.lang.sysml.SysMLFactory;
import org.omg.sysml.lang.sysml.SysMLPackage;

/** Observe constructor values for every pinned explicit Ecore default and concrete descendant. */
public final class PilotDefaultProbe {
    private PilotDefaultProbe() {}

    private static Object jsonValue(Object value) {
        if (value == null || value instanceof String || value instanceof Number || value instanceof Boolean) return value;
        if (value instanceof Enumerator enumerator) return enumerator.getLiteral();
        throw new IllegalStateException("Unsupported default type: " + value.getClass());
    }

    public static void main(String[] args) throws Exception {
        if (args.length != 1) throw new IllegalArgumentException("OUTPUT");
        List<Map<String, Object>> rows = new ArrayList<>();
        for (EClassifier classifier : SysMLPackage.eINSTANCE.getEClassifiers()) {
            if (!(classifier instanceof EClass type) || type.isAbstract() || type.isInterface()) continue;
            EObject instance = SysMLFactory.eINSTANCE.create(type);
            for (EAttribute attribute : type.getEAllAttributes()) {
                if (attribute.getDefaultValueLiteral() == null) continue;
                Map<String, Object> row = new TreeMap<>();
                row.put("class", type.getName());
                row.put("owner", attribute.getEContainingClass().getName());
                row.put("feature", attribute.getName());
                row.put("ecore_default", jsonValue(attribute.getDefaultValue()));
                try {
                    row.put("initial_value", jsonValue(instance.eGet(attribute)));
                } catch (RuntimeException error) {
                    row.put("initial_error", error.getClass().getSimpleName());
                }
                try {
                    row.put("is_set", instance.eIsSet(attribute));
                } catch (RuntimeException error) {
                    row.put("is_set_error", error.getClass().getSimpleName());
                }
                rows.add(row);
            }
        }
        rows.sort(Comparator.comparing((Map<String, Object> row) -> (String) row.get("class"))
            .thenComparing(row -> (String) row.get("feature")));
        Files.writeString(Path.of(args[0]), new GsonBuilder().serializeNulls().setPrettyPrinting().create()
            .toJson(Map.of("schema", "dev.mercurio.pilot-constructor-defaults.v1", "rows", rows)) + "\n",
            StandardCharsets.UTF_8);
    }
}
