package dev.mercurio.pilot;
import com.google.gson.GsonBuilder;
import com.sun.source.tree.*;
import com.sun.source.util.*;
import javax.tools.*;
import java.nio.file.*;
import java.util.*;
import org.omg.sysml.lang.sysml.*;
import org.omg.sysml.util.*;
public final class PilotImplicitReductionExporter {
    public static void main(String[] args) throws Exception {
        org.omg.sysml.logic.SysMLLogicStandaloneSetup.doSetup();
        var definitions=new TreeMap<String,Object>();var compiler=ToolProvider.getSystemJavaCompiler();var diagnostics=new DiagnosticCollector<JavaFileObject>();
        try(var files=compiler.getStandardFileManager(diagnostics,null,null)) {
            var task=(JavacTask)compiler.getTask(null,files,diagnostics,List.of("-proc:none","-classpath",args[0]),null,files.getJavaFileObjects(Arrays.copyOfRange(args,2,args.length)));
            var units=new ArrayList<CompilationUnitTree>();task.parse().forEach(units::add);task.analyze();
            if(diagnostics.getDiagnostics().stream().anyMatch(d->d.getKind()==Diagnostic.Kind.ERROR))throw new IllegalStateException(diagnostics.getDiagnostics().toString());
            var trees=Trees.instance(task);
            for(var unit:units)for(var decl:unit.getTypeDecls())if(decl instanceof ClassTree cls)
                for(var member:cls.getMembers())if(member instanceof MethodTree m && Set.of("removeUnnecessaryImplicitGeneralTypes","specializesExcludingTarget","specializes").contains(m.getName().toString()))
                    definitions.put(cls.getSimpleName()+"#"+m.getName()+"/"+m.getParameters().size(),PilotResultConstructionExporter.tree(unit,m.getBody(),trees));
        }
        var kinds=new TreeMap<String,Integer>();
        for(String kind:List.of("Specialization","Subsetting","Redefinition","FeatureTyping","Subclassification","ReferenceSubsetting","CrossSubsetting","ConjugatedPortTyping"))kinds.put(kind,((org.eclipse.emf.ecore.EClass)SysMLPackage.eINSTANCE.getEClassifier(kind)).getClassifierID());
        var cases=new ArrayList<Object>();
        for(String shape:List.of("empty","explicit_same","explicit_narrower","implicit_narrower","unrelated","explicit_redefinition","explicit_subsetting","implicit_redefinition","cross_category","owner_cycle","general_cycle","explicit_self","ordered")) {
            var nodes=new LinkedHashMap<String,Feature>();
            for(String id:List.of("owner","a","b","c")){var f=SysMLFactory.eINSTANCE.createFeature();f.setElementId(id);f.setIsImpliedIncluded(true);nodes.put(id,f);}
            var edges=new ArrayList<List<String>>();var pending=new ArrayList<List<String>>();
            switch(shape){
                case "empty"->{}
                case "explicit_same"->{edges.add(List.of("Subsetting","owner","a"));pending.add(List.of("Subsetting","a"));}
                case "explicit_narrower"->{edges.add(List.of("Subsetting","owner","a"));edges.add(List.of("Subsetting","a","b"));pending.add(List.of("Subsetting","b"));}
                case "implicit_narrower"->{edges.add(List.of("Subsetting","a","b"));pending.add(List.of("Subsetting","a"));pending.add(List.of("Subsetting","b"));}
                case "unrelated"->{pending.add(List.of("Subsetting","a"));pending.add(List.of("Subsetting","b"));}
                case "explicit_redefinition"->{edges.add(List.of("Redefinition","owner","a"));pending.add(List.of("Redefinition","a"));}
                case "explicit_subsetting"->{edges.add(List.of("Subsetting","owner","a"));pending.add(List.of("Redefinition","a"));}
                case "implicit_redefinition"->{edges.add(List.of("Subsetting","a","b"));pending.add(List.of("Redefinition","a"));pending.add(List.of("Redefinition","b"));}
                case "cross_category"->{edges.add(List.of("Subsetting","a","b"));pending.add(List.of("Redefinition","a"));pending.add(List.of("Subsetting","b"));}
                case "owner_cycle"->{edges.add(List.of("Subsetting","owner","a"));edges.add(List.of("Subsetting","a","owner"));pending.add(List.of("Subsetting","b"));}
                case "general_cycle"->{edges.add(List.of("Subsetting","a","b"));edges.add(List.of("Subsetting","b","a"));pending.add(List.of("Subsetting","a"));pending.add(List.of("Subsetting","b"));}
                case "explicit_self"->{edges.add(List.of("Subsetting","owner","owner"));pending.add(List.of("Subsetting","a"));}
                case "ordered"->{pending.add(List.of("Subsetting","c"));pending.add(List.of("Redefinition","a"));pending.add(List.of("Specialization","b"));pending.add(List.of("Subsetting","a"));}
            }
            for(var e:edges){var r=(Specialization)SysMLFactory.eINSTANCE.create((org.eclipse.emf.ecore.EClass)SysMLPackage.eINSTANCE.getEClassifier(e.get(0)));r.setSpecific(nodes.get(e.get(1)));r.setGeneral(nodes.get(e.get(2)));nodes.get(e.get(1)).getOwnedRelationship().add(r);}
            var adapter=(org.omg.sysml.adapter.TypeAdapter)ElementUtil.getElementAdapter(nodes.get("owner"));
            for(var item:pending)adapter.addImplicitGeneralType((org.eclipse.emf.ecore.EClass)SysMLPackage.eINSTANCE.getEClassifier(item.get(0)),nodes.get(item.get(1)));
            adapter.removeUnnecessaryImplicitGeneralTypes();
            var remaining=new ArrayList<List<String>>();adapter.forEachImplicitGeneralType((cls,g)->remaining.add(List.of(cls.getName(),g.getElementId())));
            adapter.addImplicitGeneralType(SysMLPackage.eINSTANCE.getSubsetting(),nodes.get("c"));
            var afterClosed=new ArrayList<List<String>>();adapter.forEachImplicitGeneralType((cls,g)->afterClosed.add(List.of(cls.getName(),g.getElementId())));
            cases.add(Map.of("shape",shape,"edges",edges,"pending",pending,"remaining",remaining,"after_closed",afterClosed));
        }
        for(String kind:List.of("FeatureTyping","Subclassification","ReferenceSubsetting","CrossSubsetting","ConjugatedPortTyping"))for(String shape:List.of("unrelated","explicit_same","explicit_narrower","implicit_narrower")) {
            var nodes=new LinkedHashMap<String,Type>();var nodeKinds=new TreeMap<String,String>();
            for(String id:List.of("owner","a","b","c")) {
                String cls=kind.endsWith("Subsetting") || (id.equals("owner") && kind.equals("FeatureTyping"))?"Feature":"Class";
                if(kind.equals("ConjugatedPortTyping"))cls=id.equals("owner")?"Feature":"ConjugatedPortDefinition";
                var node=(Type)SysMLFactory.eINSTANCE.create((org.eclipse.emf.ecore.EClass)SysMLPackage.eINSTANCE.getEClassifier(cls));
                node.setElementId(id);node.setIsImpliedIncluded(true);nodes.put(id,node);nodeKinds.put(id,cls);
            }
            var edges=new ArrayList<List<String>>();var pending=new ArrayList<List<String>>();
            switch(shape) {
                case "unrelated"->{pending.add(List.of(kind,"a"));pending.add(List.of(kind,"b"));}
                case "explicit_same"->{edges.add(List.of(kind,"owner","a"));pending.add(List.of(kind,"a"));}
                case "explicit_narrower"->{edges.add(List.of(kind,"owner","a"));edges.add(List.of(kind.endsWith("Subsetting")?"Subsetting":"Subclassification","a","b"));pending.add(List.of(kind,"b"));}
                case "implicit_narrower"->{edges.add(List.of(kind.endsWith("Subsetting")?"Subsetting":"Subclassification","a","b"));pending.add(List.of(kind,"a"));pending.add(List.of(kind,"b"));}
            }
            for(var e:edges){var r=(Specialization)SysMLFactory.eINSTANCE.create((org.eclipse.emf.ecore.EClass)SysMLPackage.eINSTANCE.getEClassifier(e.get(0)));r.setSpecific(nodes.get(e.get(1)));r.setGeneral(nodes.get(e.get(2)));nodes.get(e.get(1)).getOwnedRelationship().add(r);}
            var adapter=(org.omg.sysml.adapter.TypeAdapter)ElementUtil.getElementAdapter(nodes.get("owner"));
            var category=(org.eclipse.emf.ecore.EClass)SysMLPackage.eINSTANCE.getEClassifier(kind);
            for(var item:pending)adapter.addImplicitGeneralType(category,nodes.get(item.get(1)));
            adapter.removeUnnecessaryImplicitGeneralTypes();
            var remaining=new ArrayList<List<String>>();adapter.forEachImplicitGeneralType((cls,g)->remaining.add(List.of(cls.getName(),g.getElementId())));
            adapter.addImplicitGeneralType(category,nodes.get("c"));
            var afterClosed=new ArrayList<List<String>>();adapter.forEachImplicitGeneralType((cls,g)->afterClosed.add(List.of(cls.getName(),g.getElementId())));
            cases.add(Map.of("shape",kind+"_"+shape,"nodes",nodeKinds,"edges",edges,"pending",pending,"remaining",remaining,"after_closed",afterClosed));
        }
        Files.writeString(Path.of(args[1]),new GsonBuilder().setPrettyPrinting().create().toJson(Map.of("definitions",definitions,"kinds",kinds,"cases",cases))+"\n");
    }
}
