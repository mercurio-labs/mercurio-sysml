package dev.mercurio.pilot;

import java.nio.file.*;
import java.util.*;
import com.google.gson.*;
import org.eclipse.emf.ecore.*;
import org.omg.sysml.lang.sysml.*;

/** Build-time independent observations of fresh physical EMF reference slots.
 * No linking, derived getters, transformations, prepared flags or publication. */
public final class PilotFreshStructureControls {
    public static void main(String[] args) throws Exception {
        var observations=new JsonArray();
        for(var classifier:SysMLPackage.eINSTANCE.getEClassifiers()) {
            if(!(classifier instanceof EClass kind)||kind.isAbstract()||kind.isInterface())continue;
            var owner=SysMLFactory.eINSTANCE.create(kind);
            var row=new JsonObject();row.addProperty("kind",kind.getName());
            var slots=new JsonObject();
            for(var feature:kind.getEAllStructuralFeatures()) {
                if(!(feature instanceof EReference ref)||ref.isDerived()||ref.isTransient()||ref.isVolatile()
                    ||!ref.isChangeable()||ref.isUnsettable()||ref.getEOpposite()==null
                    ||ref.getLowerBound()!=0||!(ref.isContainment()&&ref.getUpperBound()==-1
                        ||ref.isContainer()&&ref.getUpperBound()==1))continue;
                Object value=owner.eGet(ref,false);
                if(ref.isMany()) {
                    var values=new JsonArray();
                    for(var endpoint:(Iterable<?>)value)values.add(((EObject)endpoint).eClass().getName());
                    slots.add(ref.getName(),values);
                } else slots.add(ref.getName(),value==null?JsonNull.INSTANCE:new JsonPrimitive(((EObject)value).eClass().getName()));
            }
            row.add("slots",slots);
            row.addProperty("completion_flag",((Element)owner).isImpliedIncluded());
            observations.add(row);
        }
        var out=new JsonObject();out.addProperty("schema","dev.mercurio.fresh-structure-controls.v1");
        out.addProperty("qualification_certificate",false);out.add("observations",observations);
        Files.writeString(Path.of(args[0]),new GsonBuilder().serializeNulls().setPrettyPrinting().create().toJson(out));
    }
}
