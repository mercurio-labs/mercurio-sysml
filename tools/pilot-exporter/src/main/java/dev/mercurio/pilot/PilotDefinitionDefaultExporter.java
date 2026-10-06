package dev.mercurio.pilot;

import com.google.gson.GsonBuilder;
import com.sun.source.tree.*;
import com.sun.source.util.*;
import javax.lang.model.element.*;
import javax.tools.*;
import java.nio.file.*;
import java.lang.reflect.Method;
import java.util.*;
import org.eclipse.emf.ecore.*;
import org.omg.sysml.lang.sysml.*;
import org.omg.sysml.util.*;

/** Bounded resolved Java policies plus independent runtime observations. */
public final class PilotDefinitionDefaultExporter {
    static void require(boolean condition, String message) {
        if (!condition) throw new IllegalStateException(message);
    }
    static Method method(java.lang.Class<?> type, String name) throws Exception {
        while (type != null) {
            try { return type.getDeclaredMethod(name); }
            catch (NoSuchMethodException ignored) { type = type.getSuperclass(); }
        }
        throw new IllegalStateException("Missing adapter method " + name);
    }
    static String key(Method method) { return method.getDeclaringClass().getName() + "#" + method.getName(); }
    static Map<String,Object> node(CompilationUnitTree unit, Tree tree, Trees trees) {
        var row = new TreeMap<String,Object>();
        row.put("kind", tree.getKind().name());
        var path = TreePath.getPath(unit, tree);
        var type = trees.getTypeMirror(path);
        if (type != null) row.put("type", type.toString());
        switch (tree) {
            case BlockTree block -> row.put("statements", block.getStatements().stream().map(t -> node(unit,t,trees)).toList());
            case ExpressionStatementTree expression -> row.put("expression", node(unit,expression.getExpression(),trees));
            case ReturnTree ret -> row.put("expression", node(unit,ret.getExpression(),trees));
            case IfTree conditional -> {
                row.put("condition",node(unit,conditional.getCondition(),trees));
                row.put("then",node(unit,conditional.getThenStatement(),trees));
                require(conditional.getElseStatement()==null,"Unexpected else statement");
            }
            case ParenthesizedTree parenthesized -> row.put("expression",node(unit,parenthesized.getExpression(),trees));
            case ConditionalExpressionTree conditional -> {
                row.put("condition",node(unit,conditional.getCondition(),trees));
                row.put("true",node(unit,conditional.getTrueExpression(),trees));
                row.put("false",node(unit,conditional.getFalseExpression(),trees));
            }
            case BinaryTree binary -> {
                require(tree.getKind()==Tree.Kind.NOT_EQUAL_TO,"Unsupported operator");
                row.put("left",node(unit,binary.getLeftOperand(),trees)); row.put("right",node(unit,binary.getRightOperand(),trees));
            }
            case LiteralTree literal -> row.put("value",literal.getValue());
            case IdentifierTree identifier -> {
                require(identifier.getName().contentEquals("super"),"Unbound identifier"); row.put("name","super");
            }
            case MethodInvocationTree call -> {
                var symbol=trees.getElement(path);
                require(symbol instanceof ExecutableElement,"Unresolved invocation");
                var executable=(ExecutableElement)symbol;
                row.put("symbol",executable.getEnclosingElement()+"#"+executable.getSimpleName());
                row.put("parameters",executable.getParameters().stream().map(p->p.asType().toString()).toList());
                row.put("arguments",call.getArguments().stream().map(t->node(unit,t,trees)).toList());
                if(call.getMethodSelect() instanceof MemberSelectTree select) row.put("receiver",node(unit,select.getExpression(),trees));
            }
            default -> throw new IllegalStateException("Unsupported syntax " + tree.getKind());
        }
        return row;
    }
    public static void main(String[] args) throws Exception {
        org.omg.sysml.logic.SysMLLogicStandaloneSetup.doSetup();
        var methods=new TreeMap<String,Object>();
        var requested=Set.of("org.omg.sysml.adapter.TypeAdapter#getDefaultSupertype",
            "org.omg.sysml.adapter.OccurrenceDefinitionAdapter#addDefaultGeneralType",
            "org.omg.sysml.adapter.ConnectionDefinitionAdapter#getDefaultSupertype",
            "org.omg.sysml.adapter.FlowDefinitionAdapter#getDefaultSupertype",
            "org.omg.sysml.adapter.NamespaceAdapter#addAdditionalMembers");
        var compiler=ToolProvider.getSystemJavaCompiler(); var diagnostics=new DiagnosticCollector<JavaFileObject>();
        try(var files=compiler.getStandardFileManager(diagnostics,null,java.nio.charset.StandardCharsets.UTF_8)) {
            var task=(JavacTask)compiler.getTask(null,files,diagnostics,List.of("-proc:none","-classpath",args[0]),null,
                files.getJavaFileObjectsFromStrings(Arrays.asList(args).subList(2,args.length)));
            var units=new ArrayList<CompilationUnitTree>(); task.parse().forEach(units::add); task.analyze();
            require(diagnostics.getDiagnostics().stream().noneMatch(d->d.getKind()==Diagnostic.Kind.ERROR),"Unresolved source: "+diagnostics.getDiagnostics());
            var trees=Trees.instance(task);
            for(var unit:units) for(var declaration:unit.getTypeDecls()) if(declaration instanceof ClassTree type) {
                var symbol=trees.getElement(TreePath.getPath(unit,type));
                for(var member:type.getMembers()) if(member instanceof MethodTree method && method.getParameters().isEmpty()) {
                    String key=symbol+"#"+method.getName();
                    if(requested.contains(key)) methods.put(key,node(unit,method.getBody(),trees));
                }
            }
        }
        require(methods.keySet().equals(requested),"Missing resolved policy bodies");
        var bindings=new ArrayList<Object>(); var controls=new ArrayList<Object>();
        var factory=SysMLFactory.eINSTANCE;
        for(var classifier:SysMLPackage.eINSTANCE.getEClassifiers()) {
            if(!(classifier instanceof EClass cls) || cls.isAbstract() || !SysMLPackage.Literals.DEFINITION.isSuperTypeOf(cls)) continue;
            var sample=(Definition)factory.create(cls); var adapter=ElementUtil.getElementAdapter(sample);
            String selection=key(method(adapter.getClass(),"getDefaultSupertype"));
            String additions=key(method(adapter.getClass(),"addDefaultGeneralType"));
            String members=key(method(adapter.getClass(),"addAdditionalMembers"));
            require(key(method(adapter.getClass(),"computeImplicitGeneralTypes")).equals("org.omg.sysml.adapter.TypeAdapter#computeImplicitGeneralTypes"),"Unassessed implicit computation override");
            require(key(method(adapter.getClass(),"getBaseTypes")).equals("org.omg.sysml.adapter.ClassifierAdapter#getBaseTypes"),"Unassessed metadata base override");
            require(methods.containsKey(selection),"Unsupported default dispatch "+selection);
            require(methods.containsKey(additions)||additions.equals("org.omg.sysml.adapter.TypeAdapter#addDefaultGeneralType"),"Unsupported additional defaults "+additions);
            require(members.equals("org.omg.sysml.adapter.NamespaceAdapter#addAdditionalMembers"),"Unassessed added members "+members);
            var labels=new TreeMap<String,String>();
            for(String label:List.of("base","binary","life")) {
                var name=ImplicitGeneralizationMap.getDefaultSupertypeFor(sample.getClass(),label);
                if(name!=null) labels.put(label,name);
            }
            bindings.add(Map.of("kind",cls.getName(),"adapter",adapter.getClass().getName(),"selection",selection,"additions",additions,"additional_members",members,"labels",labels));
            // Every supported input branch is evaluated by the actual adapter.
            for(int ends=0;ends<4;ends++) for(boolean individual:List.of(false,true)) {
                if(individual && !(sample instanceof OccurrenceDefinition)) continue;
                var owner=(Definition)factory.create(cls);
                if(owner instanceof OccurrenceDefinition occurrence) occurrence.setIsIndividual(individual);
                for(int i=0;i<ends;i++) {
                    var membership=factory.createFeatureMembership(); var feature=factory.createFeature(); feature.setIsEnd(true);
                    membership.setOwnedMemberFeature(feature); owner.getOwnedRelationship().add(membership);
                }
                // Unrelated non-end Features must not affect the selector.
                var ordinary=factory.createFeatureMembership(); ordinary.setOwnedMemberFeature(factory.createFeature()); owner.getOwnedRelationship().add(ordinary);
                var targets=new TreeMap<String,Type>(); var ids=new IdentityHashMap<Type,String>();
                for(String name:labels.values()) { var target=factory.createClassifier(); targets.put(name,target); ids.put(target,name); }
                SysMLLibraryUtil.setProviderLookup(resource->(context,name)->targets.get(name));
                var computed=TypeUtil.getGeneralTypesOf(owner).stream().map(t->Objects.requireNonNull(ids.get(t))).toList();
                controls.add(Map.of("kind",cls.getName(),"individual",individual,"owned_ends",ends,"non_end_features",1,
                    "observed_owned_ends",owner.getOwnedEndFeature().size(),"general_types",computed));
            }
        }
        SysMLLibraryUtil.setProviderLookup(null);
        Files.writeString(Path.of(args[1]),new GsonBuilder().setPrettyPrinting().create().toJson(Map.of("methods",methods,"bindings",bindings,"controls",controls))+"\n");
    }
}
