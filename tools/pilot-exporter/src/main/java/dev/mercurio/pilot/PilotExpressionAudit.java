package dev.mercurio.pilot;

import java.nio.file.*;
import java.nio.charset.StandardCharsets;
import java.util.*;
import com.google.gson.*;
import org.omg.sysml.interactive.SysMLInteractive;
import org.omg.sysml.execution.expressions.ExpressionEvaluator;
import org.omg.sysml.lang.sysml.*;
import org.omg.sysml.util.TypeUtil;

/** Executable expression oracle. Unsupported results remain visible as errors. */
public final class PilotExpressionAudit {
    private static final Gson JSON = new GsonBuilder().disableHtmlEscaping().serializeNulls().create();
    public static void main(String[] args) throws Exception {
        if (args.length != 3) throw new IllegalArgumentException("usage: PilotExpressionAudit <library> <cases.json> <results.json>");
        System.setProperty("org.eclipse.emf.common.util.ReferenceClearingQueue", "false");
        SysMLInteractive interactive = SysMLInteractive.getInstance();
        interactive.getLibraryIndexCache().setIndexDisabled(true);
        interactive.setVerbose(false);
        interactive.loadLibrary(Path.of(args[0]).toAbsolutePath().normalize().toString().replace('\\', '/'));
        JsonArray cases = JsonParser.parseString(Files.readString(Path.of(args[1]))).getAsJsonArray();
        List<Map<String,Object>> results = new ArrayList<>();
        for (JsonElement item : cases) {
            JsonObject c = item.getAsJsonObject();
            Map<String,Object> result = new LinkedHashMap<>();
            result.put("id", c.get("id").getAsString());
            result.put("expression", c.get("expression").getAsString());
            try {
                var parsed = interactive.process("calc { " + c.get("expression").getAsString() + " }", false);
                result.put("diagnostics", parsed.toString());
                if (parsed.hasErrors()) throw new IllegalArgumentException("Pilot rejected expression");
                Type calc = (Type)((Namespace)parsed.getRootElement()).getOwnedMember().get(0);
                Expression expression = (Expression)TypeUtil.getFeatureByMembershipIn(calc, ResultExpressionMembership.class);
                var values = ExpressionEvaluator.INSTANCE.evaluate(expression, null);
                if (values == null) throw new IllegalArgumentException("Pilot returned unresolved evaluation");
                List<Object> resultValues = new ArrayList<>();
                for (Element value : values) {
                    if (value instanceof LiteralBoolean) resultValues.add(((LiteralBoolean)value).isValue());
                    else if (value instanceof LiteralInteger) resultValues.add(((LiteralInteger)value).getValue());
                    else if (value instanceof LiteralRational) resultValues.add(((LiteralRational)value).getValue());
                    else if (value instanceof LiteralString) resultValues.add(((LiteralString)value).getValue());
                    else throw new IllegalArgumentException("Unevaluated result: " + value.eClass().getName());
                }
                result.put("values", resultValues);
                result.put("status", "ok");
            } catch (Exception error) {
                result.put("status", "error");
                result.put("error", error.toString());
            } finally {
                interactive.removeResource();
            }
            results.add(result);
        }
        Files.writeString(Path.of(args[2]), JSON.toJson(results) + "\n", StandardCharsets.UTF_8);
    }
}
