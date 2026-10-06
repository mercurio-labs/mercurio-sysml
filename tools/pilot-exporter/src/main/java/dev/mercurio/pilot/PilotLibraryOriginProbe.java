package dev.mercurio.pilot;
import com.google.gson.GsonBuilder;
import java.nio.file.*;
import java.util.*;
import org.omg.sysml.lang.sysml.*;
public final class PilotLibraryOriginProbe {
    static Map<String,Object> observation(String name, Element element) {
        var row=new LinkedHashMap<String,Object>();
        var library=element.libraryNamespace();
        row.put("name",name);row.put("kind",element.eClass().getName());
        row.put("library",library==null?null:library.getDeclaredName());
        row.put("is_library_element",element.isLibraryElement());
        if(element instanceof Type type) {
            var conjugator=type.getOwnedConjugator();
            row.put("is_conjugated",type.isConjugated());
            row.put("conjugator_original",conjugator==null?null:conjugator.getOriginalType().getDeclaredName());
        }
        var relation=element.getOwningRelationship();
        var relationshipLibrary=relation==null?null:relation.libraryNamespace();
        row.put("owning_relationship_library",relationshipLibrary==null?null:relationshipLibrary.getDeclaredName());
        return row;
    }
    public static void main(String[] args) throws Exception {
        org.omg.sysml.logic.SysMLLogicStandaloneSetup.doSetup();
        org.eclipse.emf.ecore.EPackage.Registry.INSTANCE.put("https://www.omg.org/spec/SysML/20250201",SysMLPackage.eINSTANCE);
        var injector=new org.omg.kerml.xtext.KerMLStandaloneSetup().createInjectorAndDoEMFRegistration();
        var cases=new ArrayList<Object>();
        for(String source:List.of(
            "package Outside { package Nested { feature f; } }",
            "library package Library { package Nested { feature f; } } package Outside { feature g; }",
            "standard library package Standard { library package Inner { package Nested { feature f; } } feature g; }",
            "library package Library { feature original; } package Outside { alias borrowed for Library::original; }",
            "package Context { classifier Original; classifier Plain; classifier Conjugated conjugates Original; feature direct; feature reverse conjugates direct; }")) {
            var resources=injector.getInstance(org.eclipse.xtext.resource.XtextResourceSet.class);
            var resource=resources.createResource(org.eclipse.emf.common.util.URI.createURI("memory:/library-origin.kerml"));
            resource.load(new java.io.ByteArrayInputStream(source.getBytes(java.nio.charset.StandardCharsets.UTF_8)),Map.of());
            if(!resource.getErrors().isEmpty())throw new IllegalStateException("Invalid source: "+resource.getErrors());
            var observations=new ArrayList<Object>();var iterator=resource.getAllContents();
            while(iterator.hasNext()) {
                var item=iterator.next();
                if(item instanceof Element element && element.getDeclaredName()!=null)
                    observations.add(observation(element.getDeclaredName(),element));
            }
            var row=new LinkedHashMap<String,Object>();row.put("source",source);row.put("observations",observations);
            // The referencing membership's origin belongs to its owner, even
            // when the target is in a library. It must not inherit target origin.
            if(source.contains("borrowed")) {
                var all=resource.getAllContents();
                while(all.hasNext())if(all.next() instanceof Membership m && "borrowed".equals(m.getMemberName()))
                    row.put("alias",observation("borrowed",m));
                if(!row.containsKey("alias"))throw new IllegalStateException("Missing alias control");
            }
            cases.add(row);resource.unload();
        }
        // Explicitly exercise the Relationship fallback with a relationship
        // contained by a relationship, rather than an owningRelatedElement.
        var factory=SysMLFactory.eINSTANCE;var library=factory.createLibraryPackage();library.setDeclaredName("DetachedLibrary");
        var membership=factory.createOwningMembership();library.getOwnedRelationship().add(membership);
        var child=factory.createSpecialization();membership.getOwnedRelatedElement().add(child);
        var factoryRow=new LinkedHashMap<String,Object>();factoryRow.put("shape","relationship_fallback");
        factoryRow.put("library",child.libraryNamespace()==null?null:child.libraryNamespace().getDeclaredName());
        factoryRow.put("is_library_element",child.isLibraryElement());
        cases.add(factoryRow);
        Files.writeString(Path.of(args[0]),new GsonBuilder().serializeNulls().setPrettyPrinting().create().toJson(cases)+"\n");
    }
}
