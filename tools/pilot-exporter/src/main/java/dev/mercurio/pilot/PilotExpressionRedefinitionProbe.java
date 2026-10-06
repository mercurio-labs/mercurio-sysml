package dev.mercurio.pilot;

import com.google.gson.GsonBuilder;
import com.sun.source.tree.*;
import com.sun.source.util.*;
import javax.lang.model.element.ExecutableElement;
import javax.tools.*;
import java.nio.file.*;
import java.lang.reflect.Method;
import java.util.*;
import org.eclipse.emf.ecore.*;
import org.omg.sysml.lang.sysml.*;
import org.omg.sysml.util.*;

/** Resolved strategy dispatch and selector trees; native algorithms are separately qualified. */
public final class PilotExpressionRedefinitionProbe {
    static void require(boolean condition, String message) {
        if (!condition) throw new IllegalStateException(message);
    }
    static Method method(java.lang.Class<?> type, String name, java.lang.Class<?>... parameters) throws Exception {
        while (type != null) {
            try { return type.getDeclaredMethod(name, parameters); }
            catch (NoSuchMethodException ignored) { type = type.getSuperclass(); }
        }
        throw new IllegalStateException("Missing adapter method " + name);
    }
    static String key(Method method) { return method.getDeclaringClass().getName()+"#"+method.getName(); }
    static Map<String,Object> node(CompilationUnitTree unit, Tree tree, Trees trees) {
        var row=new TreeMap<String,Object>(); row.put("kind",tree.getKind().name());
        var path=TreePath.getPath(unit,tree); var type=trees.getTypeMirror(path);
        if(type!=null) row.put("type",type.toString());
        switch(tree) {
            case BlockTree block -> row.put("statements",block.getStatements().stream().map(t->node(unit,t,trees)).toList());
            case ReturnTree ret -> row.put("expression",node(unit,ret.getExpression(),trees));
            case VariableTree variable -> { row.put("name",variable.getName().toString()); if(variable.getInitializer()!=null) row.put("initializer",node(unit,variable.getInitializer(),trees)); }
            case IfTree branch -> {
                row.put("condition",node(unit,branch.getCondition(),trees)); row.put("then",node(unit,branch.getThenStatement(),trees));
                if(branch.getElseStatement()!=null) row.put("else",node(unit,branch.getElseStatement(),trees));
            }
            case LambdaExpressionTree lambda -> { row.put("parameters",lambda.getParameters().stream().map(t->node(unit,t,trees)).toList()); row.put("body",node(unit,lambda.getBody(),trees)); }
            case UnaryTree unary -> row.put("expression",node(unit,unary.getExpression(),trees));
            case IdentifierTree identifier -> row.put("name",identifier.getName().toString());
            case MemberSelectTree select -> { row.put("name",select.getIdentifier().toString()); row.put("receiver",node(unit,select.getExpression(),trees)); }
            case LiteralTree literal -> row.put("value",literal.getValue());
            case ParenthesizedTree paren -> row.put("expression",node(unit,paren.getExpression(),trees));
            case ConditionalExpressionTree conditional -> {
                row.put("condition",node(unit,conditional.getCondition(),trees));
                row.put("true",node(unit,conditional.getTrueExpression(),trees)); row.put("false",node(unit,conditional.getFalseExpression(),trees));
            }
            case BinaryTree binary -> { row.put("left",node(unit,binary.getLeftOperand(),trees)); row.put("right",node(unit,binary.getRightOperand(),trees)); }
            case InstanceOfTree check -> {
                row.put("expression",node(unit,check.getExpression(),trees));
                row.put("test_type",trees.getTypeMirror(TreePath.getPath(unit,check.getType())).toString());
            }
            case TypeCastTree cast -> {
                row.put("expression",node(unit,cast.getExpression(),trees));
                row.put("cast_type",trees.getTypeMirror(TreePath.getPath(unit,cast.getType())).toString());
            }
            case MethodInvocationTree call -> {
                var symbol=trees.getElement(path); require(symbol instanceof ExecutableElement,"Unresolved invocation");
                var executable=(ExecutableElement)symbol;
                row.put("symbol",executable.getEnclosingElement()+"#"+executable.getSimpleName());
                row.put("parameters",executable.getParameters().stream().map(p->p.asType().toString()).toList());
                row.put("arguments",call.getArguments().stream().map(t->node(unit,t,trees)).toList());
                if(call.getMethodSelect() instanceof MemberSelectTree select) row.put("receiver",node(unit,select.getExpression(),trees));
            }
            default -> throw new IllegalStateException("Unsupported selector syntax "+tree.getKind());
        }
        return row;
    }
    public static void main(String[] args)throws Exception {
        org.omg.sysml.logic.SysMLLogicStandaloneSetup.doSetup();
        var compiler=ToolProvider.getSystemJavaCompiler();var diagnostics=new DiagnosticCollector<JavaFileObject>();
        var methods=new TreeMap<String,Object>();
        try(var files=compiler.getStandardFileManager(diagnostics,null,java.nio.charset.StandardCharsets.UTF_8)) {
            var task=(JavacTask)compiler.getTask(null,files,diagnostics,List.of("-proc:none","-classpath",args[0]),null,files.getJavaFileObjectsFromStrings(Arrays.asList(args).subList(2,args.length)));
            var units=new ArrayList<CompilationUnitTree>();task.parse().forEach(units::add);task.analyze();
            require(diagnostics.getDiagnostics().stream().noneMatch(d->d.getKind()==Diagnostic.Kind.ERROR),"Unresolved source: "+diagnostics.getDiagnostics());
            var trees=Trees.instance(task);
            for(var unit:units)for(var declaration:unit.getTypeDecls())if(declaration instanceof ClassTree type)
                for(var member:type.getMembers())if(member instanceof MethodTree m &&
                    (m.getName().contentEquals("getRelevantFeatures")||m.getName().contentEquals("isTransitionGuard")))
                    methods.put(trees.getElement(TreePath.getPath(unit,type))+"#"+m.getName(),node(unit,m.getBody(),trees));
        }
        require(methods.size()==2,"Missing expression selector/predicate trees");
        var f=SysMLFactory.eINSTANCE;var controls=new ArrayList<Object>();var guards=new ArrayList<Object>();var bindings=new ArrayList<Object>();
        for(var classifier:SysMLPackage.eINSTANCE.getEClassifiers()) {
            if(!(classifier instanceof EClass cls)||cls.isAbstract())continue;
            var sample=f.create(cls);if(!(sample instanceof Expression))continue;
            var adapter=ElementUtil.getElementAdapter((Element)sample);
            if(!key(method(adapter.getClass(),"getRelevantFeatures",Type.class)).equals("org.omg.sysml.adapter.ExpressionAdapter#getRelevantFeatures"))continue;
            bindings.add(cls.getName());
            for(String ownerKind:List.of("detached","Package","Class"))for(int explicit:List.of(0,2)) {
                var expression=(Expression)f.create(cls);
                if(!ownerKind.equals("detached")) {var owner=(Namespace)f.create((EClass)SysMLPackage.eINSTANCE.getEClassifier(ownerKind));var m=f.createFeatureMembership();m.getOwnedRelatedElement().add(expression);owner.getOwnedRelationship().add(m);}
                var ids=new IdentityHashMap<Feature,String>();
                for(int i=0;i<explicit;i++){var general=f.createFeature();general.setDeclaredName("General"+i);ids.put(general,"general"+i);var r=f.createRedefinition();r.setRedefiningFeature(expression);r.setRedefinedFeature(general);expression.getOwnedRelationship().add(r);}
                var row=new TreeMap<String,Object>();row.put("kind",cls.getName());row.put("owner_kind",ownerKind);row.put("explicit",explicit);
                row.put("redefined_features",FeatureUtil.getRedefinedFeaturesWithComputedOf(expression).stream().map(g->Objects.requireNonNull(ids.get(g),"Unexpected computed endpoint")).toList());
                row.put("effective_name",expression.effectiveName());row.put("effective_short_name",expression.effectiveShortName());
                row.put("transition_guard",ExpressionUtil.isTransitionGuard(expression));
                row.put("owning_type_is_feature_value",expression.getOwningType() instanceof FeatureValue);controls.add(row);
            }
            for(var kind:TransitionFeatureKind.values()) {
                var expression=(Expression)f.create(cls);var owner=f.createTransitionUsage();var m=f.createTransitionFeatureMembership();m.setKind(kind);m.getOwnedRelatedElement().add(expression);owner.getOwnedRelationship().add(m);
                guards.add(Map.of("kind",cls.getName(),"membership_kind",kind.getLiteral(),"transition_guard",ExpressionUtil.isTransitionGuard(expression)));
            }
        }
        bindings.sort((a,b)->a.toString().compareTo(b.toString()));
        Files.writeString(Path.of(args[1]),new GsonBuilder().serializeNulls().setPrettyPrinting().create().toJson(Map.of("methods",methods,"bindings",bindings,"controls",controls,"guard_controls",guards))+"\n");
    }
}
