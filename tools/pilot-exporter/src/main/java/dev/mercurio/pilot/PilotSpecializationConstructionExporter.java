package dev.mercurio.pilot;
import com.google.gson.GsonBuilder;
import com.sun.source.tree.*;
import com.sun.source.util.*;
import javax.tools.*;
import java.nio.file.*;
import java.util.*;
import org.eclipse.emf.ecore.EClass;
import org.omg.sysml.lang.sysml.*;
import org.omg.sysml.util.*;
public final class PilotSpecializationConstructionExporter {
    public static void main(String[] args) throws Exception {
        org.omg.sysml.logic.SysMLLogicStandaloneSetup.doSetup();
        var definitions=new TreeMap<String,Object>();var compiler=ToolProvider.getSystemJavaCompiler();var diagnostics=new DiagnosticCollector<JavaFileObject>();
        try(var files=compiler.getStandardFileManager(diagnostics,null,null)) {
            var task=(JavacTask)compiler.getTask(null,files,diagnostics,List.of("-proc:none","-classpath",args[0]),null,files.getJavaFileObjects(Arrays.copyOfRange(args,2,args.length)));
            var units=new ArrayList<CompilationUnitTree>();task.parse().forEach(units::add);task.analyze();
            if(diagnostics.getDiagnostics().stream().anyMatch(d->d.getKind()==Diagnostic.Kind.ERROR))throw new IllegalStateException(diagnostics.getDiagnostics().toString());
            var trees=Trees.instance(task);
            for(var unit:units)for(var decl:unit.getTypeDecls())if(decl instanceof ClassTree cls)
                for(var member:cls.getMembers())if(member instanceof MethodTree m && Set.of("insertImplicitSpecializations","addResultSubsetting","getReferentFor").contains(m.getName().toString()))
                    definitions.put(cls.getSimpleName()+"#"+m.getName(),PilotResultConstructionExporter.tree(unit,m.getBody(),trees));
        }
        var kinds=new ArrayList<String>();var cases=new ArrayList<Object>();
        for(var classifier:SysMLPackage.eINSTANCE.getEClassifiers()) {
            if(!(classifier instanceof EClass cls)||cls.isAbstract()||!SysMLPackage.eINSTANCE.getSpecialization().isSuperTypeOf(cls))continue;
            kinds.add(cls.getName());
            for(String ownership:List.of("detached","owned","unowned_membership")) {
                Type specific=SysMLPackage.eINSTANCE.getSubclassification().isSuperTypeOf(cls)?SysMLFactory.eINSTANCE.createClassifier():SysMLPackage.eINSTANCE.getSubsetting().isSuperTypeOf(cls)||SysMLPackage.eINSTANCE.getFeatureTyping().isSuperTypeOf(cls)?SysMLFactory.eINSTANCE.createFeature():SysMLFactory.eINSTANCE.createType();
                Type general=SysMLPackage.eINSTANCE.getSubclassification().isSuperTypeOf(cls)?SysMLFactory.eINSTANCE.createClassifier():SysMLPackage.eINSTANCE.getSubsetting().isSuperTypeOf(cls)?SysMLFactory.eINSTANCE.createFeature():SysMLFactory.eINSTANCE.createType();
                if(cls == SysMLPackage.eINSTANCE.getConjugatedPortTyping()) general=SysMLFactory.eINSTANCE.createConjugatedPortDefinition();
                specific.setElementId("specific");general.setElementId("general");
                if(!ownership.equals("detached")) { var member=SysMLFactory.eINSTANCE.createOwningMembership();member.setElementId("container");member.setOwnedMemberElement(general);if(ownership.equals("owned")){var root=SysMLFactory.eINSTANCE.createNamespace();root.getOwnedRelationship().add(member);} }
                TypeUtil.addImplicitGeneralTypeTo(specific,cls,general);TypeUtil.insertImplicitSpecializations(specific);
                var specs=specific.getOwnedRelationship().stream().filter(Specialization.class::isInstance).map(Specialization.class::cast).toList();
                var observed=new ArrayList<Object>();for(var spec:specs)observed.add(Map.of("kind",spec.eClass().getName(),"specific",spec.getSpecific().getElementId(),"general",spec.getGeneral().getElementId(),"implied",spec.isImplied(),"adopted",general.getOwningRelationship()==spec));
                TypeUtil.insertImplicitSpecializations(specific);
                cases.add(Map.of("kind",cls.getName(),"specific_kind",specific.eClass().getName(),"general_kind",general.eClass().getName(),"ownership",ownership,"observed",observed,"second_insert_count",specific.getOwnedRelationship().size()));
            }
        }
        var resultCases=new ArrayList<Object>();
        var resultMethod=org.omg.sysml.adapter.FeatureReferenceExpressionAdapter.class.getDeclaredMethod("addResultSubsetting");resultMethod.setAccessible(true);
        for(String context:List.of("absent","feature","classifier","first_classifier","two_features","parameter_first"))for(boolean contained:List.of(false,true)) {
            var expression=SysMLFactory.eINSTANCE.createFeatureReferenceExpression();
            TypeUtil.addResultParameterTo(expression);var result=TypeUtil.getOwnedResultParameterOf(expression);result.setElementId("result");
            var target=SysMLFactory.eINSTANCE.createFeature();target.setElementId("target");
            if(contained){var member=SysMLFactory.eINSTANCE.createOwningMembership();member.setOwnedMemberElement(target);var root=SysMLFactory.eINSTANCE.createNamespace();root.getOwnedRelationship().add(member);}
            if(context.equals("parameter_first")){var parameter=SysMLFactory.eINSTANCE.createParameterMembership();parameter.setOwnedMemberParameter(SysMLFactory.eINSTANCE.createFeature());expression.getOwnedRelationship().add(parameter);}
            if(context.equals("classifier")||context.equals("first_classifier")){var member=SysMLFactory.eINSTANCE.createMembership();member.setMemberElement(SysMLFactory.eINSTANCE.createClassifier());expression.getOwnedRelationship().add(member);}
            if(List.of("feature","first_classifier","two_features","parameter_first").contains(context)){var member=SysMLFactory.eINSTANCE.createMembership();member.setMemberElement(target);expression.getOwnedRelationship().add(member);}
            if(context.equals("two_features")){var member=SysMLFactory.eINSTANCE.createMembership();member.setMemberElement(SysMLFactory.eINSTANCE.createFeature());expression.getOwnedRelationship().add(member);}
            resultMethod.invoke(ElementUtil.getElementAdapter(expression));TypeUtil.insertImplicitSpecializations(result);
            var observations=new ArrayList<Object>();for(var relation:result.getOwnedRelationship())if(relation instanceof Specialization spec) observations.add(Map.of("kind",spec.eClass().getName(),"general",spec.getGeneral().getElementId(),"implied",spec.isImplied(),"adopted",target.getOwningRelationship()==spec));
            resultCases.add(Map.of("context",context,"contained",contained,"observed",observations,"referent",expression.getReferent()==null?"":expression.getReferent().getElementId()));
        }
        var selfCases=new ArrayList<Object>();
        for(String context:List.of("absent","feature","classifier")) {
            var self=SysMLFactory.eINSTANCE.createFeature();self.setElementId("self");
            SysMLLibraryUtil.setProviderLookup(resource -> (owner,name) -> name.equals(ExpressionUtil.SELF_REFERENCE_FEATURE)?self:null);
            var expression=SysMLFactory.eINSTANCE.createFeatureReferenceExpression();
            if(!context.equals("absent")) {
                var member=SysMLFactory.eINSTANCE.createMembership();
                var target=context.equals("feature")?SysMLFactory.eINSTANCE.createFeature():SysMLFactory.eINSTANCE.createClassifier();
                target.setElementId("target");member.setMemberElement(target);expression.getOwnedRelationship().add(member);
            }
            var referent=expression.getReferent();
            selfCases.add(Map.of("context",context,"referent",referent==null?"":referent.getElementId()));
        }
        Files.writeString(Path.of(args[1]),new GsonBuilder().setPrettyPrinting().create().toJson(Map.of("definitions",definitions,"kinds",kinds,"cases",cases,"result_cases",resultCases,"self_reference_feature",ExpressionUtil.SELF_REFERENCE_FEATURE,"self_cases",selfCases))+"\n");
    }
}
