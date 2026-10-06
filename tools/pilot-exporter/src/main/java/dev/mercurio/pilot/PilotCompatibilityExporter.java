package dev.mercurio.pilot;
import com.google.gson.GsonBuilder;
import com.sun.source.tree.*;
import com.sun.source.util.*;
import javax.tools.*;
import java.nio.file.*;
import java.util.*;
import org.omg.sysml.lang.sysml.*;
import org.omg.sysml.util.*;
public final class PilotCompatibilityExporter {
    static Map<String,Object> bindingSnapshot(Feature feature, BindingConnector root) { return bindingSnapshot(feature,root,null); }
    static Map<String,Object> bindingSnapshot(Feature feature, BindingConnector root, Feature result) {
        var generals=new ArrayList<Object>();
        for(var relation:feature.getOwnedSpecialization())generals.add(Map.of("kind",relation.eClass().getName(),"target",relation.getGeneral()==result?"$result":relation.getGeneral().getDeclaredName(),"implied",relation.isImplied()));
        var featuring=feature.getOwnedTypeFeaturing().stream().map(r->r.getFeaturingType()==root?"$binding":r.getFeaturingType().getDeclaredName()).toList();
        return Map.of("kind",feature.eClass().getName(),"complete",feature.isImpliedIncluded(),"implied",feature instanceof Relationship relationship && relationship.isImplied(),"end",feature.isEnd(),"generals",generals,"featuring",featuring,"relationships",feature.getOwnedRelationship().stream().map(r->r.eClass().getName()).toList());
    }
    static String fieldName(String name) {
        return name.replaceAll("([a-z])([A-Z])", "$1_$2").toLowerCase(Locale.ROOT);
    }
    static Object scalar(Object value) {
        if(value instanceof org.eclipse.emf.common.util.Enumerator e)return e.getLiteral();
        if(value instanceof List<?> values)return values.stream().map(PilotCompatibilityExporter::scalar).toList();
        return value;
    }
    static List<?> values(org.eclipse.emf.ecore.EObject object, org.eclipse.emf.ecore.EReference field) {
        Object value=object.eGet(field,true);
        return field.isMany()?(List<?>)value:value==null?List.of():List.of(value);
    }
    static void indexSubtree(org.eclipse.emf.ecore.EObject object,String path,IdentityHashMap<org.eclipse.emf.ecore.EObject,String> paths,List<org.eclipse.emf.ecore.EObject> objects) {
        if(paths.put(object,path)!=null)throw new IllegalStateException("Repeated containment");
        objects.add(object);
        for(var field:object.eClass().getEAllReferences())if(field.isContainment()&&!field.isDerived()&&!field.isTransient()&&!field.isVolatile()) {
            int i=0;for(var child:values(object,field))indexSubtree((org.eclipse.emf.ecore.EObject)child,path+"/"+fieldName(field.getName())+"/"+i++,paths,objects);
        }
    }
    static List<Object> storedSubtree(Element root) {
        var paths=new IdentityHashMap<org.eclipse.emf.ecore.EObject,String>();var objects=new ArrayList<org.eclipse.emf.ecore.EObject>();
        indexSubtree(root,"$",paths,objects);var rows=new ArrayList<Object>();
        for(var object:objects) {
            var attributes=new TreeMap<String,Object>();var references=new TreeMap<String,Object>();
            for(var field:object.eClass().getEAllAttributes())if(!field.isDerived()&&!field.isTransient()&&!field.isVolatile()&&!field.getName().equals("elementId")) {
                Object value=object.eGet(field,true);if(value!=null)attributes.put(fieldName(field.getName()),scalar(value));
            }
            for(var field:object.eClass().getEAllReferences())if(!field.isDerived()&&!field.isTransient()&&!field.isVolatile()) {
                var targets=new ArrayList<String>();
                for(var raw:values(object,field)) {
                    var target=(Element)raw;
                    if(target.eIsProxy())throw new IllegalStateException("Unresolved subtree reference");
                    String name=paths.get(target);
                    if(name==null){name=target.getQualifiedName();if(name==null)throw new IllegalStateException("Unnamed external subtree reference");name="external:"+name;}
                    targets.add(name);
                }
                references.put(fieldName(field.getName()),targets);
            }
            rows.add(Map.of("path",paths.get(object),"kind",object.eClass().getName(),"attributes",attributes,"references",references));
        }
        return rows;
    }
    static final class InclusionValidator extends org.omg.kerml.xtext.validation.KerMLValidator {
        final List<String> codes=new ArrayList<>();
        @Override protected void error(String message,org.eclipse.emf.ecore.EObject object,
                org.eclipse.emf.ecore.EStructuralFeature field,String code,String... data) { codes.add(code); }
    }
    static List<Object> inclusionCases() {
        var rows=new ArrayList<Object>();var validator=new InclusionValidator();
        for(var classifier:SysMLPackage.eINSTANCE.getEClassifiers()) {
            if(!(classifier instanceof org.eclipse.emf.ecore.EClass kind)||kind.isAbstract()||kind.isInterface()
                ||!SysMLPackage.eINSTANCE.getElement().isSuperTypeOf(kind))continue;
            for(boolean complete:List.of(false,true))for(String shape:List.of("empty","explicit","implied","mixed","nested")) {
                var owner=(Element)SysMLFactory.eINSTANCE.create(kind);owner.setIsImpliedIncluded(complete);
                if(!shape.equals("empty")) {
                    var relation=SysMLFactory.eINSTANCE.createMembership();relation.setIsImplied(shape.equals("implied")||shape.equals("mixed"));
                    owner.getOwnedRelationship().add(relation);
                    if(shape.equals("mixed"))owner.getOwnedRelationship().add(SysMLFactory.eINSTANCE.createMembership());
                    if(shape.equals("nested")){var nested=SysMLFactory.eINSTANCE.createMembership();nested.setIsImplied(true);relation.getOwnedRelationship().add(nested);}
                }
                validator.codes.clear();validator.checkElement(owner);
                rows.add(Map.of("kind",kind.getName(),"complete",complete,"shape",shape,"issues",List.copyOf(validator.codes)));
            }
        }
        return rows;
    }
    static List<Object> valueCases() throws Exception {
        var method=org.omg.sysml.adapter.FeatureAdapter.class.getDeclaredMethod("addBoundValueSubsetting");method.setAccessible(true);
        var rows=new ArrayList<Object>();
        for(String shape:List.of("none","ordinary","default","initial","missing_value","missing_result","first_default","first_empty","explicit_general","implied_general","directional")) {
            SysMLLibraryUtil.setProviderLookup(r -> (context,name)->null);
            var owner=SysMLFactory.eINSTANCE.createFeature();owner.setElementId("owner");
            var expression=SysMLFactory.eINSTANCE.createLiteralInteger();expression.setElementId("expression");expression.setIsImpliedIncluded(true);
            var result=SysMLFactory.eINSTANCE.createFeature();result.setElementId("result");result.setDirection(FeatureDirectionKind.OUT);result.setIsImpliedIncluded(true);
            if(!shape.equals("missing_result")) {var m=SysMLFactory.eINSTANCE.createReturnParameterMembership();m.setOwnedMemberParameter(result);expression.getOwnedRelationship().add(m);}
            if(!shape.equals("none")) {
                var value=SysMLFactory.eINSTANCE.createFeatureValue();value.setIsDefault(shape.equals("default")||shape.equals("first_default"));value.setIsInitial(shape.equals("initial"));
                if(!shape.equals("missing_value")&&!shape.equals("first_empty"))value.setValue(expression);
                owner.getOwnedRelationship().add(value);
                if(shape.startsWith("first_")){var later=SysMLFactory.eINSTANCE.createFeatureValue();later.setValue(SysMLFactory.eINSTANCE.createLiteralInteger());owner.getOwnedRelationship().add(later);}
            }
            if(shape.equals("explicit_general")||shape.equals("implied_general")){var r=SysMLFactory.eINSTANCE.createSubsetting();r.setIsImplied(shape.equals("implied_general"));r.setSubsettingFeature(owner);r.setSubsettedFeature(SysMLFactory.eINSTANCE.createFeature());owner.getOwnedRelationship().add(r);}
            if(shape.equals("directional"))owner.setDirection(FeatureDirectionKind.IN);
            var adapter=(org.omg.sysml.adapter.FeatureAdapter)ElementUtil.getElementAdapter(owner);
            method.invoke(adapter);
            var contributions=new ArrayList<Object>();
            adapter.forEachImplicitGeneralType((kind,general)->{
                var chain=((Feature)general).getChainingFeature().stream().map(Element::getElementId).toList();
                contributions.add(Map.of("kind",kind.getName(),"chain",chain));
            });
            rows.add(Map.of("shape",shape,"contributions",contributions));
        }
        return rows;
    }
    static List<Object> expressionFeaturingCases() throws Exception {
        var rows=new ArrayList<Object>();
        for(var classifier:SysMLPackage.eINSTANCE.getEClassifiers()) {
            if(!(classifier instanceof org.eclipse.emf.ecore.EClass kind)||kind.isAbstract()||kind.isInterface()
                ||!SysMLPackage.eINSTANCE.getExpression().isSuperTypeOf(kind))continue;
            for(String shape:List.of("empty","one","two","duplicate","existing")) {
                SysMLLibraryUtil.setProviderLookup(r -> (context,name)->null);
                var owner=SysMLFactory.eINSTANCE.createFeature();ElementUtil.transform(owner);owner.setIsImpliedIncluded(true);
                var expression=(Expression)SysMLFactory.eINSTANCE.create(kind);
                var value=SysMLFactory.eINSTANCE.createFeatureValue();value.setValue(expression);owner.getOwnedRelationship().add(value);
                var a=SysMLFactory.eINSTANCE.createClass();a.setElementId("a");
                var b=SysMLFactory.eINSTANCE.createClass();b.setElementId("b");
                if(!shape.equals("empty"))FeatureUtil.addTypeFeaturingTo(owner).setFeaturingType(a);
                if(shape.equals("two")||shape.equals("existing"))FeatureUtil.addTypeFeaturingTo(owner).setFeaturingType(b);
                if(shape.equals("duplicate"))FeatureUtil.addTypeFeaturingTo(owner).setFeaturingType(a);
                if(shape.equals("existing"))FeatureUtil.addTypeFeaturingTo(expression).setFeaturingType(a);
                var adapter=(org.omg.sysml.adapter.FeatureAdapter)ElementUtil.getElementAdapter(expression);
                java.lang.Class<?> implementation=adapter.getClass();java.lang.reflect.Method method=null;
                while(method==null){try{method=implementation.getDeclaredMethod("addImplicitFeaturingTypesIfNecessary");}catch(NoSuchMethodException e){implementation=implementation.getSuperclass();}}
                method.setAccessible(true);method.invoke(adapter);
                var pending=new ArrayList<String>();adapter.forEachImplicitFeaturingType(t->pending.add(t.getElementId()));
                FeatureUtil.insertImplicitTypeFeaturings(expression);
                var stored=new ArrayList<Object>();for(var r:expression.getOwnedTypeFeaturing())stored.add(Map.of("target",r.getFeaturingType().getElementId(),"implied",r.isImplied(),"adopted",r.getOwnedRelatedElement().contains(r.getFeaturingType())));
                rows.add(Map.of("kind",kind.getName(),"shape",shape,"implementation",implementation.getName(),"pending",pending,"stored",stored,"query",expression.getFeaturingType().stream().map(Element::getElementId).toList()));
            }
        }
        return rows;
    }
    static Map<String,Object> expressionLibraryResult(boolean inherited) throws Exception {
        String source="standard library package Base { feature things; } standard library package Links { assoc Link { feature participant; } feature selfLinks; } standard library package Performances { abstract expr evaluations { return libraryResult; } } package P { class A { feature y; feature x = y; } }";
        if(inherited)source=source.replace("feature selfLinks;","feature selfLinks { end feature left; end feature right; }");
        var injector=new org.omg.kerml.xtext.KerMLStandaloneSetup().createInjectorAndDoEMFRegistration();
        var resources=injector.getInstance(org.eclipse.xtext.resource.XtextResourceSet.class);
        var resource=resources.createResource(org.eclipse.emf.common.util.URI.createURI("memory:/expression-result.kerml"));
        resource.load(new java.io.ByteArrayInputStream(source.getBytes(java.nio.charset.StandardCharsets.UTF_8)),Map.of());
        var library=new HashMap<String,Element>();FeatureReferenceExpression expression=null;
        var all=resource.getAllContents();while(all.hasNext())if(all.next() instanceof Element e){if(e.getQualifiedName()!=null)library.put(e.getQualifiedName(),e);if(e instanceof FeatureReferenceExpression f)expression=f;}
        SysMLLibraryUtil.setProviderLookup(r -> (context,name)->library.get(name));
        if(expression==null)throw new IllegalStateException("Missing reference expression");
        TypeUtil.addResultParameterTo(expression);
        var result=TypeUtil.getOwnedResultParameterOf(expression);
        var targets=FeatureUtil.getRedefinedFeaturesWithComputedOf(result).stream().map(Feature::getDeclaredName).toList();
        if(!resource.getErrors().isEmpty())throw new IllegalStateException(resource.getErrors().toString());
        var adapter=(org.omg.sysml.adapter.FeatureAdapter)ElementUtil.getElementAdapter(result);
        adapter.forceComputeRedefinitions();adapter.addDefaultGeneralType();adapter.removeUnnecessaryImplicitGeneralTypes();
        var generals=new ArrayList<Object>();adapter.forEachImplicitGeneralType((kind,general)->generals.add(Map.of("kind",kind.getName(),"target",general.getDeclaredName())));
        ElementUtil.transformAll((Element)resource.getContents().get(0),true);
        var placements=new ArrayList<Object>();var bindingDefaults=new ArrayList<Object>();
        for(var item:expression.getOwnedMember())if(item instanceof BindingConnector binding) {
            bindingDefaults.add(Map.of("connector",bindingSnapshot(binding,binding,result),"ends",binding.getOwnedFeature().stream().map(e->bindingSnapshot(e,binding,result)).toList(),"end_redefined",binding.getOwnedFeature().stream().map(e->FeatureUtil.getRedefinedFeaturesWithComputedOf(e).stream().map(Feature::getDeclaredName).toList()).toList()));
            placements.add(Map.of("membership",binding.getOwningRelationship().eClass().getName(),"featuring",binding.getFeaturingType().stream().map(Element::getDeclaredName).toList(),"ends",binding.getOwnedFeature().size(),"related",binding.getRelatedFeature().stream().map(f->f==result?"$result":f.getDeclaredName()).toList()));
        }
        return Map.of("source",source,"targets",targets,"result_generals",generals,"reference_binding",placements,"binding_defaults",bindingDefaults);
    }
    static List<Object> resultDefaults() throws Exception {
        var rows=new ArrayList<Object>();
        for(String ownerKind:List.of("Expression","FeatureReferenceExpression","Function"))for(int mask=0;mask<8;mask++) {
            var owner=(Type)SysMLFactory.eINSTANCE.create((org.eclipse.emf.ecore.EClass)SysMLPackage.eINSTANCE.getEClassifier(ownerKind));owner.setIsImpliedIncluded(true);
            var result=SysMLFactory.eINSTANCE.createFeature();result.setDirection(FeatureDirectionKind.OUT);
            var member=SysMLFactory.eINSTANCE.createReturnParameterMembership();member.setOwnedMemberParameter(result);owner.getOwnedRelationship().add(member);
            if((mask&1)!=0)FeatureUtil.addFeatureTypingTo(result).setType(SysMLFactory.eINSTANCE.createClass());
            if((mask&2)!=0)FeatureUtil.addFeatureTypingTo(result).setType(SysMLFactory.eINSTANCE.createStructure());
            if((mask&4)!=0)FeatureUtil.addFeatureTypingTo(result).setType(SysMLFactory.eINSTANCE.createDataType());
            var library=new HashMap<String,Element>();SysMLLibraryUtil.setProviderLookup(r -> (c,n)->library.computeIfAbsent(n,key->{var f=SysMLFactory.eINSTANCE.createFeature();f.setElementId(key);f.setIsImpliedIncluded(true);return f;}));
            var adapter=(org.omg.sysml.adapter.FeatureAdapter)ElementUtil.getElementAdapter(result);
            adapter.forceComputeRedefinitions();adapter.addDefaultGeneralType();
            var generals=new ArrayList<Object>();adapter.forEachImplicitGeneralType((k,g)->generals.add(Map.of("kind",k.getName(),"target",g.getElementId())));
            rows.add(Map.of("owner_kind",ownerKind,"mask",mask,"generals",generals));
        }
        return rows;
    }
    static List<Object> expressionOwnedBindingDefaults() {
        var rows=new ArrayList<Object>();
        for(var classifier:SysMLPackage.eINSTANCE.getEClassifiers()) {
            if(!(classifier instanceof org.eclipse.emf.ecore.EClass kind)||kind.isAbstract()||kind.isInterface()
                ||!SysMLPackage.eINSTANCE.getExpression().isSuperTypeOf(kind))continue;
            var owner=(Expression)SysMLFactory.eINSTANCE.create(kind);
            var source=SysMLFactory.eINSTANCE.createFeature();source.setIsImpliedIncluded(true);
            var target=SysMLFactory.eINSTANCE.createFeature();target.setIsImpliedIncluded(true);
            var binding=ConnectorUtil.createBindingConnector(source,target);
            var member=SysMLFactory.eINSTANCE.createOwningMembership();member.setOwnedMemberElement(binding);owner.getOwnedRelationship().add(member);
            var library=new HashMap<String,Element>();SysMLLibraryUtil.setProviderLookup(r -> (c,n)->library.computeIfAbsent(n,key->{var f=SysMLFactory.eINSTANCE.createFeature();f.setElementId(key);f.setIsImpliedIncluded(true);return f;}));
            var adapter=(org.omg.sysml.adapter.FeatureAdapter)ElementUtil.getElementAdapter(binding);
            adapter.addDefaultGeneralType();
            var generals=new ArrayList<Object>();adapter.forEachImplicitGeneralType((k,g)->generals.add(Map.of("kind",k.getName(),"target",g.getElementId())));
            rows.add(Map.of("owner_kind",kind.getName(),"generals",generals));
        }
        return rows;
    }
    static Map<String,Object> expressionDefaults() throws Exception {
        var cases=new ArrayList<Object>();var bindings=new TreeMap<String,String>();var negatedBindings=new TreeMap<String,String>();var selectors=new TreeMap<String,String>();var excluded=new TreeMap<String,String>();
        for(var classifier:SysMLPackage.eINSTANCE.getEClassifiers()) {
            if(!(classifier instanceof org.eclipse.emf.ecore.EClass kind)||kind.isAbstract()||kind.isInterface()
                ||!SysMLPackage.eINSTANCE.getExpression().isSuperTypeOf(kind))continue;
            var probe=(Expression)SysMLFactory.eINSTANCE.create(kind);
            var method=ElementUtil.getElementAdapter(probe).getClass().getMethod("addDefaultGeneralType");
            if(method.getDeclaringClass()!=org.omg.sysml.adapter.ExpressionAdapter.class){excluded.put(kind.getName(),method.getDeclaringClass().getName());continue;}
            java.lang.Class<?> selectorClass=ElementUtil.getElementAdapter(probe).getClass();java.lang.reflect.Method selector=null;
            while(selector==null){try{selector=selectorClass.getDeclaredMethod("getDefaultSupertype");}catch(NoSuchMethodException e){selectorClass=selectorClass.getSuperclass();}}
            selectors.put(kind.getName(),selectorClass.getName());
            if(selectorClass==org.omg.sysml.adapter.InvariantAdapter.class)negatedBindings.put(kind.getName(),ImplicitGeneralizationMap.getDefaultSupertypeFor(probe.getClass(),"negated"));
            String name=ImplicitGeneralizationMap.getDefaultSupertypeFor(probe.getClass(),"base");
            if(name==null)throw new IllegalStateException("Missing expression default");bindings.put(kind.getName(),name);
            for(String context:List.of("detached","value","package"))for(boolean composite:List.of(false,true))for(boolean negated:probe instanceof Invariant?List.of(false,true):List.of(false)) {
                var expression=(Expression)SysMLFactory.eINSTANCE.create(kind);expression.setIsComposite(composite);if(expression instanceof Invariant invariant)invariant.setIsNegated(negated);
                if(context.equals("value")){var owner=SysMLFactory.eINSTANCE.createFeature();var value=SysMLFactory.eINSTANCE.createFeatureValue();value.setValue(expression);owner.getOwnedRelationship().add(value);}
                if(context.equals("package")){var owner=SysMLFactory.eINSTANCE.createLibraryPackage();var member=SysMLFactory.eINSTANCE.createOwningMembership();member.setOwnedMemberElement(expression);owner.getOwnedRelationship().add(member);}
                var library=new HashMap<String,Element>();
                SysMLLibraryUtil.setProviderLookup(r -> (c,n)->library.computeIfAbsent(n,key->{var f=SysMLFactory.eINSTANCE.createFeature();f.setElementId(key);f.setIsImpliedIncluded(true);return f;}));
                var adapter=(org.omg.sysml.adapter.TypeAdapter)ElementUtil.getElementAdapter(expression);adapter.addDefaultGeneralType();
                var generals=new ArrayList<Object>();adapter.forEachImplicitGeneralType((k,g)->generals.add(Map.of("kind",k.getName(),"target",g.getElementId())));
                cases.add(Map.of("kind",kind.getName(),"context",context,"composite",composite,"negated",negated,"generals",generals));
            }
        }
        return Map.of("bindings",bindings,"negated_bindings",negatedBindings,"selectors",selectors,"excluded",excluded,"cases",cases,"library_result_case",expressionLibraryResult(false),"inherited_binding_case",expressionLibraryResult(true),"result_defaults",resultDefaults(),"binding_defaults",expressionOwnedBindingDefaults());
    }
    static List<Object> bindingStages() throws Exception {
        var injector=new org.omg.kerml.xtext.KerMLStandaloneSetup().createInjectorAndDoEMFRegistration();
        var controls=new ArrayList<Object>();
        String source="standard library package Base { feature things; } standard library package Links { assoc Link { feature participant; } feature selfLinks; } package P { class A { feature x; feature y; } class B; }";
        for(String ownerName:List.of("A","B"))for(String targetName:List.of("x","y")) {
            var resources=injector.getInstance(org.eclipse.xtext.resource.XtextResourceSet.class);
            var resource=resources.createResource(org.eclipse.emf.common.util.URI.createURI("memory:/binding-stage.kerml"));
            resource.load(new java.io.ByteArrayInputStream(source.getBytes(java.nio.charset.StandardCharsets.UTF_8)),Map.of());
            var names=new HashMap<String,Element>();var qualified=new HashMap<String,Element>();
            var all=resource.getAllContents();while(all.hasNext())if(all.next() instanceof Element e && e.getDeclaredName()!=null){names.put(e.getDeclaredName(),e);qualified.put(e.getQualifiedName(),e);}
            SysMLLibraryUtil.setProviderLookup(r -> (context,name)->qualified.get(name));
            ElementUtil.transformAll(names.get("x"),true);
            ElementUtil.transformAll(names.get("y"),true);
            var owner=(Type)names.get(ownerName);
            var adapter=(org.omg.sysml.adapter.TypeAdapter)ElementUtil.getElementAdapter(owner);
            var binding=adapter.addBindingConnector((Feature)names.get("x"),(Feature)names.get(targetName));
            TypeUtil.insertImplicitBindingConnectors(owner);
            var ends=new ArrayList<Object>();for(var end:binding.getOwnedFeature())ends.add(bindingSnapshot(end,binding));
            if(!resource.getErrors().isEmpty())throw new IllegalStateException(resource.getErrors().toString());
            controls.add(Map.of("source",source,"owner",ownerName,"target",targetName,"membership",binding.getOwningRelationship().eClass().getName(),"connector",bindingSnapshot(binding,binding),"ends",ends,"stored_subtree",storedSubtree(binding.getOwningRelationship()),"stored_document",storedSubtree((Element)resource.getContents().get(0))));
        }
        return controls;
    }

