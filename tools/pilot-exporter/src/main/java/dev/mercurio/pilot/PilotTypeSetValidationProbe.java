package dev.mercurio.pilot;
import com.google.gson.GsonBuilder;
import java.nio.file.*;
import java.nio.charset.StandardCharsets;
import java.util.*;
import org.eclipse.emf.ecore.*;
import org.eclipse.xtext.validation.ValidationMessageAcceptor;
import org.omg.sysml.lang.sysml.*;
import org.omg.kerml.xtext.validation.KerMLValidator;

/** Independent predicate and projection observations; not full model validity. */
public final class PilotTypeSetValidationProbe {
    static Map<String,Object> object(Object... values) {
        var result=new TreeMap<String,Object>();
        for(int i=0;i<values.length;i+=2)result.put((String)values[i],values[i+1]);
        return result;
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
                if (!(subject instanceof Relationship)) throw new AssertionError("Unexpected diagnostic subject");
                subjects.put(subject, ((Element)subject).getDeclaredName());
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
    static Relationship relation(String kind) {
        return switch(kind) {
            case "Unioning" -> SysMLFactory.eINSTANCE.createUnioning();
            case "Intersecting" -> SysMLFactory.eINSTANCE.createIntersecting();
            case "Differencing" -> SysMLFactory.eINSTANCE.createDifferencing();
            default -> throw new IllegalArgumentException(kind);
        };
    }
    static Feature receiver(String kind) {
        var f=SysMLFactory.eINSTANCE;
        Feature result=switch(kind) {
            case "Feature" -> f.createFeature();
            case "Connector" -> f.createConnector();
            case "BindingConnector" -> f.createBindingConnector();
            default -> throw new IllegalArgumentException(kind);
        };
        result.setDeclaredName("receiver");result.setIsImpliedIncluded(true);return result;
    }
    public static void main(String[] args) throws Exception {
        org.omg.sysml.logic.SysMLLogicStandaloneSetup.doSetup();
        var rows=new ArrayList<Object>();
        for(String kind:List.of("Unioning","Intersecting","Differencing"))
            for(String receiverKind:List.of("Feature","Connector","BindingConnector"))
                for(int count=0;count<4;count++)for(String pattern:List.of("distinct","duplicate","self")) {
                    var subject=receiver(receiverKind);
                    var shared=SysMLFactory.eINSTANCE.createClass();shared.setDeclaredName("target0");shared.setIsImpliedIncluded(true);
                    for(int ordinal=0;ordinal<count;ordinal++) {
                        var r=relation(kind);r.setDeclaredName("relation"+ordinal);
                        Type target=shared;
                        if(pattern.equals("self") && ordinal+1==count)target=subject;
                        else if(!pattern.equals("duplicate")) {
                            var other=SysMLFactory.eINSTANCE.createClass();other.setDeclaredName("target"+ordinal);other.setIsImpliedIncluded(true);target=other;
                        }
                        String field=kind.substring(0,1).toLowerCase(Locale.ROOT)+kind.substring(1)+"Type";
                        r.eSet(r.eClass().getEStructuralFeature(field),target);subject.getOwnedRelationship().add(r);
                    }
                    String owned="owned"+kind;
                    String field=kind.substring(0,1).toLowerCase(Locale.ROOT)+kind.substring(1)+"Type";
                    @SuppressWarnings("unchecked") var ownedValues=(List<Relationship>)subject.eGet(subject.eClass().getEStructuralFeature(owned));
                    @SuppressWarnings("unchecked") var types=(List<Type>)subject.eGet(subject.eClass().getEStructuralFeature(field));
                    Capture capture=new Capture(subject,"receiver");
                    KerMLValidator validator=new KerMLValidator();
                    validator.setMessageAcceptor(capture).getState().currentObject=subject;
                    validator.checkType(subject);
                    rows.add(object("relation",kind,"receiver_kind",receiverKind,"count",count,"pattern",pattern,
                        "owned",ownedValues.stream().map(Element::getDeclaredName).toList(),
                        "endpoints",types.stream().map(Element::getDeclaredName).toList(),"diagnostics",capture.diagnostics));
                }
        var boundaries=new ArrayList<Object>();
        for(String mode:List.of("canonical","foreign_source","package_owner")) {
            var subject=receiver("Feature");
            var foreign=SysMLFactory.eINSTANCE.createClass();foreign.setDeclaredName("foreign");foreign.setIsImpliedIncluded(true);
            var r=SysMLFactory.eINSTANCE.createDisjoining();r.setDeclaredName("relation");
            r.setTypeDisjoined(mode.equals("foreign_source")?foreign:subject);r.setDisjoiningType(foreign);
            if(mode.equals("package_owner"))SysMLFactory.eINSTANCE.createPackage().getOwnedRelationship().add(r);
            else subject.getOwnedRelationship().add(r);
            boundaries.add(object("mode",mode,"owned_count",subject.getOwnedDisjoining().size(),
                "owning_is_receiver",r.getOwningType()==subject,"owning_present",r.getOwningType()!=null,
                "stored_source_is_receiver",r.getTypeDisjoined()==subject));
        }
        Files.writeString(Path.of(args[0]),new GsonBuilder().serializeNulls().setPrettyPrinting().create().toJson(
            object("cases",rows,"boundaries",boundaries)),StandardCharsets.UTF_8);
    }
}
