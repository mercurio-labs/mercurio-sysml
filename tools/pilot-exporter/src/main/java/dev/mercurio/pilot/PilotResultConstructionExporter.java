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

/** Build-time resolved constructor definition and independent adapter observations. */
public final class PilotResultConstructionExporter {
    static Object tree(CompilationUnitTree unit, Tree value, Trees trees) {
        var row=new TreeMap<String,Object>(); row.put("kind",value.getKind().name());
        var path=TreePath.getPath(unit,value); var symbol=trees.getElement(path); var type=trees.getTypeMirror(path);
        if(symbol!=null) row.put("symbol",symbol.getEnclosingElement()+"#"+symbol);
        if(type!=null) row.put("type",type.toString());
        if(value instanceof LiteralTree literal) row.put("value",literal.getValue());
        var children=new ArrayList<Object>();
        value.accept(new TreeScanner<Void,Void>() {
            @Override public Void scan(Tree child,Void unused) { if(child!=null) children.add(tree(unit,child,trees)); return null; }
        },null);
        row.put("children",children); return row;
    }
    static java.lang.reflect.Method method(java.lang.Class<?> type) throws Exception {
        while(type!=null) { try { return type.getDeclaredMethod("addAdditionalMembers"); } catch(NoSuchMethodException e) { type=type.getSuperclass(); } }
        throw new IllegalStateException("Missing member provider");
    }
    public static void main(String[] args) throws Exception {
        org.omg.sysml.logic.SysMLLogicStandaloneSetup.doSetup();
        var definitions=new TreeMap<String,Object>();
        var compiler=ToolProvider.getSystemJavaCompiler(); var diagnostics=new DiagnosticCollector<JavaFileObject>();
        try(var files=compiler.getStandardFileManager(diagnostics,null,null)) {
            var task=(JavacTask)compiler.getTask(null,files,diagnostics,List.of("-proc:none","-classpath",args[0]),null,files.getJavaFileObjects(Arrays.copyOfRange(args,2,args.length)));
            var units=new ArrayList<CompilationUnitTree>(); task.parse().forEach(units::add); task.analyze();
            if(diagnostics.getDiagnostics().stream().anyMatch(d->d.getKind()==Diagnostic.Kind.ERROR)) throw new IllegalStateException(diagnostics.getDiagnostics().toString());
            var trees=Trees.instance(task);
            for(var unit:units) for(var decl:unit.getTypeDecls()) if(decl instanceof ClassTree cls)
                for(var member:cls.getMembers()) if(member instanceof MethodTree m && (m.getName().contentEquals("addResultParameterTo") || m.getName().contentEquals("addAdditionalMembers")))
                    definitions.put(cls.getSimpleName()+"#"+m.getName()+"/"+m.getParameters().size(),tree(unit,m.getBody(),trees));
        }
        var excluded=new TreeMap<String,String>();var bindings=new TreeMap<String,String>();var cases=new ArrayList<Object>();
        for(var classifier:SysMLPackage.eINSTANCE.getEClassifiers()) {
            if(!(classifier instanceof EClass cls) || cls.isAbstract() || !SysMLPackage.eINSTANCE.getExpression().isSuperTypeOf(cls)) continue;
            var probe=(Expression)SysMLFactory.eINSTANCE.create(cls); var resolved=method(ElementUtil.getElementAdapter(probe).getClass());
            String provider=resolved.getDeclaringClass().getSimpleName();
            if(!List.of("InvocationExpressionAdapter","FeatureReferenceExpressionAdapter").contains(provider)) { excluded.put(cls.getName(),provider+"#addAdditionalMembers/0");continue; }
            bindings.put(cls.getName(),provider+"#addAdditionalMembers/0");
            for(String context:List.of("empty","ordinary_member","existing_return","empty_return")) {
                var owner=(Expression)SysMLFactory.eINSTANCE.create(cls);
                if(!context.equals("empty")) {
                    FeatureMembership member=context.equals("ordinary_member")?SysMLFactory.eINSTANCE.createFeatureMembership():SysMLFactory.eINSTANCE.createReturnParameterMembership();
                    if(!context.equals("empty_return")) { var feature=SysMLFactory.eINSTANCE.createFeature();feature.setDirection(FeatureDirectionKind.IN);member.setOwnedMemberFeature(feature); }
                    owner.getOwnedRelationship().add(member);
                }
                var adapter=ElementUtil.getElementAdapter(owner);resolved.invoke(adapter);
                Object first=PilotOperandProbe.node(owner);resolved.invoke(adapter);Object second=PilotOperandProbe.node(owner);
                cases.add(Map.of("class",cls.getName(),"context",context,"tree",first,"idempotent",first.equals(second)));
            }
        }
        Files.writeString(Path.of(args[1]),new GsonBuilder().setPrettyPrinting().create().toJson(Map.of("definitions",definitions,"bindings",bindings,"excluded_bindings",excluded,"cases",cases))+"\n");
    }
}
