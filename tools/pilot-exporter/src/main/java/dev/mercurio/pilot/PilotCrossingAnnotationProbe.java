package dev.mercurio.pilot;
import java.nio.file.*;
import java.util.*;
import org.omg.sysml.lang.sysml.*;
import org.omg.sysml.util.ElementUtil;
import com.google.gson.GsonBuilder;
public final class PilotCrossingAnnotationProbe {
    static final SysMLFactory f=SysMLFactory.eINSTANCE;
    public static void main(String[] args)throws Exception {
        org.omg.sysml.logic.SysMLLogicStandaloneSetup.doSetup();
        var rows=new ArrayList<Object>();
        for(String kind:List.of("Documentation","Comment","TextualRepresentation","MetadataFeature","absent"))
            for(String ownership:List.of("owned","owning")) {
                var subject=f.createClass();subject.setDeclaredName("subject");
                var annotation=f.createAnnotation();annotation.setAnnotatedElement(subject);
                AnnotatingElement endpoint=switch(kind){
                    case "Documentation" -> f.createDocumentation();
                    case "Comment" -> f.createComment();
                    case "TextualRepresentation" -> f.createTextualRepresentation();
                    case "MetadataFeature" -> f.createMetadataFeature();
                    default -> null;
                };
                if(endpoint!=null) {
                    endpoint.setDeclaredName("endpoint");
                    if(ownership.equals("owned")) {
                        annotation.getOwnedRelatedElement().add(endpoint);
                        subject.getOwnedRelationship().add(annotation);
                    } else { endpoint.getOwnedRelationship().add(annotation); }
                } else {subject.getOwnedRelationship().add(annotation);}
                var row=new LinkedHashMap<String,Object>();
                row.put("kind",kind);row.put("ownership",ownership);
                var resolved=annotation.getAnnotatingElement();
                row.put("annotating_kind",resolved==null?null:resolved.eClass().getName());
                row.put("metadata",ElementUtil.getAllMetadataFeaturesOf(subject).stream()
                    .map(Element::getDeclaredName).toList());
                rows.add(row);
            }
        Files.writeString(Path.of(args[0]),new GsonBuilder().serializeNulls().setPrettyPrinting().create().toJson(rows)+"\n");
    }
}
