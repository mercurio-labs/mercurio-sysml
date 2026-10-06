package dev.mercurio.pilot;

import com.google.gson.GsonBuilder;
import com.sun.source.tree.*;
import com.sun.source.util.*;
import javax.tools.*;
import java.nio.file.*;
import java.util.*;
import org.omg.sysml.lang.sysml.*;
import org.omg.sysml.util.*;
import static dev.mercurio.pilot.PilotFeatureRedefinitionExporter.*;

/** Exact Feature selector export; dependent algorithms remain separately qualified. */
public final class PilotFeatureDefaultExporter {
    static Map<String,Object> endControl(com.google.inject.Injector injector, String source) throws Exception {
        var result = new TreeMap<String,Object>(PilotDefinitionDocumentProbe.observe(injector,"kerml",source));
        var resources = injector.getInstance(org.eclipse.xtext.resource.XtextResourceSet.class);
        var resource = resources.createResource(org.eclipse.emf.common.util.URI.createURI("memory:/end-generals.kerml"));
        resource.load(new java.io.ByteArrayInputStream(source.getBytes(java.nio.charset.StandardCharsets.UTF_8)),Map.of());
        var queries = new ArrayList<Object>();
        var projections = new ArrayList<Object>();
        var all = resource.getAllContents();
        while (all.hasNext()) {
            var item = all.next();
            if (item instanceof Connector connector && "x".equals(connector.getDeclaredName())) {
                var sourceFeature = connector.getSourceFeature();
                projections.add(Map.of("source_feature",sourceFeature == null ? List.of() : List.of(sourceFeature.getDeclaredName()),
                    "target_feature",connector.getTargetFeature().stream().map(Feature::getDeclaredName).toList(),
                    "related_feature",connector.getRelatedFeature().stream().map(Feature::getDeclaredName).toList()));
            }
            if (item instanceof Feature feature && List.of("b","c").contains(feature.getDeclaredName())) {
                var names = TypeUtil.getGeneralTypesOf(feature).stream().map(Type::getDeclaredName).toList();
                queries.add(Map.of("feature",feature.getDeclaredName(),"generals",names));
            }
        }
        require(resource.getErrors().isEmpty(),"Unresolved end control: "+source+" "+resource.getErrors());
        result.put("general_queries",queries); result.put("connector_projections",projections); resource.unload(); return result;
    }

    static Map<String,Object> typeControl(com.google.inject.Injector injector, String source) throws Exception {
        var result=new TreeMap<String,Object>(PilotDefinitionDocumentProbe.observe(injector,"kerml",source));
        var resources=injector.getInstance(org.eclipse.xtext.resource.XtextResourceSet.class);
        var resource=resources.createResource(org.eclipse.emf.common.util.URI.createURI("memory:/feature-types.kerml"));
        resource.load(new java.io.ByteArrayInputStream(source.getBytes(java.nio.charset.StandardCharsets.UTF_8)),Map.of());
        var all=resource.getAllContents();
        while(all.hasNext()) {
            var item=all.next();
            if(item instanceof Association association && "x".equals(association.getDeclaredName())) {
                var first=association.getSourceType();
                result.put("association_types",Map.of("source_type",first==null ? List.of() : List.of(first.getDeclaredName()),
                    "related_type",association.getRelatedType().stream().map(Type::getDeclaredName).toList(),
                    "target_type",association.getTargetType().stream().map(Type::getDeclaredName).toList()));
            }
            if(item instanceof Feature feature && "x".equals(feature.getDeclaredName()))
                result.put("types",feature.getType().stream().map(Type::getDeclaredName).toList());
        }
        require(resource.getErrors().isEmpty() && (result.containsKey("types") || result.containsKey("association_types")),"Unresolved type control: "+source+" "+resource.getErrors());
        resource.unload(); return result;
    }

