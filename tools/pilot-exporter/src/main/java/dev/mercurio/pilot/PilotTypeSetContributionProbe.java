package dev.mercurio.pilot;

import com.google.gson.GsonBuilder;
import java.nio.file.Files;
import java.nio.file.Path;
import java.lang.reflect.Method;
import java.util.*;
import org.omg.sysml.lang.sysml.*;
import org.omg.sysml.util.*;

/** Independent bounded getter observations. Supplied completion flags are explicit;
 * this does not qualify cold lifecycle, complete resources or set constraints. */
public final class PilotTypeSetContributionProbe {
    static final List<String> RELATIONS = List.of("Disjoining", "Unioning", "Intersecting", "Differencing");
    static Method selector(Object adapter) throws Exception {
        for (java.lang.Class<?> type = adapter.getClass(); type != null; type = type.getSuperclass()) {
            try { var result = type.getDeclaredMethod("getDefaultSupertype"); result.setAccessible(true); return result; }
            catch (NoSuchMethodException missing) { /* inspect the resolved superclass */ }
        }
        throw new NoSuchMethodException("getDefaultSupertype");
    }
    static Type type(String kind, String name) {
        var factory = SysMLFactory.eINSTANCE;
        Type result = switch (kind) {
            case "Class" -> factory.createClass();
            case "Structure" -> factory.createStructure();
            case "DataType" -> factory.createDataType();
            case "Feature" -> factory.createFeature();
            default -> throw new IllegalArgumentException(kind);
        };
        result.setDeclaredName(name); result.setIsImpliedIncluded(true); return result;
    }
    static Map<String, Object> observe(String receiverKind, int mask, String ownerKind,
            boolean composite, boolean portion, String relationKind, String targetKind, String boundary) throws Exception {
        var factory = SysMLFactory.eINSTANCE;
        Feature feature = switch (receiverKind) {
            case "Feature" -> factory.createFeature();
            case "Connector" -> factory.createConnector();
            case "BindingConnector" -> factory.createBindingConnector();
            default -> throw new IllegalArgumentException(receiverKind);
        };
        feature.setDeclaredName("receiver"); feature.setIsImpliedIncluded(true);
        feature.setIsComposite(composite); feature.setIsPortion(portion);
        Namespace owner = ownerKind.equals("Package") ? factory.createPackage() : type(ownerKind, "owner");
        owner.setDeclaredName("owner");
        var member = ownerKind.equals("Package") ? factory.createOwningMembership() : factory.createFeatureMembership();
        member.getOwnedRelatedElement().add(feature); owner.getOwnedRelationship().add(member);
        String[] kinds = {"Class", "Structure", "DataType"};
        for (int bit = 0; bit < 3; bit++) if ((mask & (1 << bit)) != 0) {
            var typing = factory.createFeatureTyping();
            typing.setType(type(kinds[bit], "typing" + bit)); typing.setTypedFeature(feature);
            feature.getOwnedRelationship().add(typing);
        }
        var row = new TreeMap<String, Object>();
        if (relationKind != null) {
            Relationship relation = switch (relationKind) {
                case "Disjoining" -> factory.createDisjoining();
                case "Unioning" -> factory.createUnioning();
                case "Intersecting" -> factory.createIntersecting();
                case "Differencing" -> factory.createDifferencing();
                default -> throw new IllegalArgumentException(relationKind);
            };
            relation.setDeclaredName("constraint");
            feature.getOwnedRelationship().add(relation);
            String field = relationKind.substring(0, 1).toLowerCase(Locale.ROOT) + relationKind.substring(1) + "Type";
            var endpoint = relation.eClass().getEStructuralFeature(field);
            if (!boundary.equals("missing")) {
                if (boundary.equals("wrong_kind")) {
                    try {
                        relation.eSet(endpoint, factory.createPackage());
                        row.put("endpoint_assignment", "accepted");
                    } catch (RuntimeException rejected) {
                        row.put("endpoint_assignment", rejected.getClass().getName());
                    }
                    return row;
                }
                relation.eSet(endpoint, type(targetKind, "constraintTarget"));
            }
            if (relation instanceof Disjoining disjoining) disjoining.setTypeDisjoined(feature);
            String sourceField = switch (relationKind) {
                case "Disjoining" -> "typeDisjoined";
                case "Unioning" -> "typeUnioned";
                case "Intersecting" -> "typeIntersected";
                case "Differencing" -> "typeDifferenced";
                default -> throw new IllegalArgumentException(relationKind);
            };
            var source = (Type) relation.eGet(relation.eClass().getEStructuralFeature(sourceField), true);
            var target = (Type) relation.eGet(endpoint, true);
            row.put("source_is_receiver", source == feature);
            row.put("endpoint_present", target != null);
            row.put("endpoint_kind", target == null ? "" : target.eClass().getName());
            row.put("owned_subtree_count", relation.getOwnedRelatedElement().size());
        }
        var adapter = ElementUtil.getElementAdapter(feature);
        row.put("default_supertype", selector(adapter).invoke(adapter));
        row.put("feature_property_types", feature.getType().stream().map(Type::getDeclaredName).toList());
        row.put("adapter_all_types", FeatureUtil.getAllTypesOf(feature).stream().map(Type::getDeclaredName).toList());
        row.put("general_types", TypeUtil.getGeneralTypesOf(feature).stream().map(Type::getDeclaredName).toList());
        row.put("receiver_completed", feature.isImpliedIncluded());
        return row;
    }
    public static void main(String[] args) throws Exception {
        org.omg.sysml.logic.SysMLLogicStandaloneSetup.doSetup();
        var rows = new ArrayList<Object>();
        for (String relation : RELATIONS) for (String receiver : List.of("Feature", "Connector", "BindingConnector"))
            for (int mask : List.of(0, 1, 2, 4)) for (String owner : List.of("Package", "Class", "Structure"))
                for (boolean composite : List.of(false, true)) for (boolean portion : List.of(false, true))
                    for (String target : List.of("Class", "Structure", "DataType", "Feature")) {
                    var row = new TreeMap<String, Object>();
                    row.put("relation", relation); row.put("receiver_kind", receiver); row.put("typing_mask", mask);
                    row.put("target_kind", target); row.put("owner_kind", owner); row.put("composite", composite); row.put("portion", portion);
                    row.put("baseline", observe(receiver, mask, owner, composite, portion, null, target, "resolved"));
                    row.put("observation", observe(receiver, mask, owner, composite, portion, relation, target, "resolved"));
                    rows.add(row);
                }
        var negatives = new ArrayList<Object>();
        for (String relation : RELATIONS) for (String boundary : List.of("missing", "wrong_kind")) {
            negatives.add(Map.of("relation", relation, "boundary", boundary,
                "observation", observe("Feature", 4, "Package", false, false, relation, "Class", boundary)));
        }
        Files.writeString(Path.of(args[0]), new GsonBuilder().setPrettyPrinting().create().toJson(
            Map.of("cases", rows, "boundaries", negatives)));
    }
}
