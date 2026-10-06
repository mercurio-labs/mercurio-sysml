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
public final class PilotStepStrategyProbe {
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
            case ExpressionStatementTree statement -> row.put("expression",node(unit,statement.getExpression(),trees));
            case ReturnTree ret -> row.put("expression",node(unit,ret.getExpression(),trees));
            case VariableTree variable -> { row.put("name",variable.getName().toString()); if(variable.getInitializer()!=null) row.put("initializer",node(unit,variable.getInitializer(),trees)); }
            case IfTree branch -> {
                row.put("condition",node(unit,branch.getCondition(),trees)); row.put("then",node(unit,branch.getThenStatement(),trees));
                if(branch.getElseStatement()!=null) row.put("else",node(unit,branch.getElseStatement(),trees));
            }
            case LambdaExpressionTree lambda -> { row.put("parameters",lambda.getParameters().stream().map(t->node(unit,t,trees)).toList()); row.put("body",node(unit,lambda.getBody(),trees)); }
            case MemberReferenceTree reference -> { row.put("name",reference.getName().toString()); row.put("receiver",node(unit,reference.getQualifierExpression(),trees)); }
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
                for(var member:type.getMembers())if(member instanceof MethodTree m && type.getSimpleName().contentEquals("StepAdapter") && Set.of("getDefaultSupertype","isIncomingTransfer").contains(m.getName().toString()))
                    methods.put(trees.getElement(TreePath.getPath(unit,type))+"#"+m.getName(),node(unit,m.getBody(),trees));
        }
        require(methods.size()==2,"Missing Step selector/predicate trees");
        var f=SysMLFactory.eINSTANCE;var controls=new ArrayList<Object>();var bindings=new TreeMap<String,Object>();var typing=new ArrayList<Object>();
        for(var classifier:SysMLPackage.eINSTANCE.getEClassifiers()) {
            if(!(classifier instanceof EClass cls)||cls.isAbstract())continue;
            var sample=f.create(cls);if(!(sample instanceof Step probe))continue;
            var adapter=ElementUtil.getElementAdapter(probe);
            if(!key(method(adapter.getClass(),"getDefaultSupertype")).equals("org.omg.sysml.adapter.StepAdapter#getDefaultSupertype"))continue;
            var names=new TreeMap<String,String>();var dispatch=new TreeMap<String,String>();
            for(String name:List.of("base","ownedPerformance","subperformance","enclosedPerformance","incomingTransfer")) {
                var value=ImplicitGeneralizationMap.getDefaultSupertypeFor(probe.getClass(),name);require(value!=null,"Missing Step mapping "+name);names.put(name,value);
            }
            for(String name:List.of("getAllTypes","getTypes","getFeatureTypes","getGeneralTypes","addDefaultGeneralType","getRelevantFeatures")) {
                java.lang.Class<?>[] parameters=name.equals("getTypes")||name.equals("getFeatureTypes")?new java.lang.Class<?>[]{List.class,Set.class}:name.equals("getGeneralTypes")?new java.lang.Class<?>[]{Type.class,Element.class}:name.equals("getRelevantFeatures")?new java.lang.Class<?>[]{Type.class}:new java.lang.Class<?>[]{};
                dispatch.put(name,key(method(adapter.getClass(),name,parameters)));
            }
            bindings.put(cls.getName(),Map.of("names",names,"dispatch",dispatch));
            for(String context:List.of("detached","Class","Structure","Behavior","Step","Feature_empty","Feature_structure"))for(boolean composite:List.of(false,true))for(String payload:List.of("none","ordinary","owned","alias")) {
                var step=(Step)f.create(cls);step.setIsComposite(composite);
                if(!context.equals("detached")) {
                    var owner=(Type)f.create((EClass)SysMLPackage.eINSTANCE.getEClassifier(context.startsWith("Feature_")?"Feature":context));owner.setIsImpliedIncluded(true);
                    if(context.equals("Feature_structure")){var target=f.createStructure();target.setIsImpliedIncluded(true);var relation=f.createFeatureTyping();relation.setTypedFeature((Feature)owner);relation.setType(target);owner.getOwnedRelationship().add(relation);}
                    var membership=f.createFeatureMembership();membership.getOwnedRelatedElement().add(step);owner.getOwnedRelationship().add(membership);
                }
                if(!payload.equals("none")) {
                    var target=payload.equals("ordinary")?f.createFeature():f.createPayloadFeature();target.setIsImpliedIncluded(true);
                    if(payload.equals("alias")){var membership=f.createMembership();membership.setMemberElement(target);step.getOwnedRelationship().add(membership);}
                    else {var membership=f.createFeatureMembership();membership.getOwnedRelatedElement().add(target);step.getOwnedRelationship().add(membership);}
                }
                var library=new HashMap<String,Element>();SysMLLibraryUtil.setProviderLookup(r -> (c,n)->library.computeIfAbsent(n,key->{var target=f.createFeature();target.setElementId(key);target.setIsImpliedIncluded(true);return target;}));
                var current=(org.omg.sysml.adapter.TypeAdapter)ElementUtil.getElementAdapter(step);current.addDefaultGeneralType();var generals=new ArrayList<Object>();current.forEachImplicitGeneralType((k,g)->generals.add(Map.of("kind",k.getName(),"target",g.getElementId())));
                controls.add(Map.of("kind",cls.getName(),"context",context,"composite",composite,"payload",payload,"generals",generals));
            }
            {
                var step=f.createStep();step.setElementId("step");var general=f.createFeature();general.setElementId("general");general.setIsImpliedIncluded(true);
                var target=f.createStructure();target.setElementId("type");target.setIsImpliedIncluded(true);var relation=f.createFeatureTyping();relation.setTypedFeature(general);relation.setType(target);general.getOwnedRelationship().add(relation);
                SysMLLibraryUtil.setProviderLookup(r -> (c,n)->n.equals("Performances::performances")?general:null);
                typing.add(Map.of("kind","Step","shape","implicit_default","types",step.getType().stream().map(Element::getElementId).toList()));
            }
            for(String shape:List.of("empty","direct","subset","cycle","diamond")) {
                // Isolate reference observations: the preceding incomplete case
                // supplies a typed library default, while these stored-query
                // cases require an explicitly untyped default environment.
                var untyped=new HashMap<String,Element>();
                SysMLLibraryUtil.setProviderLookup(r -> (c,n)->untyped.computeIfAbsent(n,key->{var endpoint=f.createFeature();endpoint.setElementId(key);endpoint.setIsImpliedIncluded(true);return endpoint;}));
                var step=(Step)f.create(cls);step.setElementId("step");step.setIsImpliedIncluded(true);var target=f.createStructure();target.setElementId("type");target.setIsImpliedIncluded(true);
                var general=f.createFeature();general.setElementId("general");general.setIsImpliedIncluded(true);
                if(!shape.equals("empty")){var relation=f.createFeatureTyping();relation.setTypedFeature(shape.equals("direct")?step:general);relation.setType(target);(shape.equals("direct")?step:general).getOwnedRelationship().add(relation);}
                if(!shape.equals("empty")&&!shape.equals("direct")){var relation=f.createSubsetting();relation.setSubsettingFeature(step);relation.setSubsettedFeature(general);step.getOwnedRelationship().add(relation);}
                if(shape.equals("cycle")){var relation=f.createSubsetting();relation.setSubsettingFeature(general);relation.setSubsettedFeature(step);general.getOwnedRelationship().add(relation);}
                if(shape.equals("diamond")){var relation=f.createSubsetting();relation.setSubsettingFeature(step);relation.setSubsettedFeature(general);step.getOwnedRelationship().add(relation);}
                typing.add(Map.of("kind",cls.getName(),"shape",shape,"types",step.getType().stream().map(Element::getElementId).toList()));
            }
        }
        Files.writeString(Path.of(args[1]),new GsonBuilder().serializeNulls().setPrettyPrinting().create().toJson(Map.of("methods",methods,"bindings",bindings,"controls",controls,"typing_controls",typing))+"\n");
    }
}