    // Complete resolved preorder for the bounded override. The generator guards
    // every node, while native ownership checks prove the special branches absent.
    static List<Object> resolvedReferenceTree(CompilationUnitTree unit, Tree body, Trees trees) {
        var rows=new ArrayList<Object>();
        new TreeScanner<Void,Integer>() {
            @Override public Void scan(Tree tree,Integer depth) {
                if(tree==null) return null;
                var row=new TreeMap<String,Object>();row.put("kind",tree.getKind().name());row.put("depth",depth);
                var path=TreePath.getPath(unit,tree);var type=trees.getTypeMirror(path);
                if(type!=null) row.put("type",type.toString());
                if(tree instanceof IdentifierTree identifier) row.put("name",identifier.getName().toString());
                if(tree instanceof LiteralTree literal) row.put("value",literal.getValue());
                if(tree instanceof MethodInvocationTree) {
                    var symbol=trees.getElement(path);
                    require(symbol instanceof javax.lang.model.element.ExecutableElement,"Unresolved ReferenceUsage effect");
                    var method=(javax.lang.model.element.ExecutableElement)symbol;
                    row.put("symbol",method.getEnclosingElement()+"#"+method.getSimpleName());
                    row.put("parameters",method.getParameters().stream().map(p->p.asType().toString()).toList());
                }
                rows.add(row);return super.scan(tree,depth+1);
            }
        }.scan(body,0);
        return rows;
    }

