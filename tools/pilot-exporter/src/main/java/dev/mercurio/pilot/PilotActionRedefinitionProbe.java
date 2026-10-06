package dev.mercurio.pilot;

import com.google.gson.GsonBuilder;
import com.sun.source.tree.*;
import com.sun.source.util.*;
import javax.tools.*;
import java.nio.file.*;
import java.util.*;
import org.eclipse.emf.ecore.*;
import org.omg.sysml.lang.sysml.*;
import org.omg.sysml.util.*;
import static dev.mercurio.pilot.PilotExpressionRedefinitionProbe.*;

/** Resolved Action strategy and independent membership selectors; no release qualification. */
public final class PilotActionRedefinitionProbe {
    public static void main(String[] args) throws Exception {
        org.omg.sysml.logic.SysMLLogicStandaloneSetup.doSetup();
        var methods=new TreeMap<String,Object>();
        var compiler=ToolProvider.getSystemJavaCompiler();var diagnostics=new DiagnosticCollector<JavaFileObject>();
        try(var files=compiler.getStandardFileManager(diagnostics,null,java.nio.charset.StandardCharsets.UTF_8)) {
            var task=(JavacTask)compiler.getTask(null,files,diagnostics,List.of("-proc:none","-classpath",args[0]),null,files.getJavaFileObjectsFromStrings(Arrays.asList(args).subList(2,args.length)));
            var units=new ArrayList<CompilationUnitTree>();task.parse().forEach(units::add);task.analyze();var trees=Trees.instance(task);
            require(diagnostics.getDiagnostics().stream().noneMatch(d->d.getKind()==Diagnostic.Kind.ERROR),"Unresolved action sources");
            for(var unit:units)for(var declaration:unit.getTypeDecls())if(declaration instanceof ClassTree type)
                for(var member:type.getMembers())if(member instanceof MethodTree m && m.getBody()!=null) {
                    String name=m.getName().toString();
                    if(type.getSimpleName().contentEquals("ActionUsageAdapter") && List.of("getRedefinedFeature","getRelevantFeatures","isComputeRedefinitions").contains(name)
                        || type.getSimpleName().contentEquals("AcceptActionUsageAdapter") && name.equals("isTriggerAction"))
                        methods.put(trees.getElement(TreePath.getPath(unit,type))+"#"+name,node(unit,m.getBody(),trees));
                    if(type.getSimpleName().contentEquals("AcceptActionUsageAdapter") && name.equals("addDefaultGeneralType")) {
                        var stmts=m.getBody().getStatements();require(stmts.size()==1 && stmts.get(0) instanceof IfTree,"Changed Accept default body");
                        var branch=(IfTree)stmts.get(0);require(branch.getElseStatement()==null && branch.getThenStatement() instanceof BlockTree,"Changed Accept default branch");
                        var effects=((BlockTree)branch.getThenStatement()).getStatements();require(effects.size()==1 && effects.get(0) instanceof ExpressionStatementTree,"Changed Accept default effect");
                        methods.put("accept_default",Map.of("condition",node(unit,branch.getCondition(),trees),"effect",node(unit,((ExpressionStatementTree)effects.get(0)).getExpression(),trees)));
                    }
                    if(type.getSimpleName().contentEquals("AcceptActionUsageAdapter") && name.equals("addComputedRedefinitions")) {
                        var stmts=m.getBody().getStatements();require(stmts.size()==2 && stmts.stream().allMatch(t->t instanceof ExpressionStatementTree),"Changed Accept redefinition lifecycle");
                        methods.put("accept_redefinitions",stmts.stream().map(t->node(unit,((ExpressionStatementTree)t).getExpression(),trees)).toList());
                    }
                }
        }
        require(methods.size()==6,"Missing resolved Action policy");
        var f=SysMLFactory.eINSTANCE;var controls=new ArrayList<Object>();var ordinary=new ArrayList<Object>();var bindings=new TreeMap<String,Object>();var defaults=new TreeMap<String,Object>();
        var selector=method(org.omg.sysml.adapter.ActionUsageAdapter.class,"getRedefinedFeature",Feature.class);selector.setAccessible(true);
        for(var classifier:SysMLPackage.eINSTANCE.getEClassifiers()) {
            if(!(classifier instanceof EClass cls)||cls.isAbstract()||!(f.create(cls) instanceof ActionUsage))continue;
            var sample=(ActionUsage)f.create(cls);var adapter=ElementUtil.getElementAdapter(sample);
            if(!key(method(adapter.getClass(),"getRelevantFeatures",Type.class)).equals("org.omg.sysml.adapter.ActionUsageAdapter#getRelevantFeatures"))continue;
            var dispatch=new TreeMap<String,String>();
            for(String name:List.of("getRelevantFeatures","getGeneralTypes","addRedefinitions","addComputedRedefinitions","addFeatureWriteTypes","isComputeRedefinitions","getRedefinedFeaturesWithComputed")) {
                java.lang.Class<?>[] parameters=name.equals("getRelevantFeatures")?new java.lang.Class<?>[]{Type.class}:name.equals("getGeneralTypes")?new java.lang.Class<?>[]{Type.class,Element.class}:List.of("addRedefinitions","addComputedRedefinitions").contains(name)?new java.lang.Class<?>[]{Element.class}:new java.lang.Class<?>[]{};
                dispatch.put(name,key(method(adapter.getClass(),name,parameters)));
            }
            bindings.put(cls.getName(),dispatch);
            if(dispatch.get("addComputedRedefinitions").equals("org.omg.sysml.adapter.FeatureAdapter#addComputedRedefinitions")) {
                for(String ownerKind:List.of("detached","Package","Class"))for(int explicit:List.of(0,2)) {
                    var target=(ActionUsage)f.create(cls);target.setIsEnd(false);
                    if(!ownerKind.equals("detached")) {var owner=(Namespace)f.create((EClass)SysMLPackage.eINSTANCE.getEClassifier(ownerKind));var member=f.createFeatureMembership();member.setOwnedMemberFeature(target);owner.getOwnedRelationship().add(member);}
                    var ids=new IdentityHashMap<Feature,String>();
                    for(int i=0;i<explicit;i++) {var general=f.createFeature();general.setDeclaredName("General"+i);ids.put(general,"general"+i);var relation=f.createRedefinition();relation.setRedefiningFeature(target);relation.setRedefinedFeature(general);target.getOwnedRelationship().add(relation);}
                    var row=new TreeMap<String,Object>();row.put("kind",cls.getName());row.put("owner_kind",ownerKind);row.put("explicit",explicit);
                    row.put("redefined_features",FeatureUtil.getRedefinedFeaturesWithComputedOf(target).stream().map(g->Objects.requireNonNull(ids.get(g),"Unexpected Action ordinary endpoint")).toList());
                    row.put("effective_name",target.effectiveName());row.put("effective_short_name",target.effectiveShortName());ordinary.add(row);
                }
            }

            var names=new TreeMap<String,Object>();
            for(var role:StateSubactionKind.values())names.put(role.getLiteral(),ImplicitGeneralizationMap.getDefaultSupertypeFor(sample.getClass(),role.getLiteral()));
            for(var role:TransitionFeatureKind.values())names.put(role.getLiteral(),ImplicitGeneralizationMap.getDefaultSupertypeFor(sample.getClass(),role.getLiteral()));
            defaults.put(cls.getName(),names);
            for(String context:List.of("detached","ordinary","parameter","state:entry","state:do","state:exit","transition:trigger","transition:guard","transition:effect")) {
                var target=(ActionUsage)f.create(cls);var row=new TreeMap<String,Object>();row.put("kind",cls.getName());row.put("context",context);
                if(!context.equals("detached")) {
                    Type owner=context.startsWith("state:")?f.createStateUsage():context.startsWith("transition:")?f.createTransitionUsage():f.createClass();owner.setIsImpliedIncluded(true);
                    FeatureMembership membership;
                    if(context.startsWith("state:")){var m=f.createStateSubactionMembership();m.setKind(StateSubactionKind.get(context.substring(6)));membership=m;}
                    else if(context.startsWith("transition:")){var m=f.createTransitionFeatureMembership();m.setKind(TransitionFeatureKind.get(context.substring(11)));membership=m;}
                    else membership=context.equals("parameter")?f.createParameterMembership():f.createFeatureMembership();
                    membership.setOwnedMemberFeature(target);owner.getOwnedRelationship().add(membership);
                }
                row.put("default_redefined_feature",selector.invoke(null,target));
                var current=ElementUtil.getElementAdapter(target);var relevant=method(current.getClass(),"getRelevantFeatures",Type.class);relevant.setAccessible(true);
                var selected=(List<?>)relevant.invoke(current,target.getOwningType());
                require(selected.stream().allMatch(value->value==target),"Unexpected own relevant feature");row.put("same_owner_relevant",selected.size());
                if(target instanceof AcceptActionUsage)row.put("is_trigger_action",((org.omg.sysml.adapter.AcceptActionUsageAdapter)ElementUtil.getElementAdapter(target)).isTriggerAction());
                controls.add(row);
            }
        }
        Files.writeString(Path.of(args[1]),new GsonBuilder().serializeNulls().setPrettyPrinting().create().toJson(Map.of("methods",methods,"bindings",bindings,"controls",controls,"defaults",defaults,"ordinary_controls",ordinary))+"\n");
    }
}
