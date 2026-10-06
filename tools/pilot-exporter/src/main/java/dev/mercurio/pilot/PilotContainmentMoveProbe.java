package dev.mercurio.pilot;
import com.google.gson.GsonBuilder;
import java.nio.file.*;
import java.util.*;
import org.eclipse.emf.ecore.*;
import org.eclipse.emf.common.util.EList;
import org.omg.sysml.lang.sysml.*;

/** Observe actual EMF inverse maintenance; no native mutation code or tables. */
public final class PilotContainmentMoveProbe {
    static Map<String,EObject> objects = new LinkedHashMap<>();
    static Map<EObject,String> ids = new IdentityHashMap<>();
    static void create(String id, String kind) {
        var object=SysMLFactory.eINSTANCE.create((EClass)SysMLPackage.eINSTANCE.getEClassifier(kind));
        objects.put(id,object); ids.put(object,id);
    }
    @SuppressWarnings("unchecked")
    static void move(String child, String owner, String field, boolean inverse) {
        var parent=objects.get(owner); var target=objects.get(child);
        var feature=(EReference)parent.eClass().getEStructuralFeature(field);
        if(inverse) target.eSet(feature.getEOpposite(),parent);
        else ((EList<EObject>)parent.eGet(feature)).add(target);
    }
    static List<Object> snapshot() {
        var rows=new ArrayList<Object>();
        for(var entry:objects.entrySet()) {
            var object=entry.getValue();var fields=new TreeMap<String,Object>();
            for(var feature:object.eClass().getEAllReferences()) {
                if(feature.isDerived() || feature.isVolatile() || !(feature.isContainment() || feature.isContainer())) continue;
                var value=object.eGet(feature);
                if(feature.isMany()) fields.put(feature.getName(),((List<?>)value).stream().map(ids::get).toList());
                else fields.put(feature.getName(),value==null ? null : ids.get(value));
            }
            rows.add(Map.of("id",entry.getKey(),"class",object.eClass().getName(),"fields",fields));
        }
        return rows;
    }
    public static void main(String[] args) throws Exception {
        org.omg.sysml.logic.SysMLLogicStandaloneSetup.doSetup();
        create("p","Package");create("q","Package");
        create("a","OwningMembership");create("b","OwningMembership");create("c","OwningMembership");create("x","Class");
        move("a","p","ownedRelationship",false);move("b","p","ownedRelationship",false);
        move("c","q","ownedRelationship",false);move("x","a","ownedRelatedElement",false);
        var initial=snapshot();var controls=new ArrayList<Object>();
        for(var operation:List.of(
            List.of("a","q","ownedRelationship","forward"),
            List.of("a","q","ownedRelationship","forward"),
            List.of("x","c","ownedRelatedElement","forward"),
            List.of("a","c","ownedRelatedElement","forward"),
            List.of("a","p","ownedRelationship","inverse"),
            List.of("x","a","ownedRelatedElement","inverse"),
            List.of("a","x","ownedRelationship","inverse"),
            List.of("a","x","ownedRelationship","forward"))) {
            var row=new TreeMap<String,Object>();row.put("child",operation.get(0));row.put("owner",operation.get(1));row.put("field",operation.get(2));row.put("write_side",operation.get(3));
            // This is the exact upstream guard used by generated owner setters.
            row.put("recursive_destination",org.eclipse.emf.ecore.util.EcoreUtil.isAncestor(objects.get(operation.get(0)),objects.get(operation.get(1))));
            try {move(operation.get(0),operation.get(1),operation.get(2),operation.get(3).equals("inverse"));row.put("accepted",true);}
            catch(RuntimeException error) {row.put("accepted",false);row.put("error",error.getClass().getSimpleName());}
            row.put("after",snapshot());controls.add(row);
        }
        Files.writeString(Path.of(args[0]),new GsonBuilder().serializeNulls().setPrettyPrinting().create().toJson(Map.of("initial",initial,"steps",controls)));
    }
}
