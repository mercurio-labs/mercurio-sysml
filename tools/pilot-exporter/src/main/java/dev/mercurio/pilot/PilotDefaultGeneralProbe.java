package dev.mercurio.pilot;
import com.google.gson.GsonBuilder;
import java.nio.file.*;
import java.util.*;
import com.sun.source.tree.*;
import com.sun.source.util.*;
import javax.tools.*;
import org.eclipse.emf.ecore.*;
import org.omg.sysml.lang.sysml.*;
import org.omg.sysml.util.*;

/** Observe upstream default selection and general-type computation; only library lookup is supplied. */
public final class PilotDefaultGeneralProbe {
    static java.lang.reflect.Method method(java.lang.Class<?> type,String name,java.lang.Class<?>... parameters)throws Exception {
        while(type!=null) {try{return type.getDeclaredMethod(name,parameters);}catch(NoSuchMethodException ignored){type=type.getSuperclass();}}
        throw new IllegalStateException("Missing adapter method "+name);
    }
    static String key(java.lang.reflect.Method m){return m.getDeclaringClass().getName()+"#"+m.getName();}
    public static void main(String[] args) throws Exception {
        org.omg.sysml.logic.SysMLLogicStandaloneSetup.doSetup();
        var factory=SysMLFactory.eINSTANCE;
        var methods=new TreeMap<String,Object>();var compiler=ToolProvider.getSystemJavaCompiler();var diagnostics=new DiagnosticCollector<JavaFileObject>();
        try(var files=compiler.getStandardFileManager(diagnostics,null,java.nio.charset.StandardCharsets.UTF_8)) {
            var task=(JavacTask)compiler.getTask(null,files,diagnostics,List.of("-proc:none","-classpath",args[1]),null,files.getJavaFileObjects(args[2]));
            var units=new ArrayList<CompilationUnitTree>();task.parse().forEach(units::add);task.analyze();
            if(diagnostics.getDiagnostics().stream().anyMatch(d->d.getKind()==Diagnostic.Kind.ERROR))throw new IllegalStateException("Unresolved source "+diagnostics.getDiagnostics());
            var trees=Trees.instance(task);
            for(var unit:units)for(var declaration:unit.getTypeDecls())if(declaration instanceof ClassTree type)
                for(var member:type.getMembers())if(member instanceof MethodTree m && m.getName().contentEquals("getDefaultSupertype") && m.getParameters().isEmpty())
                    methods.put(trees.getElement(TreePath.getPath(unit,type))+"#"+m.getName(),PilotStepStrategyProbe.node(unit,m.getBody(),trees));
        }
        var admitted=new TreeMap<String,Map<String,String>>();var excluded=new TreeMap<String,Object>();
        for(var classifier:SysMLPackage.eINSTANCE.getEClassifiers()) {
            if(!(classifier instanceof EClass cls)||cls.isAbstract())continue;
            var sample=factory.create(cls);if(!(sample instanceof Type target)||sample instanceof Feature)continue;
            var adapter=ElementUtil.getElementAdapter(target);var dispatch=new TreeMap<String,String>();
            dispatch.put("getDefaultSupertype",key(method(adapter.getClass(),"getDefaultSupertype")));
            dispatch.put("addDefaultGeneralType",key(method(adapter.getClass(),"addDefaultGeneralType")));
            dispatch.put("getInheritedMemberships",key(method(adapter.getClass(),"getInheritedMemberships",Set.class,Set.class,boolean.class)));
            dispatch.put("getGeneralTypesOf",key(TypeUtil.class.getDeclaredMethod("getGeneralTypesOf",Type.class)));
            dispatch.put("doTransform",key(method(adapter.getClass(),"doTransform")));
            if(dispatch.get("getDefaultSupertype").equals("org.omg.sysml.adapter.TypeAdapter#getDefaultSupertype") && dispatch.get("addDefaultGeneralType").equals("org.omg.sysml.adapter.TypeAdapter#addDefaultGeneralType") && dispatch.get("getInheritedMemberships").equals("org.omg.sysml.adapter.TypeAdapter#getInheritedMemberships"))admitted.put(cls.getName(),dispatch);
            else excluded.put(cls.getName(),dispatch);
        }
        List<Object> bindings=new ArrayList<>(), controls=new ArrayList<>();
        var kinds=new ArrayList<String>(List.of("Type","Classifier","Class","DataType","Structure"));
        for(var kind:admitted.keySet())if(!kinds.contains(kind))kinds.add(kind);
        for (String kind:kinds) {
            EClass cls=(EClass)SysMLPackage.eINSTANCE.getEClassifier(kind);
            Type sample=(Type)factory.create(cls);
            var actual=ElementUtil.getElementAdapter(sample);var specializationMethod=method(actual.getClass(),"getSpecializationEClass");specializationMethod.setAccessible(true);
            String specialization=((EClass)specializationMethod.invoke(actual)).getName();
            String name=ImplicitGeneralizationMap.getDefaultSupertypeFor(sample.getClass(), "base");
            if(name==null) throw new IllegalStateException("Missing default for "+kind);
            bindings.add(Map.of("kind",kind,"qualified_name",name,"specialization",specialization,"dispatch",admitted.get(kind)));
            for(int mode=0;mode<7;mode++) {
                Type owner=(Type)factory.create(cls), target=(Type)factory.create(cls), explicit=(Type)factory.create(cls);
                Type selected=mode==3?owner:target;
                SysMLLibraryUtil.setProviderLookup(resource->(context,qualified)->qualified.equals(name)?selected:null);
                var own=factory.createMembership(); own.setMemberName("Own");own.setMemberElement(owner);owner.getOwnedRelationship().add(own);
                var inherited=factory.createMembership(); inherited.setMemberName("Inherited");inherited.setMemberElement(explicit);target.getOwnedRelationship().add(inherited);
                if(mode==4||mode==5)inherited.setVisibility(mode==4?VisibilityKind.PRIVATE:VisibilityKind.PROTECTED);
                Map<EObject,String> ids=new IdentityHashMap<>(); ids.put(owner,"owner");ids.put(target,"default");ids.put(explicit,"explicit"); ids.put(own,"own");ids.put(inherited,"inherited");
                List<Object> graph=new ArrayList<>();
                Map<String,Object> root=new LinkedHashMap<>(); root.put("@id","owner");root.put("@type",kind);
                root.put("ownedRelationship", mode==1||mode==2||mode==6?List.of(Map.of("@id","own"),Map.of("@id","specialization")):List.of(Map.of("@id","own"))); graph.add(root);
                for(String id:List.of("default","explicit")) graph.add(Map.of("@id",id,"@type",kind,"ownedRelationship",id.equals("default")?List.of(Map.of("@id","inherited")):List.of()));
                graph.add(Map.of("@id","own","@type","Membership","owningRelatedElement",Map.of("@id","owner"),"memberElement",Map.of("@id","owner"),"memberName","Own"));
                graph.add(Map.of("@id","inherited","@type","Membership","owningRelatedElement",Map.of("@id","default"),"memberElement",Map.of("@id","explicit"),"memberName","Inherited","visibility",mode==4?"private":mode==5?"protected":"public"));
                if(mode==1||mode==2||mode==6) {
                    Specialization relation=owner instanceof Classifier?factory.createSubclassification():factory.createSpecialization();
                    relation.setSpecific(owner);relation.setGeneral(mode==1?explicit:target);relation.setIsImplied(mode==6);owner.getOwnedRelationship().add(relation);
                    graph.add(Map.of("@id","specialization","@type",relation.eClass().getName(),"owningRelatedElement",Map.of("@id","owner"),"isImplied",mode==6,
                        owner instanceof Classifier?"subclassifier":"specific",Map.of("@id","owner"),
                        owner instanceof Classifier?"superclassifier":"general",Map.of("@id",mode==1?"explicit":"default")));
                }
                controls.add(Map.of("kind",kind,"mode",mode,"graph",graph,"library_bindings",Map.of(name,mode==3?"owner":"default"),
                    "general_types",TypeUtil.getGeneralTypesOf(owner).stream().map(t->Objects.requireNonNull(ids.get(t))).toList(),
                    "membership",owner.getMembership().stream().map(t->Objects.requireNonNull(ids.get(t))).toList()));
            }
        }
        SysMLLibraryUtil.setProviderLookup(null);
        Files.writeString(Path.of(args[0]),new GsonBuilder().setPrettyPrinting().create().toJson(Map.of("bindings",bindings,"controls",controls,"excluded_bindings",excluded,"methods",methods)));
    }
}
