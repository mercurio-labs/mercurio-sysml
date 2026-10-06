package dev.mercurio.pilot;
import java.nio.file.*;
import java.util.*;
import org.eclipse.emf.ecore.EObject;
import org.eclipse.xtext.nodemodel.util.NodeModelUtils;
import org.omg.sysml.lang.sysml.*;
import com.google.gson.GsonBuilder;
public final class PilotLibraryReferenceInventory {
    static List<String> packagePath(EObject item) {
        var path=new ArrayList<String>();
        for(EObject e=item;e!=null;e=e.eContainer()) if(e instanceof org.omg.sysml.lang.sysml.Package p && p.getDeclaredName()!=null) path.add(p.getDeclaredName());
        Collections.reverse(path);return path;
    }
    public static void main(String[] args)throws Exception {
        org.omg.sysml.logic.SysMLLogicStandaloneSetup.doSetup();
        org.eclipse.emf.ecore.EPackage.Registry.INSTANCE.put("https://www.omg.org/spec/SysML/20250201",SysMLPackage.eINSTANCE);
        var kerml=new org.omg.kerml.xtext.KerMLStandaloneSetup().createInjectorAndDoEMFRegistration();
        var sysml=new org.omg.sysml.xtext.SysMLStandaloneSetup().createInjectorAndDoEMFRegistration();var rows=new ArrayList<Object>();
        for(String filename:Files.readAllLines(Path.of(args[0]))) {
            var injector=filename.endsWith(".kerml")?kerml:sysml;
            var resource=injector.getInstance(org.eclipse.xtext.resource.XtextResourceSet.class).createResource(org.eclipse.emf.common.util.URI.createFileURI(filename));
            try(var input=Files.newInputStream(Path.of(filename))) { resource.load(input,Map.of()); }
            if(!resource.getErrors().isEmpty())throw new IllegalStateException(filename+": "+resource.getErrors());
            var packages=new ArrayList<Object>();var imports=new ArrayList<Object>();var references=new ArrayList<Object>();var iterator=resource.getAllContents();
            var converter=injector.getInstance(org.eclipse.xtext.naming.IQualifiedNameConverter.class);
            while(iterator.hasNext()) {
                var e=iterator.next();
                if(e instanceof org.omg.sysml.lang.sysml.Package p && p.getDeclaredName()!=null) packages.add(packagePath(e));
                // Read concrete cross-reference syntax without invoking derived
                // getters or lazy linking. Ecore supplies receiver/target types;
                // the upstream converter supplies exact qualified-name segments.
                for(var reference:e.eClass().getEAllReferences()) {
                    if(reference.isContainment() || reference.isContainer()) continue;
                    for(var node:NodeModelUtils.findNodesForFeature(e,reference)) {
                        var item=new LinkedHashMap<String,Object>();
                        item.put("kind",e.eClass().getName());item.put("feature",reference.getName());
                        item.put("target_type",reference.getEReferenceType().getName());
                        item.put("owner_package",packagePath(e));item.put("offset",node.getOffset());
                        item.put("target_segments",converter.toQualifiedName(NodeModelUtils.getTokenText(node)).getSegments());
                        references.add(item);
                    }
                }
                if(e instanceof org.omg.sysml.lang.sysml.Import i) {
                    String field=i instanceof NamespaceImport?"importedNamespace":"importedMembership";
                    var feature=i.eClass().getEStructuralFeature(field);
                    var nodes=NodeModelUtils.findNodesForFeature(i,feature);var row=new LinkedHashMap<String,Object>();
                    row.put("kind",i.eClass().getName());row.put("owner_package",packagePath(i));row.put("feature",field);
                    if(nodes.size()>1)throw new IllegalStateException("Ambiguous import syntax");
                    if(nodes.isEmpty()) {row.put("target_segments",List.of());row.put("implicit_filter_target",true);}
                    else {row.put("target_segments",converter.toQualifiedName(NodeModelUtils.getTokenText(nodes.get(0))).getSegments());row.put("offset",nodes.get(0).getOffset());}
                    imports.add(row);
                }
            }
            var row=new LinkedHashMap<String,Object>();row.put("path",filename);row.put("packages",packages);row.put("imports",imports);row.put("references",references);rows.add(row);resource.unload();
        }
        Files.writeString(Path.of(args[1]),new GsonBuilder().setPrettyPrinting().create().toJson(rows)+"\n");
    }
}
