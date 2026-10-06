package dev.mercurio.pilot;
import java.nio.file.*;
import java.util.*;
import org.omg.sysml.lang.sysml.*;
import com.google.gson.GsonBuilder;
public final class PilotOwnershipProjectionProbe {
    static String name(Element e) {return e==null?null:e.getDeclaredName();}
    static Map<String,Object> annotated(AnnotatingElement e) {
        var row=new LinkedHashMap<String,Object>();row.put("name",name(e));row.put("kind",e.eClass().getName());
        row.put("targets",e.getAnnotatedElement().stream().map(PilotOwnershipProjectionProbe::name).toList());
        row.put("annotation_count",e.getAnnotation().size());
        if(e instanceof Documentation d)row.put("documented",name(d.getDocumentedElement()));
        return row;
    }
    public static void main(String[] args)throws Exception {
        org.omg.sysml.logic.SysMLLogicStandaloneSetup.doSetup();
        org.eclipse.emf.ecore.EPackage.Registry.INSTANCE.put("https://www.omg.org/spec/SysML/20250201",SysMLPackage.eINSTANCE);
        var injector=new org.omg.kerml.xtext.KerMLStandaloneSetup().createInjectorAndDoEMFRegistration();
        var rows=new ArrayList<Object>();
        for(String source:List.of(
            "package P { doc note /*package*/ feature f { doc nested /*feature*/ } }",
            "package P { classifier A; classifier B; comment c about B,A /*ordered*/ comment fallback /*owner*/ }",
            "package P { feature base { feature leaf; } feature chain chains base.leaf; }",
            "package P { feature a; feature b; comment c about a,a,b /*unique*/ }")) {
            var resources=injector.getInstance(org.eclipse.xtext.resource.XtextResourceSet.class);
            var resource=resources.createResource(org.eclipse.emf.common.util.URI.createURI("memory:/projections.kerml"));
            resource.load(new java.io.ByteArrayInputStream(source.getBytes(java.nio.charset.StandardCharsets.UTF_8)),Map.of());
            if(!resource.getErrors().isEmpty())throw new IllegalStateException("Invalid source: "+resource.getErrors());
            var observations=new ArrayList<Object>();var iterator=resource.getAllContents();var ordinals=new HashMap<String,Integer>();
            while(iterator.hasNext()) {
                var item=iterator.next();
                if(item instanceof AnnotatingElement e) observations.add(annotated(e));
                else if(item instanceof FeatureChaining r) {
                    String owner=name(r.getOwningRelatedElement());var row=new LinkedHashMap<String,Object>();
                    row.put("kind","FeatureChaining");row.put("owner",owner);row.put("ordinal",ordinals.merge(owner,1,Integer::sum)-1);
                    row.put("feature_chained",name(r.getFeatureChained()));observations.add(row);
                }
            }
            var row=new LinkedHashMap<String,Object>();row.put("source",source);row.put("observations",observations);rows.add(row);resource.unload();
        }
        var f=SysMLFactory.eINSTANCE;var p=f.createPackage();p.setDeclaredName("P");var target=f.createFeature();target.setDeclaredName("target");
        var comment=f.createComment();comment.setDeclaredName("mixed");var outer=f.createAnnotation();outer.setAnnotatedElement(p);
        p.getOwnedRelationship().add(outer);outer.getOwnedRelatedElement().add(comment);
        var owned=f.createAnnotation();owned.setAnnotatedElement(target);comment.getOwnedRelationship().add(owned);
        var row=new LinkedHashMap<String,Object>();row.put("shape","owning_then_owned");row.put("observation",annotated(comment));rows.add(row);
        var detached=f.createComment();detached.setDeclaredName("detached");var negative=new LinkedHashMap<String,Object>();
        negative.put("shape","detached");negative.put("observation",annotated(detached));
        var chaining=f.createFeatureChaining();p.getOwnedRelationship().add(chaining);negative.put("wrong_container_feature_chained",name(chaining.getFeatureChained()));rows.add(negative);
        Files.writeString(Path.of(args[0]),new GsonBuilder().serializeNulls().setPrettyPrinting().create().toJson(rows)+"\n");
    }
}
