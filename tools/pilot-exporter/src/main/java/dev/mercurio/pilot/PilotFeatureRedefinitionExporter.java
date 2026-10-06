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
public final class PilotFeatureRedefinitionExporter {
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
            case TypeCastTree cast -> row.put("expression",node(unit,cast.getExpression(),trees));
            case BinaryTree binary -> { row.put("left",node(unit,binary.getLeftOperand(),trees)); row.put("right",node(unit,binary.getRightOperand(),trees)); }
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
    static boolean bounded(Map<String,String> dispatch) {
        for(String name:List.of("addRedefinitions","addFeatureWriteTypes","addComputedRedefinitions","isComputeRedefinitions","getRedefinedFeaturesWithComputed"))
            if(!dispatch.get(name).equals("org.omg.sysml.adapter.FeatureAdapter#"+name)) return false;
        return List.of("org.omg.sysml.adapter.FeatureAdapter#getRelevantFeatures","org.omg.sysml.adapter.MultiplicityAdapter#getRelevantFeatures").contains(dispatch.get("getRelevantFeatures"));
    }
    // Input topology is exported separately from independently executed results.
    static final class EndModel {
        final boolean parameters;
        EndModel() { this(false); }
        EndModel(boolean parameters) { this.parameters=parameters; }
        final SysMLFactory factory = SysMLFactory.eINSTANCE;
        final Map<String,org.omg.sysml.lang.sysml.Class> types = new LinkedHashMap<>();
        final Map<String,Feature> features = new LinkedHashMap<>();
        final Map<Feature,String> ids = new IdentityHashMap<>();
        final List<Object> typeRows = new ArrayList<>(), featureRows = new ArrayList<>(), redefinitions = new ArrayList<>();
        void type(String id, String... parents) {
            org.omg.sysml.lang.sysml.Class type = parameters ? factory.createFunction() : factory.createClass(); type.setIsImpliedIncluded(true); types.put(id,type);
            for (String parent : parents) {
                var specialization = factory.createSubclassification(); specialization.setSubclassifier(type);
                specialization.setSuperclassifier(types.get(parent)); type.getOwnedRelationship().add(specialization);
            }
            typeRows.add(Map.of("id",id,"kind",type.eClass().getName(),"parents",List.of(parents)));
        }
        Feature feature(String id, EClass cls, String owner, boolean end, String visibility, String name) {
            var feature = (Feature)factory.create(cls); feature.setIsEnd(end); feature.setDeclaredName(name);
            feature.setDeclaredShortName(name == null ? null : "short_"+name);
            var membership = factory.createFeatureMembership(); membership.setVisibility(VisibilityKind.get(visibility));
            membership.setOwnedMemberFeature(feature); types.get(owner).getOwnedRelationship().add(membership);
            features.put(id,feature); ids.put(feature,id);
            var row = new TreeMap<String,Object>(); row.put("id",id); row.put("kind",cls.getName()); row.put("owner",owner);
            row.put("is_end",end); row.put("visibility",visibility); row.put("name",name);
            featureRows.add(row); return feature;
        }
        Feature parameter(String id, EClass cls, String owner, String direction, boolean result, String visibility, String name) {
            var feature=(Feature)factory.create(cls); feature.setIsEnd(false);
            feature.setDirection(direction==null?null:FeatureDirectionKind.get(direction)); feature.setDeclaredName(name);
            feature.setDeclaredShortName(name==null?null:"short_"+name);
            FeatureMembership membership=result?factory.createReturnParameterMembership():factory.createFeatureMembership();
            membership.setVisibility(VisibilityKind.get(visibility)); membership.setOwnedMemberFeature(feature);
            types.get(owner).getOwnedRelationship().add(membership); features.put(id,feature); ids.put(feature,id);
            var row=new TreeMap<String,Object>(); row.put("id",id); row.put("kind",cls.getName()); row.put("owner",owner);
            row.put("direction",direction); row.put("result",result); row.put("is_end",false); row.put("visibility",visibility); row.put("name",name);
            featureRows.add(row); return feature;
        }
        @SuppressWarnings("unchecked")
        void chain(String id, String... targets) {
            for (String target : targets) FeatureUtil.addChainingFeature(features.get(id), features.get(target));
            for (Object row : featureRows) {
                var fields=(Map<String,Object>)row;
                if (id.equals(fields.get("id"))) fields.put("chain",List.of(targets));
            }
        }
        void redefine(Feature feature, Feature general) {
            var redefinition = factory.createRedefinition(); redefinition.setRedefiningFeature(feature);
            redefinition.setRedefinedFeature(general); feature.getOwnedRelationship().add(redefinition);
            redefinitions.add(Map.of("specific",ids.get(feature),"general",ids.get(general)));
        }
    }
    static Object endControl(EClass cls, String shape, int position) {
        var m = new EndModel(); var featureClass = SysMLPackage.Literals.FEATURE;
        m.type("base");
        int size = shape.equals("short") ? 1 : 2;
        for (int i=0;i<size;i++) m.feature("base"+i,featureClass,"base",true,
                shape.equals("private") ? "private" : shape.equals("protected") ? "protected" : "public", "Base"+i);
        // Ordinary members must never change an end's positional index.
        m.feature("ordinaryBase",featureClass,"base",false,"public","Ordinary");
        switch (shape) {
            case "inherited", "private", "protected" -> { m.type("middle","base"); m.type("owner","middle"); }
            case "diamond" -> { m.type("left","base"); m.type("right","base"); m.type("owner","left","right"); }
            case "multiple", "chain_equal", "chain_distinct" -> {
                m.type("other");
                for(int i=0;i<2;i++) m.feature("other"+i,featureClass,"other",true,"public","Other"+i);
                m.type("owner","base","other");
            }
            case "duplicate" -> m.type("owner","base","base");
            default -> m.type("owner","base");
        }
        m.feature("ordinary",featureClass,"owner",false,"public","OrdinaryLocal");
        Feature target=null;
        for(int i=0;i<3;i++) {
            var f=m.feature("local"+i,cls,"owner",true,"public",i==position?null:"Local"+i);
            if(i==position) target=f;
        }
        if(shape.startsWith("chain_")) {
            m.chain("base0","ordinaryBase","ordinary");
            if(shape.equals("chain_equal")) m.chain("other0","ordinaryBase","ordinary");
            else m.chain("other0","ordinary","ordinaryBase");
        }
        if(shape.equals("explicit")) m.redefine(target,m.features.get("base0"));
        var refs=FeatureUtil.getRedefinedFeaturesWithComputedOf(target).stream().map(f->Objects.requireNonNull(m.ids.get(f),"Unexpected end endpoint")).toList();
        var row=new TreeMap<String,Object>(); row.put("kind",cls.getName()); row.put("shape",shape); row.put("position",position);
        row.put("types",m.typeRows); row.put("features",m.featureRows); row.put("explicit",m.redefinitions); row.put("target",m.ids.get(target));
        row.put("redefined_features",refs); row.put("effective_name",target.effectiveName()); row.put("effective_short_name",target.effectiveShortName());
        var effectiveEnds=new TreeMap<String,Object>();
        for(var entry:m.types.entrySet()) effectiveEnds.put(entry.getKey(),TypeUtil.getEndFeatureOf(entry.getValue()).stream().map(f->Objects.requireNonNull(m.ids.get(f))).toList());
        row.put("effective_ends",effectiveEnds);
        return row;
    }
    static Object parameterControl(EClass cls, String shape, int position) {
        var m=new EndModel(true); var featureClass=SysMLPackage.Literals.FEATURE;
        m.type("base");
        if(!shape.equals("no_result")) m.parameter("baseResult",featureClass,"base","out",true,"public","BaseResult");
        int size=shape.equals("short")?1:2;
        String visibility=shape.equals("private")?"private":shape.equals("protected")?"protected":"public";
        for(int i=0;i<size;i++) m.parameter("base"+i,featureClass,"base",i==0?"in":"out",false,visibility,"Base"+i);
        m.parameter("ordinaryBase",featureClass,"base",null,false,"public","Ordinary");
        switch(shape) {
            case "inherited", "private", "protected" -> { m.type("middle","base"); m.type("owner","middle"); }
            case "diamond" -> { m.type("left","base"); m.type("right","base"); m.type("owner","left","right"); }
            case "multiple", "chain_equal", "chain_distinct" -> {
                m.type("other"); m.parameter("otherResult",featureClass,"other","out",true,"public","OtherResult");
                for(int i=0;i<2;i++) m.parameter("other"+i,featureClass,"other","inout",false,"public","Other"+i);
                m.type("owner","base","other");
            }
            default -> m.type("owner","base");
        }
        if(shape.equals("result_first")) m.parameter("localResult",cls,"owner","out",true,"public",position==3?null:"LocalResult");
        m.parameter("ordinary",featureClass,"owner",null,false,"public","OrdinaryLocal");
        for(int i=0;i<3;i++) m.parameter("local"+i,cls,"owner",List.of("in","out","inout").get(i),false,"public",position==i?null:"Local"+i);
        if(!shape.equals("result_first")) m.parameter("localResult",cls,"owner","out",true,"public",position==3?null:"LocalResult");
        var target=m.features.get(position==3?"localResult":"local"+position);
        if(shape.startsWith("chain_")) {
            m.chain("base0","ordinaryBase","ordinary");
            if(shape.equals("chain_equal")) m.chain("other0","ordinaryBase","ordinary");
            else m.chain("other0","ordinary","ordinaryBase");
        }
        if(shape.equals("explicit")) m.redefine(target,m.features.get("base0"));
        var refs=FeatureUtil.getRedefinedFeaturesWithComputedOf(target).stream().map(f->Objects.requireNonNull(m.ids.get(f))).toList();
        var row=new TreeMap<String,Object>(); row.put("kind",cls.getName()); row.put("shape",shape); row.put("position",position);
        row.put("types",m.typeRows); row.put("features",m.featureRows); row.put("explicit",m.redefinitions); row.put("target",m.ids.get(target));
        row.put("redefined_features",refs); row.put("effective_name",target.effectiveName()); row.put("effective_short_name",target.effectiveShortName());
        var effective=new TreeMap<String,Object>(); var results=new TreeMap<String,Object>();
        for(var entry:m.types.entrySet()) {
            effective.put(entry.getKey(),TypeUtil.getFeatureOf(entry.getValue()).stream().map(f->Objects.requireNonNull(m.ids.get(f))).toList());
            var result=TypeUtil.getResultParameterOf(entry.getValue()); results.put(entry.getKey(),result==null?null:Objects.requireNonNull(m.ids.get(result)));
        }
        row.put("effective_features",effective); row.put("results",results); return row;
    }
    static Object invocationControl(EClass cls) {
        var f=SysMLFactory.eINSTANCE; var owner=f.createInvocationExpression();
        var feature=(Feature)f.create(cls); feature.setIsEnd(false); feature.setDirection(FeatureDirectionKind.IN);
        var member=f.createParameterMembership(); member.setOwnedMemberParameter(feature); owner.getOwnedRelationship().add(member);
        var general=f.createFeature(); general.setDeclaredName("Explicit");
        var redefinition=f.createRedefinition(); redefinition.setRedefiningFeature(feature); redefinition.setRedefinedFeature(general);
        feature.getOwnedRelationship().add(redefinition);
        var refs=FeatureUtil.getRedefinedFeaturesWithComputedOf(feature);
        require(refs.size()==1 && refs.get(0)==general,"Invocation suppression changed");
        return Map.of("kind",cls.getName(),"redefined_features",List.of("general"));
    }
    public static void main(String[] args) throws Exception {
        org.omg.sysml.logic.SysMLLogicStandaloneSetup.doSetup();
        var methods=new TreeMap<String,Object>();
        var compiler=ToolProvider.getSystemJavaCompiler(); var diagnostics=new DiagnosticCollector<JavaFileObject>();
        try(var files=compiler.getStandardFileManager(diagnostics,null,java.nio.charset.StandardCharsets.UTF_8)) {
            var task=(JavacTask)compiler.getTask(null,files,diagnostics,List.of("-proc:none","-classpath",args[0]),null,files.getJavaFileObjectsFromStrings(Arrays.asList(args).subList(2,args.length)));
            var units=new ArrayList<CompilationUnitTree>(); task.parse().forEach(units::add); task.analyze();
            require(diagnostics.getDiagnostics().stream().noneMatch(d->d.getKind()==Diagnostic.Kind.ERROR),"Unresolved source: "+diagnostics.getDiagnostics());
            var trees=Trees.instance(task);
            for(var unit:units) for(var declaration:unit.getTypeDecls()) if(declaration instanceof ClassTree type)
                for(var member:type.getMembers()) if(member instanceof MethodTree method && (method.getName().contentEquals("getRelevantFeatures") || method.getName().contentEquals("getEndRelevantFeatures") || Set.of("getParameterRelevantFeatures","getRelevantParameters","filterIgnoredParameters","isIgnoredParameter").contains(method.getName().toString()) || method.getName().contentEquals("addAdditionalMembers")))
                    methods.put(trees.getElement(TreePath.getPath(unit,type))+"#"+method.getName(),node(unit,method.getBody(),trees));
        }
        require(methods.size()==8,"Missing relevant-feature selector trees");
        var bindings=new ArrayList<Object>(); var memberDispatch=new TreeMap<String,String>(); var controls=new ArrayList<Object>(); var endControls=new ArrayList<Object>(); var parameterControls=new ArrayList<Object>(); var invocationControls=new ArrayList<Object>();
        var factory=SysMLFactory.eINSTANCE;
        for(var classifier:SysMLPackage.eINSTANCE.getEClassifiers()) {
            if(!(classifier instanceof EClass cls) || cls.isAbstract()) continue;
            var sample=(Element)factory.create(cls); var adapter=ElementUtil.getElementAdapter(sample);
            if(sample instanceof Type) memberDispatch.put(cls.getName(),key(method(adapter.getClass(),"addAdditionalMembers")));
            if(!(sample instanceof Feature)) continue;
            var dispatch=new TreeMap<String,String>();
            for(String name:List.of("addRedefinitions","addComputedRedefinitions")) dispatch.put(name,key(method(adapter.getClass(),name,Element.class)));
            dispatch.put("getRelevantFeatures",key(method(adapter.getClass(),"getRelevantFeatures",Type.class)));
            dispatch.put("getEndRelevantFeatures",key(method(adapter.getClass(),"getEndRelevantFeatures",Type.class)));
            for(String name:List.of("getParameterRelevantFeatures","getRelevantParameters")) dispatch.put(name,key(method(adapter.getClass(),name,Type.class)));
            dispatch.put("filterIgnoredParameters",key(method(adapter.getClass(),"filterIgnoredParameters",List.class)));
            dispatch.put("isIgnoredParameter",key(method(adapter.getClass(),"isIgnoredParameter")));
            dispatch.put("getGeneralTypes",key(method(adapter.getClass(),"getGeneralTypes",Type.class,Element.class)));
            for(String name:List.of("addFeatureWriteTypes","isComputeRedefinitions","getRedefinedFeaturesWithComputed")) dispatch.put(name,key(method(adapter.getClass(),name)));
            dispatch.put("computeFeaturingType", key(method(adapter.getClass(), "computeFeaturingType")));
            dispatch.put("isVariableGetter", key(sample.getClass().getMethod("isVariable")));
            if (sample instanceof Usage) dispatch.put("mayTimeVaryGetter", key(sample.getClass().getMethod("isMayTimeVary")));
            dispatch.put("owningTypeGetter", key(sample.getClass().getMethod("getOwningType")));
            bindings.add(Map.of("kind",cls.getName(),"adapter",adapter.getClass().getName(),"methods",dispatch));
            if(!bounded(dispatch)) continue;
            for(String shape:List.of("direct","short","multiple","inherited","diamond","private","protected","explicit","duplicate","chain_equal","chain_distinct"))
                for(int position=0;position<3;position++) endControls.add(endControl(cls,shape,position));
            for(String shape:List.of("direct","short","multiple","inherited","diamond","private","protected","explicit","result_first","no_result","chain_equal","chain_distinct"))
                for(int position=0;position<4;position++) parameterControls.add(parameterControl(cls,shape,position));
            invocationControls.add(invocationControl(cls));
            for(String ownerKind:List.of("detached","Package","Class")) for(int explicit:List.of(0,2)) {
                var feature=(Feature)factory.create(cls);
                if(!ownerKind.equals("detached")) {
                    var owner=(Namespace)factory.create((EClass)SysMLPackage.eINSTANCE.getEClassifier(ownerKind));
                    var membership=factory.createFeatureMembership(); membership.setOwnedMemberFeature(feature); owner.getOwnedRelationship().add(membership);
                }
                var ids=new IdentityHashMap<Feature,String>();
                for(int i=0;i<explicit;i++) {
                    var general=factory.createFeature(); general.setDeclaredName("General"+i); ids.put(general,"general"+i);
                    var relation=factory.createRedefinition(); relation.setRedefiningFeature(feature); relation.setRedefinedFeature(general); feature.getOwnedRelationship().add(relation);
                }
                var refs=FeatureUtil.getRedefinedFeaturesWithComputedOf(feature).stream().map(f->Objects.requireNonNull(ids.get(f),"Unexpected computed endpoint")).toList();
                var row=new TreeMap<String,Object>(); row.put("kind",cls.getName()); row.put("owner_kind",ownerKind); row.put("explicit",explicit);
                row.put("redefined_features",refs); row.put("effective_name",feature.effectiveName()); row.put("effective_short_name",feature.effectiveShortName()); controls.add(row);
            }
        }
        Files.writeString(Path.of(args[1]),new GsonBuilder().serializeNulls().setPrettyPrinting().create().toJson(Map.of("participant_default",ImplicitGeneralizationMap.getDefaultSupertypeFor(factory.createFeature().getClass(),"participant"),"methods",methods,"bindings",bindings,"additional_members_dispatch",memberDispatch,"controls",controls,"end_controls",endControls,"parameter_controls",parameterControls,"invocation_controls",invocationControls))+"\n");
    }
}
