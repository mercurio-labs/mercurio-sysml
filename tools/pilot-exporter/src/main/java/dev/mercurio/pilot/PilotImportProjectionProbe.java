package dev.mercurio.pilot;
import java.nio.file.*;
import java.util.*;
import org.omg.sysml.lang.sysml.*;
import com.google.gson.GsonBuilder;
public final class PilotImportProjectionProbe {
    static String name(Element e) { return e==null?null:e.getDeclaredName(); }
    static Map<String,Object> observation(org.omg.sysml.lang.sysml.Import i) {
        var r=new LinkedHashMap<String,Object>();r.put("kind",i.eClass().getName());
        r.put("imported_element",name(i.getImportedElement()));
        if(i instanceof NamespaceImport n)r.put("stored_target",name(n.getImportedNamespace()));
        if(i instanceof MembershipImport m)r.put("stored_target",m.getImportedMembership()==null?null:m.getImportedMembership().getMemberName());
        return r;
    }
    public static void main(String[] args)throws Exception {
        org.omg.sysml.logic.SysMLLogicStandaloneSetup.doSetup();
        org.eclipse.emf.ecore.EPackage.Registry.INSTANCE.put("https://www.omg.org/spec/SysML/20250201",SysMLPackage.eINSTANCE);
        var injector=new org.omg.kerml.xtext.KerMLStandaloneSetup().createInjectorAndDoEMFRegistration();
        var rows=new ArrayList<Object>();
        for(String source:List.of(
            "package P { class A; alias a for A; } package Q { private import P::*; private import P::a; }",
            "package Q { private import P::*; private import P::a; } package P { class A; alias a for A; }",
            "package P { package N { class A; } } package Q { private import P::N::*; private import P::N::A; }",
            "package P { class A; alias aliasAlpha for aliasBeta; alias aliasBeta for A; } package Q { private import P::*; private import P::aliasAlpha; }")) {
            var resources=injector.getInstance(org.eclipse.xtext.resource.XtextResourceSet.class);
            var resource=resources.createResource(org.eclipse.emf.common.util.URI.createURI("memory:/imports.kerml"));
            resource.load(new java.io.ByteArrayInputStream(source.getBytes(java.nio.charset.StandardCharsets.UTF_8)),Map.of());
            if(!resource.getErrors().isEmpty())throw new IllegalStateException("Invalid source: "+resource.getErrors());
            var observations=new ArrayList<Object>();var iterator=resource.getAllContents();
            while(iterator.hasNext()) { var item=iterator.next();if(item instanceof org.omg.sysml.lang.sysml.Import i)observations.add(observation(i)); }
            var row=new LinkedHashMap<String,Object>();row.put("source",source);row.put("observations",observations);rows.add(row);resource.unload();
        }
        var f=SysMLFactory.eINSTANCE;var detached=new LinkedHashMap<String,Object>();detached.put("shape","missing_targets");
        detached.put("observations",List.of(observation(f.createNamespaceImport()),observation(f.createMembershipImport())));rows.add(detached);
        var member=f.createMembership();var target=f.createFeature();target.setDeclaredName("factoryTarget");member.setMemberElement(target);
        var m=f.createMembershipImport();m.setImportedMembership(member);var factory=new LinkedHashMap<String,Object>();
        factory.put("shape","detached_membership_target");factory.put("observation",observation(m));rows.add(factory);
        Files.writeString(Path.of(args[0]),new GsonBuilder().serializeNulls().setPrettyPrinting().create().toJson(rows)+"\n");
    }
}
