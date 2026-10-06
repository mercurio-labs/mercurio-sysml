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

/** Mixed-category parameter selection; supplied factory states are not full model qualification. */
public final class PilotMixedParameterProbe {
    static void require(boolean condition,String message){if(!condition)throw new IllegalStateException(message);}
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
            case AssignmentTree assignment -> { row.put("left",node(unit,assignment.getVariable(),trees));row.put("right",node(unit,assignment.getExpression(),trees)); }
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

    static final SysMLFactory f=SysMLFactory.eINSTANCE;
    static final class Model {
        final Map<String,Type> types=new LinkedHashMap<>();final Map<String,Feature> features=new LinkedHashMap<>();
        final Map<Element,String> ids=new IdentityHashMap<>();final List<Object> typeRows=new ArrayList<>(),featureRows=new ArrayList<>();
        void type(String id,String kind,String...parents) {
            Type t=(Type)f.create((EClass)SysMLPackage.eINSTANCE.getEClassifier(kind));t.setIsImpliedIncluded(true);types.put(id,t);ids.put(t,id);
            var relations=new ArrayList<String>();
            for(String parent:parents){Type g=types.get(parent);Specialization r;
                if(t instanceof Feature tf && g instanceof Feature gf){var x=f.createSubsetting();x.setSubsettingFeature(tf);x.setSubsettedFeature(gf);r=x;}
                else if(t instanceof Feature tf && g instanceof Classifier gc){var x=f.createFeatureTyping();x.setTypedFeature(tf);x.setType(gc);r=x;}
                else if(t instanceof Classifier tc && g instanceof Classifier gc){var x=f.createSubclassification();x.setSubclassifier(tc);x.setSuperclassifier(gc);r=x;}
                else {r=f.createSpecialization();r.setSpecific(t);r.setGeneral(g);}
                t.getOwnedRelationship().add(r);relations.add(r.eClass().getName());}
            typeRows.add(Map.of("id",id,"kind",kind,"parents",List.of(parents),"general_kinds",relations));
        }
        void feature(String id,String kind,String owner,String direction,boolean result,String visibility) {
            Feature x=(Feature)f.create((EClass)SysMLPackage.eINSTANCE.getEClassifier(kind));x.setDirection(direction==null?null:FeatureDirectionKind.get(direction));x.setDeclaredName(id);x.setDeclaredShortName("s_"+id);
            FeatureMembership m=result?f.createReturnParameterMembership():f.createFeatureMembership();m.setVisibility(VisibilityKind.get(visibility));m.getOwnedRelatedElement().add(x);types.get(owner).getOwnedRelationship().add(m);features.put(id,x);ids.put(x,id);
            var row=new TreeMap<String,Object>();row.put("id",id);row.put("kind",kind);row.put("owner",owner);row.put("direction",direction);row.put("result",result);row.put("is_end",false);row.put("visibility",visibility);row.put("name",id);featureRows.add(row);
        }
        List<String> names(Collection<? extends Element> values){return values.stream().map(x->Objects.requireNonNull(ids.get(x),"Unexpected generated endpoint: "+x.eClass().getName())).toList();}
    }
    static Object control(String binding,String ownerKind,String generalKind,String shape,int position) {
        var m=new Model();String base=generalKind;
        m.type("base",base);
        if(!shape.equals("empty")){
            String visibility=shape.equals("private")?"private":shape.equals("protected")?"protected":"public";
            m.feature("base0","Feature","base","in",false,visibility);m.feature("base1","Feature","base","out",false,visibility);
            m.feature("baseResult","Feature","base","out",m.types.get("base") instanceof org.omg.sysml.lang.sysml.Function || m.types.get("base") instanceof Expression,visibility);
            m.feature("ordinaryBase","Feature","base",null,false,"public");
        }
        if(shape.equals("inherited")||shape.equals("private")||shape.equals("protected")){m.type("middle","Class","base");m.type("owner",ownerKind,"middle");}
        else if(shape.equals("diamond")){m.type("left","Class","base");m.type("right","Class","base");m.type("owner",ownerKind,"left","right");}
        else m.type("owner",ownerKind,"base");
        for(int i=0;i<3;i++)m.feature("local"+i,binding,"owner",List.of("in","out","inout").get(i),false,"public");
        if(position==3)m.feature("localResult",binding,"owner","out",true,"public");
        var target=m.features.get(position==3?"localResult":"local"+position);
        var collections=new TreeMap<String,Object>();var results=new TreeMap<String,Object>();
        for(var e:m.types.entrySet()){
            collections.put(e.getKey(),Map.of("owned",m.names(TypeUtil.getOwnedParametersOf(e.getValue())),"all",m.names(TypeUtil.getAllParametersOf(e.getValue()))));
            var r=TypeUtil.getResultParameterOf(e.getValue());results.put(e.getKey(),r==null?null:m.ids.get(r));
        }
        return new TreeMap<>(Map.of("kind",binding,"owner_kind",ownerKind,"general_kind",generalKind,"shape",shape,"position",position,"types",m.typeRows,"features",m.featureRows,"explicit",List.of(),"target",m.ids.get(target),"observations",Map.of("collections",collections,"results",results,"redefined_features",m.names(FeatureUtil.getRedefinedFeaturesWithComputedOf(target)))));
    }
    public static void main(String[] args)throws Exception {
        org.omg.sysml.logic.SysMLLogicStandaloneSetup.doSetup();SysMLLibraryUtil.setProviderLookup(r->(c,n)->null);
        var compiler=ToolProvider.getSystemJavaCompiler();var diagnostics=new DiagnosticCollector<JavaFileObject>();var methods=new TreeMap<String,Object>();
        try(var files=compiler.getStandardFileManager(diagnostics,null,java.nio.charset.StandardCharsets.UTF_8)){
            var task=(JavacTask)compiler.getTask(null,files,diagnostics,List.of("-proc:none","-classpath",args[0]),null,files.getJavaFileObjectsFromStrings(Arrays.asList(args).subList(2,args.length)));var units=new ArrayList<CompilationUnitTree>();task.parse().forEach(units::add);task.analyze();
            require(diagnostics.getDiagnostics().stream().noneMatch(d->d.getKind()==Diagnostic.Kind.ERROR),"Unresolved source");var trees=Trees.instance(task);
            for(var unit:units)for(var declaration:unit.getTypeDecls())if(declaration instanceof ClassTree cls)for(var member:cls.getMembers())if(member instanceof MethodTree method && ((cls.getSimpleName().contentEquals("FeatureUtil") && method.getName().contentEquals("isParameter")) || (cls.getSimpleName().contentEquals("TypeUtil") && Set.of("getOwnedParametersOf","getAllParametersOf","getOwnedResultParameterOf").contains(method.getName().toString()))))methods.put(trees.getElement(TreePath.getPath(unit,cls))+"#"+method.getName(),node(unit,method.getBody(),trees));
        }
        var bindings=new TreeMap<String,Object>();
        for(var classifier:SysMLPackage.eINSTANCE.getEClassifiers()){
            if(!(classifier instanceof EClass cls)||cls.isAbstract())continue;var sample=f.create(cls);if(!(sample instanceof Feature feature))continue;
            var adapter=ElementUtil.getElementAdapter(feature);var dispatch=new TreeMap<String,String>();
            for(String name:List.of("addRedefinitions","addFeatureWriteTypes","addComputedRedefinitions","isComputeRedefinitions","getRedefinedFeaturesWithComputed","getRelevantFeatures","getGeneralTypes","getParameterRelevantFeatures","getRelevantParameters","filterIgnoredParameters")){
                java.lang.Class<?>[] params=name.equals("addRedefinitions")||name.equals("addComputedRedefinitions")?new java.lang.Class<?>[]{Element.class}:name.equals("getRelevantFeatures")||name.equals("getParameterRelevantFeatures")||name.equals("getRelevantParameters")?new java.lang.Class<?>[]{Type.class}:name.equals("getGeneralTypes")?new java.lang.Class<?>[]{Type.class,Element.class}:name.equals("filterIgnoredParameters")?new java.lang.Class<?>[]{List.class}:new java.lang.Class<?>[]{};
                dispatch.put(name,PilotFeatureRedefinitionExporter.key(PilotFeatureRedefinitionExporter.method(adapter.getClass(),name,params)));
            }
            if(PilotFeatureRedefinitionExporter.bounded(dispatch) && List.of("getGeneralTypes","getParameterRelevantFeatures","getRelevantParameters","filterIgnoredParameters").stream().allMatch(n->dispatch.get(n).equals("org.omg.sysml.adapter.FeatureAdapter#"+n)))bindings.put(cls.getName(),dispatch);
        }
        var controls=new ArrayList<Object>();
        for(String owner:List.of("Behavior","Function","Class","Step","Expression"))for(String general:List.of("Type","Classifier","Class","Behavior","Function","Predicate","Step","Expression"))for(String shape:List.of("direct","empty","inherited","private","protected","diamond"))for(int position=0;position<4;position++){
            if(position==3 && !Set.of("Function","Expression").contains(owner))continue;
            controls.add(control("Feature",owner,general,shape,position));
        }
        for(String binding:bindings.keySet())if(!binding.equals("Feature"))for(String shape:List.of("direct","empty","inherited"))for(int position=0;position<4;position++)controls.add(control(binding,"Function","Class",shape,position));
        Files.writeString(Path.of(args[1]),new GsonBuilder().serializeNulls().setPrettyPrinting().create().toJson(Map.of("methods",methods,"bindings",bindings,"controls",controls))+"\n");
    }
}
