package dev.mercurio.pilot;
import java.nio.file.*;
import java.util.*;
import org.omg.sysml.lang.sysml.*;
import com.google.gson.GsonBuilder;
public final class PilotMultiplicityProjectionProbe {
    static Object value(Expression e) {return e==null?null:e instanceof LiteralInteger i?i.getValue():"*";}
    static Map<String,Object> observation(MultiplicityRange m) {
        var r=new LinkedHashMap<String,Object>();r.put("lower",value(m.getLowerBound()));r.put("upper",value(m.getUpperBound()));
        r.put("bounds",m.getBound().stream().map(PilotMultiplicityProjectionProbe::value).toList());return r;
    }
    public static void main(String[] args)throws Exception {
        org.omg.sysml.logic.SysMLLogicStandaloneSetup.doSetup();
        org.eclipse.emf.ecore.EPackage.Registry.INSTANCE.put("https://www.omg.org/spec/SysML/20250201",SysMLPackage.eINSTANCE);
        var injector=new org.omg.kerml.xtext.KerMLStandaloneSetup().createInjectorAndDoEMFRegistration();var rows=new ArrayList<Object>();
        for(String bound:List.of("*","2","1..3","1..*")) {
            String source="package P { feature f ["+bound+"]; }";
            var resources=injector.getInstance(org.eclipse.xtext.resource.XtextResourceSet.class);
            var resource=resources.createResource(org.eclipse.emf.common.util.URI.createURI("memory:/bounds.kerml"));
            resource.load(new java.io.ByteArrayInputStream(source.getBytes(java.nio.charset.StandardCharsets.UTF_8)),Map.of());
            if(!resource.getErrors().isEmpty())throw new IllegalStateException("Invalid source: "+resource.getErrors());
            var iterator=resource.getAllContents();var observations=new ArrayList<Object>();
            while(iterator.hasNext()) {var item=iterator.next();if(item instanceof MultiplicityRange m)observations.add(observation(m));}
            if(observations.size()!=1)throw new IllegalStateException("Missing multiplicity");
            var row=new LinkedHashMap<String,Object>();row.put("source",source);row.put("observation",observations.get(0));rows.add(row);resource.unload();
        }
        var f=SysMLFactory.eINSTANCE;
        for(int size=0;size<=3;size++) {
            var m=f.createMultiplicityRange();var other=f.createFeature();var otherMember=f.createOwningMembership();otherMember.setOwnedMemberElement(other);m.getOwnedRelationship().add(otherMember);
            for(int i=0;i<size;i++) {var e=f.createLiteralInteger();e.setValue(i+1);var member=f.createOwningMembership();member.setOwnedMemberElement(e);m.getOwnedRelationship().add(member);}
            var ignored=f.createLiteralInteger();ignored.setValue(999);var alias=f.createMembership();alias.setMemberElement(ignored);m.getOwnedRelationship().add(alias);
            var row=new LinkedHashMap<String,Object>();row.put("owned_expression_count",size);row.put("observation",observation(m));rows.add(row);
        }
        Files.writeString(Path.of(args[0]),new GsonBuilder().serializeNulls().setPrettyPrinting().create().toJson(rows)+"\n");
    }
}
