package dev.mercurio.pilot;

import com.google.gson.GsonBuilder;
import java.nio.file.*;
import java.util.*;
import org.eclipse.emf.ecore.*;
import org.eclipse.emf.ecore.util.EObjectValidator;
import org.omg.sysml.lang.sysml.*;

/** Feature-level EMF validation, independent of native required-value code. */
public final class PilotRequiredFeatureProbe extends EObjectValidator {
    boolean check(EObject object, EStructuralFeature feature) {
        return validate_MultiplicityConforms(object, feature, null, new HashMap<>());
    }
    public static void main(String[] args) throws Exception {
        org.omg.sysml.logic.SysMLLogicStandaloneSetup.doSetup();
        var checker = new PilotRequiredFeatureProbe();
        var rows = new ArrayList<Object>();
        for (var classifier : SysMLPackage.eINSTANCE.getEClassifiers()) {
            if (!(classifier instanceof EClass type) || type.isAbstract() || type.isInterface()) continue;
            for (var feature : type.getEAllStructuralFeatures()) {
                if (feature.getLowerBound() <= 0 || feature.isDerived() || feature.isVolatile()) continue;
                var object = SysMLFactory.eINSTANCE.create(type);
                var row = new TreeMap<String,Object>();
                row.put("class", type.getName()); row.put("feature", feature.getName());
                row.put("owner", feature.getEContainingClass().getName());
                try { row.put("valid", checker.check(object, feature)); }
                catch (RuntimeException error) { row.put("error", error.getClass().getSimpleName()); }
                rows.add(row);
            }
        }
        rows.sort(Comparator.comparing(row -> ((Map<?,?>)row).get("class")+"."+((Map<?,?>)row).get("feature")));
        Files.writeString(Path.of(args[0]), new GsonBuilder().setPrettyPrinting().create().toJson(rows));
    }
}
