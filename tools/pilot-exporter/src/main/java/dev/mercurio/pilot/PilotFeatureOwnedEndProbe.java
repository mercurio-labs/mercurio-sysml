package dev.mercurio.pilot;
import com.google.gson.GsonBuilder;
import java.nio.file.*;
import java.util.*;
import org.omg.sysml.lang.sysml.*;
public final class PilotFeatureOwnedEndProbe {
    static java.lang.reflect.Method method(java.lang.Class<?> type,String name) throws Exception {
        for (;type!=null;type=type.getSuperclass()) {
            try {var result=type.getDeclaredMethod(name);result.setAccessible(true);return result;}
            catch(NoSuchMethodException absent) { }
        }
        throw new NoSuchMethodException(name);
    }
    static List<String> path(Element element) {
        var names=new ArrayList<String>();
        for (Element cursor=element;cursor!=null;cursor=org.omg.sysml.util.NamespaceUtil.getParentNamespaceOf(cursor))
            if (cursor.getDeclaredName()!=null) names.add(org.omg.sysml.util.ElementUtil.unescapeString(cursor.getDeclaredName()));
        Collections.reverse(names);return names;
    }
    public static void main(String[] args) throws Exception {
        org.omg.sysml.logic.SysMLLogicStandaloneSetup.doSetup();
        org.eclipse.emf.ecore.EPackage.Registry.INSTANCE.put("https://www.omg.org/spec/SysML/20250201",SysMLPackage.eINSTANCE);
        var injector=new org.omg.kerml.xtext.KerMLStandaloneSetup().createInjectorAndDoEMFRegistration();
        String library="standard library package Base { classifier Anything; feature things; } standard library package Links { assoc Link specializes Base::Anything { feature participant; } assoc BinaryLink specializes Link { end feature source; end feature target; } }";
        var rows=new ArrayList<Object>();
        for (String shape:List.of("direct","subsetted_owner","extra_end","explicit")) for (boolean reversed:List.of(false,true)) {
            String ends=shape.equals("explicit") ? "end feature leftEnd redefines Links::BinaryLink::source; end feature rightEnd;" : "end feature leftEnd; end feature rightEnd;";
            if(shape.equals("extra_end")) ends+="end feature extraEnd;";
            String context=shape.equals("subsetted_owner") ? "feature base : Links::BinaryLink; feature outer subsets base {"+ends+"}" : "feature outer : Links::BinaryLink {"+ends+"}";
            String model="package P {"+context+"}";
            String source=reversed ? model+library : library+model;
            var resources=injector.getInstance(org.eclipse.xtext.resource.XtextResourceSet.class);
            var resource=resources.createResource(org.eclipse.emf.common.util.URI.createURI("memory:/feature-owned-end.kerml"));
            resource.load(new java.io.ByteArrayInputStream(source.getBytes(java.nio.charset.StandardCharsets.UTF_8)),Map.of());
            if(!resource.getErrors().isEmpty()) throw new IllegalStateException("Invalid source: "+resource.getErrors());
            var observations=new ArrayList<Object>();var iterator=resource.getAllContents();
            while(iterator.hasNext()) {
                var item=iterator.next();
                if(item instanceof Feature feature && List.of("leftEnd","rightEnd","extraEnd").contains(feature.getDeclaredName())) {
                    var adapter=org.omg.sysml.util.ElementUtil.getElementAdapter(feature);
                    var generals=org.omg.sysml.util.TypeUtil.getGeneralTypesOf(feature);
                    if(generals.stream().anyMatch(type->type==null || type.eIsProxy())) throw new IllegalStateException("Unresolved general");
                    observations.add(Map.of("feature",feature.getDeclaredName(),"generals",generals.stream().map(PilotFeatureOwnedEndProbe::path).toList(),"default",method(adapter.getClass(),"getDefaultSupertype").invoke(adapter)));
                }
            }
            if(observations.size()!=(shape.equals("extra_end") ? 3 : 2) || !resource.getErrors().isEmpty()) throw new IllegalStateException("Incomplete control");
            rows.add(Map.of("shape",shape,"reversed",reversed,"source",source,"observations",observations));
            resource.unload();
        }
        Files.writeString(Path.of(args[0]),new GsonBuilder().setPrettyPrinting().create().toJson(rows)+"\n");
    }
}
