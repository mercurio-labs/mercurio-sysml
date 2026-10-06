package dev.mercurio.pilot;

import com.google.gson.GsonBuilder;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.*;
import org.eclipse.emf.ecore.EClass;
import org.eclipse.emf.ecore.util.InternalEList;
import org.omg.sysml.lang.sysml.*;

/** Construction-only oracle: no library loading, specialization or argument derivation. */
public final class PilotOperandProbe {
    static Map<String,Object> node(Element element) {
        Map<String,Object> row = new TreeMap<>();
        row.put("kind", element.eClass().getName());
        if (element instanceof LiteralInteger literal) row.put("value", literal.getValue());
        if (element instanceof Membership member) row.put("visibility", member.getVisibility().getLiteral());
        if (element instanceof Feature feature && feature.getDirection() != null) row.put("direction", feature.getDirection().getLiteral());
        row.put("relationships", element.getOwnedRelationship().stream().map(PilotOperandProbe::node).toList());
        if (element instanceof Relationship relationship) {
            row.put("elements", relationship.getOwnedRelatedElement().stream().map(PilotOperandProbe::node).toList());
            for (Element child : relationship.getOwnedRelatedElement())
                if (child.getOwningRelationship() != relationship) throw new IllegalStateException("Missing element inverse");
        }
        for (Relationship relationship : element.getOwnedRelationship())
            if (relationship.getOwningRelatedElement() != element) throw new IllegalStateException("Missing relationship inverse");
        return row;
    }
    public static void main(String[] args) throws Exception {
        org.omg.sysml.logic.SysMLLogicStandaloneSetup.doSetup();
        List<Map<String,Object>> cases = new ArrayList<>();
        for (var classifier : SysMLPackage.eINSTANCE.getEClassifiers()) {
            if (!(classifier instanceof EClass type) || type.isAbstract() || type.isInterface() ||
                !SysMLPackage.eINSTANCE.getInvocationExpression().isSuperTypeOf(type)) continue;
            InvocationExpression owner = (InvocationExpression)SysMLFactory.eINSTANCE.create(type);
            for (int value : new int[]{2, 2}) {
                LiteralInteger literal = SysMLFactory.eINSTANCE.createLiteralInteger();
                literal.setValue(value);
                ((InternalEList<Expression>)owner.getOperand()).addUnique(literal);
            }
            Map<String,Object> row = new TreeMap<>();
            row.put("class", type.getName());
            row.put("tree", node(owner));
            row.put("operand_read_size", owner.getOperand().size());
            cases.add(row);
        }
        Files.writeString(Path.of(args[0]), new GsonBuilder().setPrettyPrinting().create().toJson(cases) + "\n");
    }
}
