package dev.mercurio.pilot;
import com.google.gson.GsonBuilder;
import com.sun.source.tree.*;
import com.sun.source.util.*;
import javax.lang.model.element.*;
import javax.tools.*;
import java.nio.file.*;
import java.util.*;

/** Bounded resolved-source translation: super(); field = defaultField = constant. */
public final class PilotConstructorConstantExporter {
    static final String BASE = "org.omg.sysml.lang.sysml.impl.OperatorExpressionImpl";
    static void require(boolean condition, String message) {
        if (!condition) throw new IllegalStateException(message);
    }
    public static void main(String[] args) throws Exception {
        var compiler = ToolProvider.getSystemJavaCompiler();
        var diagnostics = new DiagnosticCollector<JavaFileObject>();
        try (var files = compiler.getStandardFileManager(diagnostics, null, java.nio.charset.StandardCharsets.UTF_8)) {
            var sources = files.getJavaFileObjectsFromStrings(Arrays.asList(args).subList(2,args.length));
            var task = (JavacTask)compiler.getTask(null, files, diagnostics,
                List.of("-proc:none", "-classpath", args[0]), null, sources);
            var units = new ArrayList<CompilationUnitTree>();
            task.parse().forEach(units::add); task.analyze();
            require(diagnostics.getDiagnostics().stream().noneMatch(d -> d.getKind() == Diagnostic.Kind.ERROR), "Unresolved constructor source: "+diagnostics.getDiagnostics());
            var trees = Trees.instance(task);
            var rows = new ArrayList<Object>();
            for (var unit : units) for (var declaration : unit.getTypeDecls()) {
                if (!(declaration instanceof ClassTree type)) continue;
                var typePath = TreePath.getPath(unit,type);
                var symbol = (TypeElement)trees.getElement(typePath);
                require(symbol.getSuperclass().toString().equals(BASE), "Changed superclass");
                var constructors = type.getMembers().stream().filter(m -> m instanceof MethodTree method && method.getReturnType() == null).toList();
                require(constructors.size() == 1, "Changed constructor set");
                var constructor = (MethodTree)constructors.get(0);
                require(constructor.getParameters().isEmpty() && constructor.getBody() != null, "Parameterized constructor");
                var statements = constructor.getBody().getStatements();
                require(statements.size() == 2, "Unassessed constructor statements");
                require(statements.get(0) instanceof ExpressionStatementTree && ((ExpressionStatementTree)statements.get(0)).getExpression() instanceof MethodInvocationTree, "Missing super call");
                var call = (MethodInvocationTree)((ExpressionStatementTree)statements.get(0)).getExpression();
                var called = trees.getElement(TreePath.getPath(unit,call));
                require(called.getKind() == ElementKind.CONSTRUCTOR && called.getEnclosingElement().toString().equals(BASE) && call.getArguments().isEmpty(), "Changed super call");
                require(statements.get(1) instanceof ExpressionStatementTree, "Unassessed initialization");
                ExpressionTree expression = ((ExpressionStatementTree)statements.get(1)).getExpression();
                var writes = new ArrayList<String>();
                while (expression instanceof AssignmentTree assignment) {
                    var field = trees.getElement(TreePath.getPath(unit,assignment.getVariable()));
                    require(field instanceof VariableElement && field.getKind() == ElementKind.FIELD && field.getEnclosingElement().toString().equals(BASE) && field.asType().toString().equals("java.lang.String"), "Unresolved or changed target field");
                    writes.add(field.getSimpleName().toString());
                    expression = assignment.getExpression();
                }
                require(writes.equals(List.of("operator", "OPERATOR_EDEFAULT")), "Changed assignment chain");
                var constant = trees.getElement(TreePath.getPath(unit,expression));
                require(constant instanceof VariableElement && ((VariableElement)constant).getConstantValue() instanceof String, "Nonconstant initializer");
                var value = ((VariableElement)constant).getConstantValue();
                var row = new TreeMap<String,Object>();
                String kind = type.getSimpleName().toString().replaceFirst("Impl$", "");
                row.put("class",kind); row.put("field","operator"); row.put("value",value);
                row.put("writes",writes); row.put("constant",constant.getEnclosingElement()+"."+constant.getSimpleName());
                row.put("constant_type",constant.asType().toString());
                // Independent runtime observation is checked separately from source resolution.
                var classifier = (org.eclipse.emf.ecore.EClass)org.omg.sysml.lang.sysml.SysMLPackage.eINSTANCE.getEClassifier(kind);
                var object = org.omg.sysml.lang.sysml.SysMLFactory.eINSTANCE.create(classifier);
                row.put("observed_value",object.eGet(classifier.getEStructuralFeature("operator")));
                rows.add(row);
            }
            Files.writeString(Path.of(args[1]),new GsonBuilder().setPrettyPrinting().create().toJson(rows));
        }
    }
}