    public static void main(String[] args) throws Exception {
        org.omg.sysml.logic.SysMLLogicStandaloneSetup.doSetup();
        var methods = new TreeMap<String,Object>();
        var compiler = ToolProvider.getSystemJavaCompiler();
        var diagnostics = new DiagnosticCollector<JavaFileObject>();
        try (var files = compiler.getStandardFileManager(diagnostics,null,java.nio.charset.StandardCharsets.UTF_8)) {
            var task = (JavacTask)compiler.getTask(null,files,diagnostics,List.of("-proc:none","-classpath",args[0]),null,files.getJavaFileObjects(args[2],args[3],args[4],args[5],args[6],args[7]));
            var units = new ArrayList<CompilationUnitTree>(); task.parse().forEach(units::add); task.analyze();
            require(diagnostics.getDiagnostics().stream().noneMatch(d -> d.getKind() == Diagnostic.Kind.ERROR), "Unresolved source: " + diagnostics.getDiagnostics());
            var trees = Trees.instance(task);
            for (var unit: units) for (var declaration: unit.getTypeDecls()) if (declaration instanceof ClassTree type)
                for (var member: type.getMembers()) if (member instanceof MethodTree method) {
                    if ((type.getSimpleName().contentEquals("FeatureAdapter") && List.of("getAllTypes","getTypes","getFeatureTypes","removeRedundantTypes").contains(method.getName().toString())) || (type.getSimpleName().contentEquals("Feature_typingFeatures_InvocationDelegate") && method.getName().contentEquals("dynamicInvoke")))
                        methods.put("typing_"+method.getName(),resolvedReferenceTree(unit,method.getBody(),trees));
                    if (type.getSimpleName().contentEquals("Usage_definition_SettingDelegate") && method.getName().contentEquals("basicGet"))
                        methods.put("typing_usage_definition",resolvedReferenceTree(unit,method.getBody(),trees));
                    if (method.getName().contentEquals("getDefaultSupertype")) methods.put(type.getSimpleName().contentEquals("AssociationAdapter") ? "association_selector" : "selector",node(unit,method.getBody(),trees));
                    if (type.getSimpleName().contentEquals("UsageAdapter") && method.getName().contentEquals("addDefaultGeneralType")) {
                        var statements=method.getBody().getStatements();
                        require(statements.size()==2 && statements.stream().allMatch(t->t instanceof ExpressionStatementTree),"Changed Usage default contributions");
                        methods.put("usage_contributions",statements.stream().map(t->node(unit,((ExpressionStatementTree)t).getExpression(),trees)).toList());
                    }
                    if (type.getSimpleName().contentEquals("UsageAdapter") && method.getName().contentEquals("isAddMultiplicity"))
                        methods.put("usage_multiplicity",node(unit,method.getBody(),trees));
                    if (type.getSimpleName().contentEquals("UsageAdapter") && method.getName().contentEquals("addAdditionalMembers")) {
                        var statements=method.getBody().getStatements();
                        require(statements.size()==1 && statements.get(0) instanceof IfTree,"Changed Usage added members");
                        var branch=(IfTree)statements.get(0);
                        require(branch.getElseStatement()==null && branch.getThenStatement() instanceof BlockTree,"Changed Usage added-member branch");
                        var body=((BlockTree)branch.getThenStatement()).getStatements();
                        require(body.size()==1 && body.get(0) instanceof ExpressionStatementTree,"Changed Usage added-member effect");
                        methods.put("usage_added_members",Map.of("condition",node(unit,branch.getCondition(),trees),"effect",node(unit,((ExpressionStatementTree)body.get(0)).getExpression(),trees)));
                    }
                    if (type.getSimpleName().contentEquals("ReferenceUsageAdapter") && method.getName().contentEquals("addDefaultGeneralType"))
                        methods.put("reference_default_contributions",resolvedReferenceTree(unit,method.getBody(),trees));
                    if (type.getSimpleName().contentEquals("ReferenceUsageAdapter") && method.getName().contentEquals("addRedefinitions")) {
                        var body=method.getBody().getStatements();
                        require(body.size()==3 && body.get(0) instanceof VariableTree && body.get(1) instanceof VariableTree && body.get(2) instanceof IfTree,"Changed ReferenceUsage fallback");
                        var branch=(IfTree)body.get(2);
                        Tree condition=branch.getCondition();while(condition instanceof ParenthesizedTree paren) condition=paren.getExpression();
                        require(condition instanceof BinaryTree && condition.getKind()==Tree.Kind.CONDITIONAL_AND,"Changed ReferenceUsage guard");
                        Tree left=((BinaryTree)condition).getLeftOperand();while(left instanceof ParenthesizedTree paren) left=paren.getExpression();
                        require(left instanceof InstanceOfTree,"Changed ReferenceUsage owner test");
                        var test=(InstanceOfTree)left;
                        require(branch.getElseStatement() instanceof BlockTree,"Missing ReferenceUsage fallback block");
                        var fallback=((BlockTree)branch.getElseStatement()).getStatements();
                        require(fallback.size()==1 && fallback.get(0) instanceof ExpressionStatementTree,"Changed ReferenceUsage fallback effect");
                        methods.put("reference_redefinition_fallback",Map.of("locals",body.subList(0,2).stream().map(t->node(unit,t,trees)).toList(),
                            "link_identity",node(unit,((BinaryTree)condition).getRightOperand(),trees),
                            "guard_operand",node(unit,test.getExpression(),trees),"guard_type",trees.getTypeMirror(TreePath.getPath(unit,test.getType())).toString(),
                            "else_effect",node(unit,((ExpressionStatementTree)fallback.get(0)).getExpression(),trees)));
                    }
                    if (method.getName().contentEquals("addParticipantSubsetting")) {
                        var statements = method.getBody().getStatements();
                        require(statements.size() == 1 && statements.get(0) instanceof IfTree,"Changed participant body");
                        var branch = (IfTree)statements.get(0);
                        require(branch.getElseStatement() == null && branch.getThenStatement() instanceof BlockTree,"Changed participant branches");
                        var body = ((BlockTree)branch.getThenStatement()).getStatements();
                        require(body.size() == 1 && body.get(0) instanceof ExpressionStatementTree,"Changed participant effect");
                        methods.put("participant",Map.of("condition",node(unit,branch.getCondition(),trees),"effect",node(unit,((ExpressionStatementTree)body.get(0)).getExpression(),trees)));
                    }
                }
        }
        require(methods.size() == 14,"Missing selector");
        var factory = SysMLFactory.eINSTANCE;
        var sample = factory.createFeature();
        var adapter = ElementUtil.getElementAdapter(sample);
        var dispatch = new TreeMap<String,String>();
        for (String name: List.of("getDefaultSupertype","computeImplicitGeneralTypes","addDefaultGeneralType","getSpecializationEClass","hasStructureType"))
            dispatch.put(name,key(method(adapter.getClass(),name)));
        methods.put("dispatch",dispatch);
        var defaults = new TreeMap<String,String>();
        for (String name: List.of("base","object","subobject","occurrence","suboccurrence","portion","dataValue"))
            defaults.put(name,ImplicitGeneralizationMap.getDefaultSupertypeFor(sample.getClass(),name));
        var controls = new ArrayList<Object>();
        for (int mask=0;mask<8;mask++) for (String owner: List.of("detached","Package","Class","Structure","Feature")) for (int ownerMask=0;ownerMask<(owner.equals("Feature") ? 8 : 1);ownerMask++) for (boolean composite: List.of(false,true)) for (boolean portion: List.of(false,true)) {
            var connector = factory.createFeature(); connector.setIsImpliedIncluded(true);
            connector.setIsComposite(composite); connector.setIsPortion(portion);
            if (!owner.equals("detached")) {
                Namespace namespace = owner.equals("Package") ? factory.createPackage() : owner.equals("Class") ? factory.createClass() : owner.equals("Structure") ? factory.createStructure() : factory.createFeature();
                if (namespace instanceof Type type) type.setIsImpliedIncluded(true);
                if (namespace instanceof Feature feature) for (int bit=0;bit<3;bit++) if ((ownerMask & (1<<bit)) != 0) {
                    Type type = bit==0 ? factory.createClass() : bit==1 ? factory.createStructure() : factory.createDataType(); type.setIsImpliedIncluded(true);
                    var typing = factory.createFeatureTyping(); typing.setTypedFeature(feature); typing.setType(type); feature.getOwnedRelationship().add(typing);
                }
                var membership = owner.equals("Package") ? factory.createOwningMembership() : factory.createFeatureMembership();
                membership.getOwnedRelatedElement().add(connector); namespace.getOwnedRelationship().add(membership);
            }
            for (int bit=0;bit<3;bit++) if ((mask & (1<<bit)) != 0) {
                Type type = bit==0 ? factory.createClass() : bit==1 ? factory.createStructure() : factory.createDataType(); type.setIsImpliedIncluded(true);
                var relationship = factory.createFeatureTyping(); relationship.setTypedFeature(connector); relationship.setType(type); connector.getOwnedRelationship().add(relationship);
            }
            var actualAdapter = ElementUtil.getElementAdapter(connector);
            var selector = method(actualAdapter.getClass(),"getDefaultSupertype"); selector.setAccessible(true);
            var row = new TreeMap<String,Object>(); row.put("typing_mask",mask); row.put("owner",owner); row.put("owner_typing_mask",ownerMask); row.put("composite",composite); row.put("portion",portion);
            row.put("default_supertype",selector.invoke(actualAdapter)); controls.add(row);
        }
        var associationDefaults = new TreeMap<String,Object>();
        var associationDispatch = new TreeMap<String,Object>();
        var associationControls = new ArrayList<Object>();
        for (String kind: List.of("Association","AssociationStructure")) {
            var names = new TreeMap<String,String>();
            var sampleAssociation = kind.equals("Association") ? factory.createAssociation() : factory.createAssociationStructure();
            var inherited = new TreeMap<String,String>();
            for (String name: List.of("getDefaultSupertype","computeImplicitGeneralTypes","addDefaultGeneralType","getSpecializationEClass"))
                inherited.put(name,key(method(ElementUtil.getElementAdapter(sampleAssociation).getClass(),name)));
            associationDispatch.put(kind,inherited);
            for (String key: List.of("base","binary")) names.put(key,ImplicitGeneralizationMap.getDefaultSupertypeFor(sampleAssociation.getClass(),key));
            associationDefaults.put(kind,names);
            for (int count=0;count<4;count++) for (boolean owned: List.of(false,true)) {
                var association = kind.equals("Association") ? factory.createAssociation() : factory.createAssociationStructure();
                association.setIsImpliedIncluded(true);
                if (owned) {
                    var namespace=factory.createPackage();var member=factory.createOwningMembership();
                    member.getOwnedRelatedElement().add(association); namespace.getOwnedRelationship().add(member);
                }
                for (int i=0;i<count;i++) {
                    var end=factory.createFeature();end.setIsEnd(true);end.setIsImpliedIncluded(true);
                    var member=factory.createEndFeatureMembership();member.setOwnedMemberFeature(end);association.getOwnedRelationship().add(member);
                }
                var actual=ElementUtil.getElementAdapter(association);var selector=method(actual.getClass(),"getDefaultSupertype");selector.setAccessible(true);
                associationControls.add(Map.of("kind",kind,"owned_ends",count,"package_owned",owned,"default_supertype",selector.invoke(actual)));
            }
        }
        var usageBindings=new TreeMap<String,Object>();
        var usageControls=new ArrayList<Object>();
        for(String kind:List.of("Usage","BindingConnectorAsUsage","ReferenceUsage")) {
            var cls=(org.eclipse.emf.ecore.EClass)SysMLPackage.eINSTANCE.getEClassifier(kind);
            var target=(Feature)factory.create(cls);
            var targetAdapter=ElementUtil.getElementAdapter(target);
            var inherited=new TreeMap<String,String>();
            for(String name:List.of("getDefaultSupertype","computeImplicitGeneralTypes","addDefaultGeneralType","getSpecializationEClass","hasStructureType","addAdditionalMembers","isAddMultiplicity"))
                inherited.put(name,key(method(targetAdapter.getClass(),name)));
            var names=new TreeMap<String,String>();
            for(String name:defaults.keySet()) names.put(name,ImplicitGeneralizationMap.getDefaultSupertypeFor(target.getClass(),name));
            usageBindings.put(kind,Map.of("dispatch",inherited,"defaults",names));
            for(int mask=0;mask<8;mask++) for(boolean owned:List.of(false,true)) for(boolean composite:List.of(false,true)) for(boolean portion:List.of(false,true)) {
                var feature=(Feature)factory.create(cls);feature.setIsImpliedIncluded(true);feature.setIsComposite(composite);feature.setIsPortion(portion);
                if(owned) { var namespace=factory.createPackage();var membership=factory.createOwningMembership();membership.getOwnedRelatedElement().add(feature);namespace.getOwnedRelationship().add(membership); }
                for(int bit=0;bit<3;bit++) if((mask & (1<<bit))!=0) {
                    Type type=bit==0 ? factory.createClass() : bit==1 ? factory.createStructure() : factory.createDataType();type.setIsImpliedIncluded(true);
                    var typing=factory.createFeatureTyping();typing.setTypedFeature(feature);typing.setType(type);feature.getOwnedRelationship().add(typing);
                }
                var actual=ElementUtil.getElementAdapter(feature);var selector=method(actual.getClass(),"getDefaultSupertype");selector.setAccessible(true);
                usageControls.add(Map.of("kind",kind,"typing_mask",mask,"package_owned",owned,"composite",composite,"portion",portion,"default_supertype",selector.invoke(actual)));
            }
        }
        var result = new TreeMap<String,Object>(); result.put("schema","dev.mercurio.feature-defaults.v1"); result.put("binding","Feature"); result.put("methods",methods); result.put("defaults",defaults); result.put("controls",controls);
        var referenceDispatch=new TreeMap<String,String>();
        var referenceAdapter=ElementUtil.getElementAdapter(factory.createReferenceUsage());
        for(String name:List.of("addRedefinitions","addFeatureWriteTypes","addComputedRedefinitions","isComputeRedefinitions","getRedefinedFeaturesWithComputed","getRelevantFeatures","getGeneralTypes","getEndRelevantFeatures","getParameterRelevantFeatures","getRelevantParameters","filterIgnoredParameters")) {
            java.lang.Class<?>[] parameters=name.equals("addRedefinitions") || name.equals("addComputedRedefinitions") ? new java.lang.Class<?>[]{Element.class} : name.equals("getRelevantFeatures") || name.equals("getEndRelevantFeatures") || name.equals("getParameterRelevantFeatures") || name.equals("getRelevantParameters") ? new java.lang.Class<?>[]{Type.class} : name.equals("filterIgnoredParameters") ? new java.lang.Class<?>[]{List.class} : name.equals("getGeneralTypes") ? new java.lang.Class<?>[]{Type.class,Element.class} : new java.lang.Class<?>[]{};
            referenceDispatch.put(name,key(method(referenceAdapter.getClass(),name,parameters)));
        }
        result.put("reference_dispatch",referenceDispatch);
        var transitionControls=new ArrayList<Object>();
        for(String membershipKind:List.of("FeatureMembership","ParameterMembership","TransitionFeatureMembership"))
            for(boolean preceding:List.of(false,true)) {
                var transition=factory.createTransitionUsage();transition.setIsImpliedIncluded(true);
                if(preceding) {
                    var first=factory.createFeatureMembership();first.setOwnedMemberFeature(factory.createReferenceUsage());transition.getOwnedRelationship().add(first);
                }
                var membership=(FeatureMembership)factory.create((org.eclipse.emf.ecore.EClass)SysMLPackage.eINSTANCE.getEClassifier(membershipKind));
                Feature reference=membershipKind.equals("TransitionFeatureMembership") ? factory.createStep() : factory.createReferenceUsage();membership.setOwnedMemberFeature(reference);transition.getOwnedRelationship().add(membership);
                transitionControls.add(Map.of("membership",membershipKind,"preceding",preceding,"is_transition_link",reference==UsageUtil.getTransitionLinkFeatureOf(transition)));
            }
        result.put("reference_transition_controls",transitionControls);

        var referenceEnds=new ArrayList<Object>();
        for(String shape:List.of("direct","short","multiple","inherited","diamond","private","protected","explicit","duplicate","chain_equal","chain_distinct"))
            for(int position=0;position<3;position++) referenceEnds.add(PilotFeatureRedefinitionExporter.endControl(SysMLPackage.Literals.REFERENCE_USAGE,shape,position));
        result.put("reference_end_controls",referenceEnds);
        var referenceParameters=new ArrayList<Object>();
        for(String shape:List.of("direct","short","multiple","inherited","diamond","private","protected","explicit","result_first","no_result","chain_equal","chain_distinct"))
            for(int position=0;position<4;position++) referenceParameters.add(PilotFeatureRedefinitionExporter.parameterControl(SysMLPackage.eINSTANCE.getReferenceUsage(),shape,position));
        result.put("reference_parameter_controls",referenceParameters);

        var typingBindings=new TreeMap<String,Object>();var typingControls=new ArrayList<Object>();
        for(String kind:List.of("Feature","Usage","ReferenceUsage")) {
            var cls=(org.eclipse.emf.ecore.EClass)SysMLPackage.eINSTANCE.getEClassifier(kind);
            var actual=ElementUtil.getElementAdapter((Element)factory.create(cls));var resolved=new TreeMap<String,String>();
            resolved.put("getAllTypes",key(method(actual.getClass(),"getAllTypes")));
            for(String name:List.of("getTypes","getFeatureTypes")) resolved.put(name,key(method(actual.getClass(),name,List.class,Set.class)));
            typingBindings.put(kind,resolved);
            for(int mask=0;mask<16;mask++) for(String relationship:List.of("Subsetting","Redefinition","ReferenceSubsetting")) for(String shape:List.of("direct","chain","diamond","cycle")) {
                var candidate=(Feature)factory.create(cls);candidate.setDeclaredName("candidate");candidate.setIsImpliedIncluded(true);
                var leaf=factory.createFeature();leaf.setDeclaredName("leaf");leaf.setIsImpliedIncluded(true);
                var left=factory.createFeature();left.setDeclaredName("left");left.setIsImpliedIncluded(true);
                var right=factory.createFeature();right.setDeclaredName("right");right.setIsImpliedIncluded(true);
                var features=Map.of("candidate",candidate,"leaf",leaf,"left",left,"right",right);
                List<List<String>> edges=switch(shape) {
                    case "direct" -> List.of(List.of("candidate","leaf"));
                    case "chain" -> List.of(List.of("candidate","left"),List.of("left","leaf"));
                    case "diamond" -> List.of(List.of("candidate","left"),List.of("candidate","right"),List.of("left","leaf"),List.of("right","leaf"));
                    default -> List.of(List.of("candidate","leaf"),List.of("leaf","candidate"));
                };
                for(var edge:edges) {
                    var relation=(Subsetting)factory.create((org.eclipse.emf.ecore.EClass)SysMLPackage.eINSTANCE.getEClassifier(relationship));
                    relation.setSubsettedFeature(features.get(edge.get(1)));relation.setSubsettingFeature(features.get(edge.get(0)));
                    features.get(edge.get(0)).getOwnedRelationship().add(relation);
                }
                for(int bit=0;bit<4;bit++) if((mask & (1<<bit))!=0) {
                    Type type=bit==0?factory.createClass():bit==1?factory.createStructure():bit==2?factory.createDataType():factory.createType();type.setDeclaredName("type"+bit);type.setIsImpliedIncluded(true);
                    var typing=factory.createFeatureTyping();typing.setType(type);typing.setTypedFeature(leaf);leaf.getOwnedRelationship().add(typing);
                }
                var typingAdapter=ElementUtil.getElementAdapter(candidate);var selector=method(typingAdapter.getClass(),"getDefaultSupertype");selector.setAccessible(true);
                typingControls.add(Map.of("kind",kind,"typing_mask",mask,"relationship",relationship,"shape",shape,"edges",edges,
                    "types",FeatureUtil.getAllTypesOf(candidate).stream().map(Type::getDeclaredName).toList(),"projected_types",candidate.getType().stream().map(Type::getDeclaredName).toList(),"default_supertype",selector.invoke(typingAdapter)));
            }
        }
        result.put("typing_bindings",typingBindings);result.put("inherited_typing_controls",typingControls);
        result.put("usage_bindings",usageBindings);result.put("usage_controls",usageControls);
        result.put("association_dispatch",associationDispatch); result.put("association_defaults",associationDefaults); result.put("association_controls",associationControls);
        var injector = new org.omg.kerml.xtext.KerMLStandaloneSetup().createInjectorAndDoEMFRegistration();
        var sourceControls = new ArrayList<Object>();
        for (String source: List.of(
            "standard library package Base { feature things; } standard library package Links { feature links subsets Base::things { feature marker; } } package P { connector x; feature y subsets x::marker; feature outer { feature middle { feature leaf; } } }",
            "standard library package Base { feature things; } standard library package Occurrences { class Occurrence; feature occurrences; } package P { class C; feature outer : C { feature inner : C; } }"
        )) sourceControls.add(PilotDefinitionDocumentProbe.observe(injector,"kerml",source));
        result.put("source_controls",sourceControls);
        var endControls = new ArrayList<Object>();
        String library = "standard library package Base { feature things; } standard library package Occurrences { class Occurrence; } standard library package Objects { struct Object; } ";
        for (String body: List.of(
            "class A { end feature a; } class B specializes A { end feature b; } class C specializes B { end feature c; }",
            "struct A { end feature a; } struct B specializes A { end feature b redefines a; } struct C specializes B { end feature c; }"
        )) endControls.add(endControl(injector,library + "package P { " + body + " }"));
        result.put("end_general_controls",endControls);
        var participantControls = new ArrayList<Object>();
        String participantLibrary = "standard library package Base { feature things; } standard library package Links { assoc Link { feature participant; } feature links; feature binaryLinks; } ";
        for (String body: List.of(
            "feature target; connector x { end feature b references target; }",
            "feature target; feature a; connector x { end feature b references target redefines P::a; }",
            "feature target; connector base { end feature a references target; } connector x subsets base { end feature b references target; }",
            "feature target; connector base { end feature a references target; } connector x subsets base { end feature b references target redefines base::a; }",
            "feature target; connector base { end feature a references target; } connector x subsets base { end feature b references target; end feature c references target; }"
        )) participantControls.add(endControl(injector,participantLibrary + "package P { " + body + " }"));
        result.put("connector_participant_controls",participantControls);
        var bindingControls=new ArrayList<Object>();
        String bindingLibrary="standard library package Base { feature things; } standard library package Links { assoc Link { feature participant; } feature selfLinks; } ";
        for(String body:List.of(
            "feature target; binding x { end feature b references target; end feature c references target; }",
            "feature target; feature other; binding x { end feature b references target; end feature c references other; }",
            "feature target; binding x { end feature b; end feature c references target; }",
            "feature target; feature other; binding x { end feature b references target; end feature c references other redefines P::target; }"
        ))bindingControls.add(endControl(injector,bindingLibrary+"package P { "+body+" }"));
        result.put("binding_connector_controls",bindingControls);
        var absentReferenceControls = new ArrayList<Object>();
        for (String body: List.of(
            "connector x { end feature b; }",
            "feature target; connector x { end feature b; end feature c references target; }",
            "feature target; connector base { end feature a references target; } connector x subsets base { end feature b; }",
            "feature target; connector base { end feature a references target; } connector x subsets base;",
            "feature target; connector x { end feature b references target; end feature c; }"
        )) absentReferenceControls.add(endControl(injector,participantLibrary + "package P { " + body + " }"));
        result.put("absent_reference_controls",absentReferenceControls);
        var associationSources=new ArrayList<Object>();
        String associationLibrary="standard library package Base { feature things; } standard library package Links { assoc Link { feature participant; } assoc BinaryLink specializes Link; } standard library package Objects { assoc struct LinkObject; assoc struct BinaryLinkObject specializes LinkObject; } ";
        for (String declaration: List.of("assoc","assoc struct")) for (String body: List.of(";", " { end feature b; end feature c; }"))
            associationSources.add(endControl(injector,associationLibrary+"package P { "+declaration+" A"+body+" }"));
        result.put("association_source_controls",associationSources);
        var typeControls=new ArrayList<Object>();
        String typeLibrary="standard library package Base { feature things; } standard library package Occurrences { class Occurrence; feature occurrences; } ";
        for(String body:List.of(
            "class A; class B; feature x : A, B;",
            "class A; class B specializes A; feature x : A, B;",
            "class A; class B; feature a : A; feature b : B; feature x subsets a, b;",
            "class A; class B; feature a : A subsets b; feature b : B subsets a; feature x subsets a;",
            "class A; feature a : A; feature x redefines P::a;",
            "class A; class B; feature b : B; feature x : A references b;",
            "class A specializes B; class B specializes A; feature x : A, B;",
            "feature x;"
        )) typeControls.add(typeControl(injector,typeLibrary+"package P { "+body+" }"));
        result.put("feature_type_controls",typeControls);
        var associationTypeControls=new ArrayList<Object>();
        String associationTypeLibrary=associationLibrary+"standard library package Occurrences { class Occurrence; feature occurrences; } ";
        for(String body:List.of(
            "assoc x;",
            "class A; assoc x { end feature b : A; }",
            "class A; class B; assoc x { end feature b : A; end feature c : B; }",
            "class A; assoc x { end feature b : A; end feature c : A; }",
            "class A; class B; assoc x { end feature b : A, B; end feature c : B; }",
            "class A; class B; assoc base { end feature b : A; } assoc x specializes base { end feature c : B; }",
            "class A; assoc base { end feature b : A; } assoc x specializes base;",
            "class A; class B; assoc struct x { end feature b : A; end feature c : B; }"
        )) associationTypeControls.add(typeControl(injector,associationTypeLibrary+"package P { "+body+" }"));
        result.put("association_type_controls",associationTypeControls);
        var sysmlInjector=new org.omg.sysml.xtext.SysMLStandaloneSetup().createInjectorAndDoEMFRegistration();
        var usageSources=new ArrayList<Object>();
        String usageLibrary="standard library package Links { part a; part b; binding selfLinks bind a = b; } ";
        for(String declaration:List.of("binding x bind a = b;","binding x bind b = a;","bind a = a;","binding x subsets Links::selfLinks bind a = b;"))
            usageSources.add(PilotDefinitionDocumentProbe.observe(sysmlInjector,"sysml",usageLibrary+"package P { part a; part b; "+declaration+" }"));
        result.put("usage_source_controls",usageSources);
        var annotationDefaults = new ArrayList<Object>();
        for (String annotation : List.of("", "doc /* documentation */", "comment C /* comment */", "rep R language \"text\" /* representation */"))
            for (String typing : List.of("", " : D")) for (String bounds : List.of("", " [1]", " [0..*]", " [1..2]"))
                for (String owner : List.of("", "classifier C", "datatype C")) {
                String declaration = "feature x" + typing + bounds + " { " + annotation + " }";
                String source = "standard library package Base { classifier Anything; datatype DataValue specializes Anything; } package P { datatype D; " +
                    (owner.isEmpty() ? declaration : owner + " { " + declaration + " }") + " }";
                var resources = injector.getInstance(org.eclipse.xtext.resource.XtextResourceSet.class);
                var resource = resources.createResource(org.eclipse.emf.common.util.URI.createURI("memory:/annotation-default.kerml"));
                resource.load(new java.io.ByteArrayInputStream(source.getBytes(java.nio.charset.StandardCharsets.UTF_8)), Map.of());
                var all = resource.getAllContents();
                while (all.hasNext()) {
                    var item = all.next();
                    if (item instanceof Feature feature && "x".equals(feature.getDeclaredName())) {
                        var annotationAdapter = ElementUtil.getElementAdapter(feature);
                        var selector = method(annotationAdapter.getClass(), "getDefaultSupertype"); selector.setAccessible(true);
                        annotationDefaults.add(Map.of("source", source, "default_supertype", selector.invoke(annotationAdapter)));
                    }
                }
                require(resource.getErrors().isEmpty(), "Invalid annotation default control: " + source + " " + resource.getErrors());
                resource.unload();
            }
        result.put("annotation_default_controls", annotationDefaults);
        Files.writeString(Path.of(args[1]),new GsonBuilder().serializeNulls().setPrettyPrinting().create().toJson(result)+"\n");
    }
}
