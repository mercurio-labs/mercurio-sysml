package dev.mercurio.pilot;

import com.google.gson.GsonBuilder;
import com.sun.source.tree.*;
import com.sun.source.util.*;
import javax.tools.*;
import java.nio.file.*;
import java.util.*;
import org.omg.sysml.lang.sysml.*;
import org.omg.sysml.util.*;
import static dev.mercurio.pilot.PilotFeatureRedefinitionExporter.*;

/** Shared selector with separately resolved Connector and BindingConnector dispatch. */
public final class PilotConnectorDefaultExporter {
    public static void main(String[] args) throws Exception {
        org.omg.sysml.logic.SysMLLogicStandaloneSetup.doSetup();
        var methods = new TreeMap<String,Object>();
        var compiler = ToolProvider.getSystemJavaCompiler();
        var diagnostics = new DiagnosticCollector<JavaFileObject>();
        try (var files = compiler.getStandardFileManager(diagnostics,null,java.nio.charset.StandardCharsets.UTF_8)) {
            var task = (JavacTask)compiler.getTask(null,files,diagnostics,List.of("-proc:none","-classpath",args[0]),null,files.getJavaFileObjects(args[2]));
            var units = new ArrayList<CompilationUnitTree>(); task.parse().forEach(units::add); task.analyze();
            require(diagnostics.getDiagnostics().stream().noneMatch(d -> d.getKind() == Diagnostic.Kind.ERROR), "Unresolved source: " + diagnostics.getDiagnostics());
            var trees = Trees.instance(task);
            for (var unit: units) for (var declaration: unit.getTypeDecls()) if (declaration instanceof ClassTree type)
                for (var member: type.getMembers()) if (member instanceof MethodTree method && method.getName().contentEquals("getDefaultSupertype"))
                    methods.put("selector",node(unit,method.getBody(),trees));
        }
        require(methods.size() == 1,"Missing selector");
        var factory = SysMLFactory.eINSTANCE;
        var bindings = new TreeMap<String,Object>();
        for(String binding:List.of("Connector","BindingConnector")) {
        Connector sample = binding.equals("Connector")?factory.createConnector():factory.createBindingConnector();
        var adapter = ElementUtil.getElementAdapter(sample);
        var dispatch = new TreeMap<String,String>();
        for (String name: List.of("getDefaultSupertype","computeImplicitGeneralTypes","addDefaultGeneralType","getSpecializationEClass","hasStructureType"))
            dispatch.put(name,key(method(adapter.getClass(),name)));
        var bindingMethods=new TreeMap<String,Object>(methods); bindingMethods.put("dispatch",dispatch);
        // Resolve inherited typing implementations with Pilot's own adapters;
        // recording selector inheritance alone does not establish this dispatch.
        var typingDispatch = new TreeMap<String,String>();
        typingDispatch.put("getAllTypes",key(method(adapter.getClass(),"getAllTypes")));
        for (String name: List.of("getTypes","getFeatureTypes"))
            typingDispatch.put(name,key(method(adapter.getClass(),name,List.class,Set.class)));
        bindingMethods.put("typing_dispatch",typingDispatch);
        var defaults = new TreeMap<String,String>();
        for (String name: List.of("base","binary","object","binaryObject"))
            defaults.put(name,ImplicitGeneralizationMap.getDefaultSupertypeFor(sample.getClass(),name));
        var controls = new ArrayList<Object>();
        for (int count: new int[]{0,1,2,3}) for (String typing: List.of("none","Class","Structure")) for (String owner: List.of("detached","Package","Class")) {
            Connector connector = binding.equals("Connector")?factory.createConnector():factory.createBindingConnector(); connector.setIsImpliedIncluded(true);
            if (!owner.equals("detached")) {
                Namespace namespace = owner.equals("Package") ? factory.createPackage() : factory.createClass();
                var membership = owner.equals("Package") ? factory.createOwningMembership() : factory.createFeatureMembership();
                membership.getOwnedRelatedElement().add(connector); namespace.getOwnedRelationship().add(membership);
            }
            for (int i=0;i<count;i++) {
                var end = factory.createFeature(); end.setIsEnd(true); end.setIsImpliedIncluded(true);
                var membership = factory.createFeatureMembership(); membership.setOwnedMemberFeature(end); connector.getOwnedRelationship().add(membership);
            }
            if (!typing.equals("none")) {
                var type = typing.equals("Class") ? factory.createClass() : factory.createStructure(); type.setIsImpliedIncluded(true);
                var relationship = factory.createFeatureTyping(); relationship.setTypedFeature(connector); relationship.setType(type); connector.getOwnedRelationship().add(relationship);
            }
            var actualAdapter = ElementUtil.getElementAdapter(connector);
            var selector = method(actualAdapter.getClass(),"getDefaultSupertype"); selector.setAccessible(true);
            var row = new TreeMap<String,Object>(); row.put("owned_ends",count); row.put("typing",typing); row.put("owner",owner);
            row.put("default_supertype",selector.invoke(actualAdapter)); controls.add(row);
        }
        var result = new TreeMap<String,Object>(); result.put("schema","dev.mercurio.connector-defaults.v1"); result.put("binding",binding); result.put("methods",bindingMethods); result.put("defaults",defaults); result.put("controls",controls);
        bindings.put(binding,result);
        }
        @SuppressWarnings("unchecked")
        var result = new TreeMap<String,Object>((Map<String,Object>)bindings.get("Connector"));
        result.put("binding_connector",bindings.get("BindingConnector"));
        Files.writeString(Path.of(args[1]),new GsonBuilder().serializeNulls().setPrettyPrinting().create().toJson(result)+"\n");
    }
}
