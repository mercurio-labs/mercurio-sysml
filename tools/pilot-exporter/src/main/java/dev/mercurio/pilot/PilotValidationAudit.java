package dev.mercurio.pilot;

import java.nio.file.*;
import java.nio.charset.StandardCharsets;
import java.util.*;
import com.google.gson.*;
import org.eclipse.emf.ecore.resource.Resource;
import org.eclipse.xtext.resource.XtextResource;
import org.eclipse.xtext.validation.CheckMode;
import org.eclipse.xtext.validation.Issue;
import org.eclipse.xtext.diagnostics.Severity;
import org.eclipse.xtext.util.CancelIndicator;
import org.omg.sysml.interactive.SysMLInteractive;

/** Full validation; each invocation is restricted to an identical input set. */
public final class PilotValidationAudit {
    private static final Gson JSON = new GsonBuilder().disableHtmlEscaping().create();
    private static final class Spec { List<Case> cases; }
    private static final class Case { String relative_path; List<String> input_files; }

    public static void main(String[] args) throws Exception {
        if (args.length != 3) throw new IllegalArgumentException("usage: PilotValidationAudit <library> <spec> <jsonl>");
        Spec spec = JSON.fromJson(Files.readString(Path.of(args[1])), Spec.class);
        if (spec.cases == null || spec.cases.isEmpty()) throw new IllegalArgumentException("empty corpus");
        Set<String> expected = new TreeSet<>(spec.cases.get(0).input_files);
        for (Case c : spec.cases) {
            if (!expected.equals(new TreeSet<>(c.input_files))) throw new IllegalArgumentException("batch input sets differ");
        }
        System.setProperty("org.eclipse.emf.common.util.ReferenceClearingQueue", "false");
        SysMLInteractive interactive = SysMLInteractive.getInstance();
        interactive.getLibraryIndexCache().setIndexDisabled(true);
        interactive.setVerbose(false);
        interactive.loadLibrary(args[0]);
        Map<String, Resource> resources = new LinkedHashMap<>();
        for (String path : expected) {
            Resource resource = interactive.readResource(path);
            interactive.addInputResource(resource);
            resources.put(path, resource);
        }
        // The same CheckMode.ALL validation used by Pilot's interactive process.
        // Resource.getErrors() alone omits semantic validation.
        try (var out = Files.newBufferedWriter(Path.of(args[2]), StandardCharsets.UTF_8)) {
            for (Case c : spec.cases) {
                long start = System.nanoTime();
                Map<String, Object> result = new LinkedHashMap<>();
                result.put("relative_path", c.relative_path);
                List<Map<String, Object>> diagnostics = new ArrayList<>();
                try {
                    XtextResource resource = (XtextResource) resources.get(c.input_files.get(c.input_files.size() - 1));
                    result.put("parse_ok", resource.getParseResult() != null && !resource.getParseResult().hasSyntaxErrors());
                    var issues = resource.getResourceServiceProvider().getResourceValidator()
                        .validate(resource, CheckMode.ALL, CancelIndicator.NullImpl);
                    for (Issue issue : issues) {
                        Map<String, Object> diagnostic = new LinkedHashMap<>();
                        diagnostic.put("severity", issue.getSeverity().toString());
                        diagnostic.put("syntax", issue.isSyntaxError());
                        diagnostic.put("code", issue.getCode());
                        diagnostic.put("message", issue.getMessage());
                        diagnostic.put("line", issue.getLineNumber());
                        diagnostic.put("column", issue.getColumn());
                        diagnostics.add(diagnostic);
                    }
                    result.put("status", issues.stream().anyMatch(i -> i.getSeverity() == Severity.ERROR) ? "error" : "ok");
                } catch (Throwable error) {
                    if (error instanceof VirtualMachineError && !(error instanceof StackOverflowError)) throw error;
                    result.put("status", "infrastructure_error");
                    result.put("error", error.toString());
                }
                result.put("diagnostics", diagnostics);
                result.put("elapsed_ms", (System.nanoTime() - start) / 1_000_000);
                out.write(JSON.toJson(result));
                out.newLine();
                out.flush();
            }
        }
    }
}
