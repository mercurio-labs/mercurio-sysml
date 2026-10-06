package dev.mercurio.pilot;
import com.google.gson.GsonBuilder;
import com.sun.source.tree.*;
import com.sun.source.util.*;
import javax.tools.*;
import java.nio.file.*;
import java.util.*;
import org.omg.sysml.lang.sysml.*;
import org.omg.sysml.util.*;
public final class PilotFeaturingQueryExporter {
    public static void main(String[] args) throws Exception {
        org.omg.sysml.logic.SysMLLogicStandaloneSetup.doSetup();
        var definitions=new TreeMap<String,Object>();var compiler=ToolProvider.getSystemJavaCompiler();var diagnostics=new DiagnosticCollector<JavaFileObject>();
        try(var files=compiler.getStandardFileManager(diagnostics,null,null)) {
            var task=(JavacTask)compiler.getTask(null,files,diagnostics,List.of("-proc:none","-classpath",args[0]),null,files.getJavaFileObjects(Arrays.copyOfRange(args,2,args.length)));
            var units=new ArrayList<CompilationUnitTree>();task.parse().forEach(units::add);task.analyze();
            if(diagnostics.getDiagnostics().stream().anyMatch(d->d.getKind()==Diagnostic.Kind.ERROR))throw new IllegalStateException(diagnostics.getDiagnostics().toString());
            var trees=Trees.instance(task);
            for(var unit:units)for(var decl:unit.getTypeDecls())if(decl instanceof ClassTree cls)
                for(var member:cls.getMembers())if(member instanceof MethodTree m && Set.of("getAllFeaturingTypesOf","getFirstChainingFeatureOf","basicGet","computeFeaturingType").contains(m.getName().toString()))
                    definitions.put(cls.getSimpleName()+"#"+m.getName(),PilotResultConstructionExporter.tree(unit,m.getBody(),trees));
        }
        var cases=new ArrayList<Object>();
        for(String shape:List.of("empty","direct","duplicate","breadth","diamond","cycle","self","first_chain","value","computed_owner")) {
            var nodes=new LinkedHashMap<String,Type>();
            nodes.put("root",shape.equals("value")?SysMLFactory.eINSTANCE.createExpression():SysMLFactory.eINSTANCE.createFeature());
            for(String id:List.of("a","b","value_owner"))nodes.put(id,SysMLFactory.eINSTANCE.createFeature());
            for(String id:List.of("t","u","owner"))nodes.put(id,SysMLFactory.eINSTANCE.createType());
            nodes.forEach((id,n)->{n.setElementId(id);n.setIsImpliedIncluded(true);});
            String[][] edges=switch(shape){
                case "empty" -> new String[][]{};
                case "direct" -> new String[][]{{"root","t"}};
                case "duplicate" -> new String[][]{{"root","t"},{"root","t"}};
                case "breadth" -> new String[][]{{"root","a"},{"root","b"},{"a","t"},{"b","u"}};
                case "diamond" -> new String[][]{{"root","a"},{"root","b"},{"a","t"},{"b","t"}};
                case "cycle" -> new String[][]{{"root","a"},{"a","root"}};
                case "self" -> new String[][]{{"root","root"}};
                case "first_chain" -> new String[][]{{"root","u"},{"a","t"},{"b","u"}};
                case "value" -> new String[][]{{"value_owner","t"},{"root","u"}};
                default -> new String[][]{{"root","t"}};
            };
            for(var edge:edges){var relation=SysMLFactory.eINSTANCE.createTypeFeaturing();relation.setFeatureOfType((Feature)nodes.get(edge[0]));relation.setFeaturingType(nodes.get(edge[1]));nodes.get(edge[0]).getOwnedRelationship().add(relation);}
            var root=(Feature)nodes.get("root");
            if(shape.equals("first_chain"))for(String id:List.of("a","b")){var chain=SysMLFactory.eINSTANCE.createFeatureChaining();chain.setChainingFeature((Feature)nodes.get(id));root.getOwnedRelationship().add(chain);}
            if(shape.equals("value")){var value=SysMLFactory.eINSTANCE.createFeatureValue();value.setValue((Expression)root);nodes.get("value_owner").getOwnedRelationship().add(value);}
            if(shape.equals("computed_owner")){root.setIsImpliedIncluded(false);TypeUtil.addOwnedFeatureTo(nodes.get("owner"),root);var method=org.omg.sysml.adapter.FeatureAdapter.class.getDeclaredMethod("computeFeaturingType");method.setAccessible(true);method.invoke(ElementUtil.getElementAdapter(root));}
            var kinds=new TreeMap<String,String>();nodes.forEach((id,n)->kinds.put(id,n.eClass().getName()));
            cases.add(Map.of("shape",shape,"nodes",kinds,"edges",edges,"direct",root.getFeaturingType().stream().map(Type::getElementId).toList(),"all",FeatureUtil.getAllFeaturingTypesOf(root).stream().map(Type::getElementId).toList()));
        }
        Files.writeString(Path.of(args[1]),new GsonBuilder().setPrettyPrinting().create().toJson(Map.of("definitions",definitions,"cases",cases))+"\n");
    }
}