    public static void main(String[] args) throws Exception {
        org.omg.sysml.logic.SysMLLogicStandaloneSetup.doSetup();
        var definitions=new TreeMap<String,Object>();var expressionDefinitions=new TreeMap<String,Object>();var compiler=ToolProvider.getSystemJavaCompiler();var diagnostics=new DiagnosticCollector<JavaFileObject>();
        try(var files=compiler.getStandardFileManager(diagnostics,null,null)) {
            var task=(JavacTask)compiler.getTask(null,files,diagnostics,List.of("-proc:none","-classpath",args[0]),null,files.getJavaFileObjects(Arrays.copyOfRange(args,2,args.length)));
            var units=new ArrayList<CompilationUnitTree>();task.parse().forEach(units::add);task.analyze();
            if(diagnostics.getDiagnostics().stream().anyMatch(d->d.getKind()==Diagnostic.Kind.ERROR))throw new IllegalStateException(diagnostics.getDiagnostics().toString());
            var trees=Trees.instance(task);
            for(var unit:units)for(var decl:unit.getTypeDecls())if(decl instanceof ClassTree cls)
                for(var member:cls.getMembers())if(member instanceof MethodTree m) {
                    if(Set.of("isCompatible","dynamicInvoke","canAccess","getContextTypeFor").contains(m.getName().toString()))
                        definitions.put(cls.getSimpleName()+"#"+m.getName()+(m.getName().contentEquals("canAccess")?"/"+m.getParameters().size():""),PilotResultConstructionExporter.tree(unit,m.getBody(),trees));
                    if(cls.getSimpleName().contentEquals("InvariantAdapter") && m.getName().contentEquals("getDefaultSupertype"))
                        expressionDefinitions.put("invariant_selector",PilotResultConstructionExporter.tree(unit,m.getBody(),trees));
                    if(cls.getSimpleName().contentEquals("ExpressionAdapter") && Set.of("getDefaultSupertype","addDefaultGeneralType").contains(m.getName().toString()))
                        expressionDefinitions.put(m.getName().toString(),PilotResultConstructionExporter.tree(unit,m.getBody(),trees));
                }
        }
        var cases=new ArrayList<Object>();
        for(String shape:List.of("unrelated","same_context","narrower_context","wider_context","different_context","unfeatured_super","owned_feature","direct_specialization","variable_owner","first_variable","nested_positive","nested_negative","nested_cycle","nested_diamond","global_positive","global_negative","global_empty","nested_global")) {
            var nodes=new LinkedHashMap<String,Type>();
            for(String id:List.of("sub","sup","base","child","first"))nodes.put(id,SysMLFactory.eINSTANCE.createFeature());
            for(String id:List.of("a","b","c","anything"))nodes.put(id,SysMLFactory.eINSTANCE.createType());
            nodes.put("anything",SysMLFactory.eINSTANCE.createClassifier());
            nodes.forEach((id,n)->{n.setElementId(id);n.setIsImpliedIncluded(true);});
            SysMLLibraryUtil.setProviderLookup(resource -> (context,name) -> name.equals("Base::Anything")?nodes.get("anything"):null);
            var edges=new ArrayList<List<String>>();
            edges.add(List.of("Specialization","b","a"));
            if(!shape.equals("unrelated")) { edges.add(List.of("Redefinition","sub","base"));edges.add(List.of("Redefinition","sup","base")); }
            if(shape.equals("direct_specialization"))edges.add(List.of("Subsetting","sub","sup"));
            String subContext=shape.startsWith("nested_")?"first":shape.equals("narrower_context")?"b":"a";
            if(shape.startsWith("nested_") && !shape.equals("nested_global"))edges.add(List.of("TypeFeaturing","first",switch(shape){case "nested_positive"->"a";case "nested_cycle"->"sub";case "nested_diamond"->"child";default->"c";}));
            if(shape.equals("nested_diamond")){edges.add(List.of("TypeFeaturing","first","a"));edges.add(List.of("TypeFeaturing","child","a"));}
            String superContext=switch(shape){case "wider_context"->"b";case "different_context","variable_owner","first_variable"->"c";default->"a";};
            if(!shape.startsWith("global_"))edges.add(List.of("TypeFeaturing","sub",subContext));
            if(!shape.equals("unfeatured_super") && !shape.equals("global_empty"))edges.add(List.of("TypeFeaturing","sup",shape.equals("global_positive") || shape.equals("nested_global")?"anything":superContext));
            if(shape.equals("owned_feature"))edges.add(List.of("FeatureMembership","sup","child"));
            if(shape.equals("variable_owner")){((Feature)nodes.get("sup")).setIsVariable(true);edges.add(List.of("FeatureMembership","a","sup"));}
            if(shape.equals("first_variable")){((Feature)nodes.get("first")).setIsVariable(true);edges.add(List.of("FeatureMembership","a","first"));edges.add(List.of("FeatureChaining","sup","first"));}
            for(var edge:edges){
                Type source=nodes.get(edge.get(1)),target=nodes.get(edge.get(2));
                switch(edge.get(0)){
                    case "FeatureMembership"->TypeUtil.addOwnedFeatureTo(source,(Feature)target);
                    case "TypeFeaturing"->{var r=SysMLFactory.eINSTANCE.createTypeFeaturing();r.setFeatureOfType((Feature)source);r.setFeaturingType(target);source.getOwnedRelationship().add(r);}
                    case "FeatureChaining"->{var r=SysMLFactory.eINSTANCE.createFeatureChaining();r.setChainingFeature((Feature)target);source.getOwnedRelationship().add(r);}
                    default->{Specialization r=switch(edge.get(0)){case "Redefinition"->SysMLFactory.eINSTANCE.createRedefinition();case "Subsetting"->SysMLFactory.eINSTANCE.createSubsetting();default->SysMLFactory.eINSTANCE.createSpecialization();};r.setGeneral(target);r.setSpecific(source);source.getOwnedRelationship().add(r);}
                }
            }
            var kinds=new TreeMap<String,String>();nodes.forEach((id,n)->kinds.put(id,n.eClass().getName()));
            cases.add(Map.of("shape",shape,"nodes",kinds,"edges",edges,"variable",shape.equals("variable_owner")?"sup":shape.equals("first_variable")?"first":"","can_access",FeatureUtil.canAccess((Feature)nodes.get("sub"),(Feature)nodes.get("sup")),"compatible",TypeUtil.isCompatible(nodes.get("sub"),nodes.get("sup"))));
        }
        var contextCases=new ArrayList<Object>();
        for(String shape:List.of("empty","same","narrower","wider","unrelated","source_context","target_context","multiple","duplicate","nested")) {
            var nodes=new LinkedHashMap<String,Type>();
            for(String id:List.of("sub","sup","first"))nodes.put(id,SysMLFactory.eINSTANCE.createFeature());
            for(String id:List.of("a","b","c"))nodes.put(id,SysMLFactory.eINSTANCE.createType());
            nodes.forEach((id,n)->{n.setElementId(id);n.setIsImpliedIncluded(true);});
            var edges=new ArrayList<List<String>>();edges.add(List.of("Specialization","b","a"));
            switch(shape){
                case "source_context"->edges.add(List.of("TypeFeaturing","sup","sub"));
                case "target_context"->edges.add(List.of("TypeFeaturing","sub","sup"));
                case "empty"->{}
                default->{
                    edges.add(List.of("TypeFeaturing","sub",shape.equals("wider")?"b":shape.equals("nested")?"first":"a"));
                    edges.add(List.of("TypeFeaturing","sup",shape.equals("narrower")||shape.equals("multiple")?"b":shape.equals("unrelated")?"c":"a"));
                    if(shape.equals("multiple"))edges.add(List.of("TypeFeaturing","sub","c"));
                    if(shape.equals("nested"))edges.add(List.of("TypeFeaturing","first","a"));
                }
            }
            for(var edge:edges){
                var source=nodes.get(edge.get(1));var target=nodes.get(edge.get(2));
                if(edge.get(0).equals("TypeFeaturing")){var r=SysMLFactory.eINSTANCE.createTypeFeaturing();r.setFeatureOfType((Feature)source);r.setFeaturingType(target);source.getOwnedRelationship().add(r);}
                else {var r=SysMLFactory.eINSTANCE.createSpecialization();r.setSpecific(source);r.setGeneral(target);source.getOwnedRelationship().add(r);}
            }
            String target=shape.equals("duplicate")?"sub":"sup";
            var connector=ConnectorUtil.createBindingConnector((Feature)nodes.get("sub"),(Feature)nodes.get(target));
            connector.setIsImpliedIncluded(true);
            for(var end:connector.getOwnedFeature())end.setIsImpliedIncluded(true);
            var context=ConnectorUtil.getContextTypeFor(connector);
            var kinds=new TreeMap<String,String>();nodes.forEach((id,n)->kinds.put(id,n.eClass().getName()));
            var placements=new ArrayList<Object>();
            SysMLLibraryUtil.setProviderLookup(resource -> (owner,name) -> null);
            for(String owner:List.of("a","c")) {
                var adapter=(org.omg.sysml.adapter.TypeAdapter)ElementUtil.getElementAdapter(nodes.get(owner));
                var binding=adapter.addBindingConnector((Feature)nodes.get("sub"),(Feature)nodes.get(target));
                adapter.forEachImplicitBindingConnector((item,kind)->{
                    if(item==binding)placements.add(Map.of("owner",owner,"membership",kind.getName(),"featuring",item.getFeaturingType().stream().map(Element::getElementId).toList()));
                });
            }
            contextCases.add(Map.of("shape",shape,"nodes",kinds,"edges",edges,"source","sub","target",target,"context",context==null?"":context.getElementId(),"placements",placements));
        }
        Files.writeString(Path.of(args[1]),new GsonBuilder().setPrettyPrinting().create().toJson(Map.of("definitions",definitions,"cases",cases,"context_cases",contextCases,"binding_stages",bindingStages(),"inclusion_cases",inclusionCases(),"value_cases",valueCases(),"expression_featuring_cases",expressionFeaturingCases(),"expression_default_definitions",expressionDefinitions,"expression_defaults",expressionDefaults()))+"\n");
    }
}
