package dev.mercurio.pilot;

import com.google.gson.GsonBuilder;
import java.nio.file.*;
import java.util.*;
import org.eclipse.emf.common.util.URI;
import org.eclipse.emf.ecore.*;
import org.eclipse.emf.ecore.resource.impl.ResourceSetImpl;
import org.eclipse.emf.ecore.util.EcoreUtil;
import org.eclipse.emf.ecore.xmi.impl.EcoreResourceFactoryImpl;
import org.omg.sysml.delegate.invocation.OperationInvocationDelegateSelector;

/** Build-time upstream selection, not a reconstruction of Java/Xtend dispatch. */
public final class PilotNameDelegateExporter {
    static final class Selector extends OperationInvocationDelegateSelector {
        Selector(EOperation op) { super(op); }
        String selected(EClass type) {
            var delegate = calculateInvocationDelegateRecursive(type);
            if(delegate == null) throw new IllegalStateException("Missing name delegate: " + type.getName());
            return delegate.getClass().getName();
        }
    }
    public static void main(String[] args) throws Exception {
        var set = new ResourceSetImpl();
        set.getResourceFactoryRegistry().getExtensionToFactoryMap().put("ecore", new EcoreResourceFactoryImpl());
        set.getPackageRegistry().put(EcorePackage.eNS_URI, EcorePackage.eINSTANCE);
        var resource = set.getResource(URI.createFileURI(Path.of(args[0]).toAbsolutePath().toString()), true);
        EcoreUtil.resolveAll(set);
        if(!EcoreUtil.UnresolvedProxyCrossReferencer.find(set).isEmpty() || !resource.getErrors().isEmpty()) throw new IllegalStateException("Unresolved Ecore");
        var dispatch = new ArrayList<Object>();
        var classes = new TreeSet<String>();
        var objects = resource.getAllContents();
        while(objects.hasNext()) {
            if(!(objects.next() instanceof EClass type)) continue;
            classes.add(type.getName());
            for(String name : List.of("effectiveName", "effectiveShortName")) {
                var candidates = type.getEAllOperations().stream().filter(op -> op.getName().equals(name) && op.getEParameters().isEmpty()).toList();
                if(candidates.size()!=1) throw new IllegalStateException("Ambiguous name operation: " + type.getName());
                var op = candidates.get(0);
                dispatch.add(Map.of("class",type.getName(),"operation",name,
                    "operation_owner",op.getEContainingClass().getName(),"delegate",new Selector(op).selected(type)));
            }
        }
        org.omg.sysml.logic.SysMLLogicStandaloneSetup.doSetup();
        var observations = new ArrayList<Object>();
        for(String kind : List.of("Package","PartDefinition","PartUsage","ConjugatedPortDefinition")) {
            for(int mode=0;mode<3;mode++) {
                var type = (EClass)org.omg.sysml.lang.sysml.SysMLPackage.eINSTANCE.getEClassifier(kind);
                if(type == null) throw new IllegalStateException("Missing runtime classifier: " + kind);
                var element = (org.omg.sysml.lang.sysml.Element)org.omg.sysml.lang.sysml.SysMLFactory.eINSTANCE.create(type);
                if(mode==1) element.setDeclaredName("Long");
                if(mode==2) element.setDeclaredShortName("Short");
                var row = new TreeMap<String,Object>();
                row.put("class",kind); row.put("declared_name",element.getDeclaredName()); row.put("declared_short_name",element.getDeclaredShortName());
                row.put("effective_name",element.effectiveName()); row.put("effective_short_name",element.effectiveShortName());
                observations.add(row);
            }
        }
        for(int mode=0;mode<4;mode++) {
            var factory = org.omg.sysml.lang.sysml.SysMLFactory.eINSTANCE;
            var original = factory.createPortDefinition();
            if(mode==1 || mode==3) original.setDeclaredName("Original");
            if(mode==2 || mode==3) original.setDeclaredShortName("O");
            var conjugated = factory.createConjugatedPortDefinition();
            conjugated.setDeclaredName("Ignored");
            conjugated.setDeclaredShortName("C");
            var conjugation = factory.createPortConjugation();
            conjugation.setOriginalPortDefinition(original);
            conjugated.getOwnedRelationship().add(conjugation);
            var row = new TreeMap<String,Object>();
            row.put("class","ConjugatedPortDefinition");
            row.put("declared_name",conjugated.getDeclaredName());
            row.put("declared_short_name",conjugated.getDeclaredShortName());
            row.put("original_port",true);
            row.put("original_declared_name",original.getDeclaredName());
            row.put("original_declared_short_name",original.getDeclaredShortName());
            row.put("effective_name",conjugated.effectiveName());
            row.put("effective_short_name",conjugated.effectiveShortName());
            observations.add(row);
        }
        var redefinitionObservations = new ArrayList<Object>();
        for(String kind : List.of("Feature", "PartUsage", "AttributeUsage")) {
            for(int mode=0;mode<4;mode++) {
                var factory = org.omg.sysml.lang.sysml.SysMLFactory.eINSTANCE;
                var type = (EClass)org.omg.sysml.lang.sysml.SysMLPackage.eINSTANCE.getEClassifier(kind);
                var first = (org.omg.sysml.lang.sysml.Feature)factory.create(type);
                if(mode!=1) first.setDeclaredName("First");
                if(mode!=0) first.setDeclaredShortName("F");
                var second = (org.omg.sysml.lang.sysml.Feature)factory.create(type);
                second.setDeclaredName("Second"); second.setDeclaredShortName("S");
                var middle = (org.omg.sysml.lang.sysml.Feature)factory.create(type);
                var root = (org.omg.sysml.lang.sysml.Feature)factory.create(type);
                if(mode==3) root.setDeclaredShortName("Own");
                var rootRedefinition = factory.createRedefinition();
                rootRedefinition.setRedefinedFeature(middle);
                rootRedefinition.setRedefiningFeature(root);
                root.getOwnedRelationship().add(rootRedefinition);
                for(var target : List.of(first,second)) {
                    var redefinition = factory.createRedefinition();
                    redefinition.setRedefinedFeature(target);
                    redefinition.setRedefiningFeature(middle);
                    middle.getOwnedRelationship().add(redefinition);
                }
                var ids = new IdentityHashMap<org.omg.sysml.lang.sysml.Feature,String>();
                ids.put(root,"root"); ids.put(middle,"middle"); ids.put(first,"first"); ids.put(second,"second");
                var nodes = new ArrayList<Object>();
                for(var node : List.of(root,middle,first,second)) {
                    var stored = new TreeMap<String,Object>();
                    stored.put("id",ids.get(node)); stored.put("class",kind);
                    stored.put("declared_name",node.getDeclaredName());
                    stored.put("declared_short_name",node.getDeclaredShortName());
                    stored.put("redefines",node.getOwnedRedefinition().stream().map(r->ids.get(r.getRedefinedFeature())).toList());
                    nodes.add(stored);
                }
                var row = new TreeMap<String,Object>();
                row.put("nodes",nodes); row.put("root","root");
                row.put("effective_name",root.effectiveName());
                row.put("effective_short_name",root.effectiveShortName());
                redefinitionObservations.add(row);
            }
        }
        var result = Map.of("schema","dev.mercurio.name-delegate-dispatch.v1","classes",classes,
            "dispatch",dispatch,"observations",observations,"redefinition_observations",redefinitionObservations,
            "meaning","Selected upstream delegates; algorithm support is separately assessed");
        Files.writeString(Path.of(args[1]), new GsonBuilder().serializeNulls().setPrettyPrinting().create().toJson(result)+"\n");
    }
}
