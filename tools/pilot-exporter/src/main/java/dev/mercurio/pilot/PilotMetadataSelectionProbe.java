package dev.mercurio.pilot;
import java.nio.file.*;
import java.util.*;
import org.omg.sysml.lang.sysml.*;
import org.omg.sysml.util.ElementUtil;
import com.google.gson.GsonBuilder;
public final class PilotMetadataSelectionProbe {
    static final SysMLFactory f=SysMLFactory.eINSTANCE;
    static <T extends Element> T named(T e,String name){e.setDeclaredName(name);return e;}
    static void own(Namespace owner,Element member){var m=f.createOwningMembership();m.getOwnedRelatedElement().add(member);owner.getOwnedRelationship().add(m);}
    static Annotation annotate(Element owner,Element target,AnnotatingElement value){var a=f.createAnnotation();a.setAnnotatedElement(target);a.getOwnedRelatedElement().add(value);owner.getOwnedRelationship().add(a);return a;}
    static List<String> names(Collection<? extends Element> values){return values.stream().map(Element::getDeclaredName).toList();}
    public static void main(String[] args)throws Exception {
        org.omg.sysml.logic.SysMLLogicStandaloneSetup.doSetup();
        var rows=new ArrayList<Object>();
        for(String shape:List.of("empty","documentation","unrelated","annotation_owned","annotation_external","member_implicit","member_other","alias","ordered")) {
            Type owner=named(f.createClass(),"subject");var other=named(f.createPackage(),"other");
            switch(shape){
                case "documentation" -> annotate(owner,owner,named(f.createDocumentation(),"doc"));
                case "unrelated" -> own(other,named(f.createMetadataFeature(),"unrelated"));
                case "annotation_owned" -> annotate(owner,owner,named(f.createMetadataFeature(),"applied"));
                case "annotation_external" -> {var meta=named(f.createMetadataFeature(),"external");var a=f.createAnnotation();a.setAnnotatedElement(owner);meta.getOwnedRelationship().add(a);own(other,meta);}
                case "member_implicit" -> own(owner,named(f.createMetadataFeature(),"member"));
                case "member_other" -> {var meta=named(f.createMetadataFeature(),"member");own(owner,meta);var a=f.createAnnotation();a.setAnnotatedElement(other);meta.getOwnedRelationship().add(a);}
                case "alias" -> {var meta=named(f.createMetadataFeature(),"alias");own(other,meta);var m=f.createMembership();m.setMemberElement(meta);owner.getOwnedRelationship().add(m);}
                case "ordered" -> {annotate(owner,owner,named(f.createMetadataFeature(),"first"));annotate(owner,owner,named(f.createMetadataFeature(),"second"));own(owner,named(f.createMetadataFeature(),"third"));}
                default -> {}
            }
            var row=new LinkedHashMap<String,Object>();row.put("shape",shape);
            row.put("metadata",names(ElementUtil.getAllMetadataFeaturesOf(owner)));
            var annotations=new ArrayList<Object>();for(var a:owner.getOwnedAnnotation()){
                var entry=new LinkedHashMap<String,Object>();entry.put("target",a.getAnnotatedElement().getDeclaredName());
                entry.put("annotating",a.getAnnotatingElement()==null?null:a.getAnnotatingElement().getDeclaredName());annotations.add(entry);
            }
            row.put("owned_annotations",annotations);rows.add(row);
        }
        Files.writeString(Path.of(args[0]),new GsonBuilder().serializeNulls().setPrettyPrinting().create().toJson(rows)+"\n");
    }
}
