package dev.mercurio.pilot;
import java.nio.file.*;
import java.util.*;
import org.omg.sysml.lang.sysml.*;
import com.google.gson.GsonBuilder;
public final class PilotEvaluabilityProbe {
    static Map<String,Object> observation(Expression e) {
        var r=new LinkedHashMap<String,Object>();r.put("kind",e.eClass().getName());
        r.put("attribute",e.isModelLevelEvaluable());r.put("operation",e.modelLevelEvaluable(new org.eclipse.emf.common.util.BasicEList<Feature>()));return r;
    }
    public static void main(String[] args)throws Exception {
        org.omg.sysml.logic.SysMLLogicStandaloneSetup.doSetup();
        org.eclipse.emf.ecore.EPackage.Registry.INSTANCE.put("https://www.omg.org/spec/SysML/20250201",SysMLPackage.eINSTANCE);
        var injector=new org.omg.kerml.xtext.KerMLStandaloneSetup().createInjectorAndDoEMFRegistration();var rows=new ArrayList<Object>();
        for(String value:List.of("2","2.5","true","\"text\"","*","null")) {
            String source="package P { feature f = "+value+"; }";
            var resources=injector.getInstance(org.eclipse.xtext.resource.XtextResourceSet.class);
            var resource=resources.createResource(org.eclipse.emf.common.util.URI.createURI("memory:/evaluability.kerml"));
            resource.load(new java.io.ByteArrayInputStream(source.getBytes(java.nio.charset.StandardCharsets.UTF_8)),Map.of());
            if(!resource.getErrors().isEmpty())throw new IllegalStateException("Invalid source: "+resource.getErrors());
            var iterator=resource.getAllContents();var observations=new ArrayList<Object>();
            while(iterator.hasNext()) {var item=iterator.next();if(item instanceof Expression e)observations.add(observation(e));}
            if(observations.size()!=1)throw new IllegalStateException("Missing expression");
            var row=new LinkedHashMap<String,Object>();row.put("source",source);row.put("observations",observations);rows.add(row);resource.unload();
        }
        var f=SysMLFactory.eINSTANCE;
        for(Expression e:List.of(f.createLiteralInteger(),f.createLiteralRational(),f.createLiteralBoolean(),f.createLiteralString(),f.createLiteralInfinity(),f.createNullExpression(),f.createMetadataAccessExpression())) {
            var row=new LinkedHashMap<String,Object>();row.put("shape","detached");row.put("observations",List.of(observation(e)));rows.add(row);
        }
        Files.writeString(Path.of(args[0]),new GsonBuilder().serializeNulls().setPrettyPrinting().create().toJson(rows)+"\n");
    }
}
