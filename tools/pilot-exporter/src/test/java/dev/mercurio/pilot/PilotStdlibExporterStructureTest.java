package dev.mercurio.pilot;

import java.lang.reflect.InvocationTargetException;
import java.lang.reflect.Method;
import java.nio.file.Path;
import org.eclipse.emf.common.util.URI;
import org.eclipse.emf.ecore.resource.Resource;
import org.eclipse.emf.ecore.resource.ResourceSet;
import org.eclipse.emf.ecore.resource.impl.ResourceImpl;
import org.eclipse.emf.ecore.resource.impl.ResourceSetImpl;
import org.omg.sysml.lang.sysml.Namespace;
import org.omg.sysml.lang.sysml.OwningMembership;
import org.omg.sysml.lang.sysml.SysMLFactory;

/** Standalone structural regression controls; no full Pilot library load. */
public final class PilotStdlibExporterStructureTest {
    private static final Path ROOT = Path.of("synthetic-pilot-library").toAbsolutePath().normalize();
    private static final SysMLFactory FACTORY = initializedFactory();

    private static SysMLFactory initializedFactory() {
        org.omg.sysml.interactive.SysMLInteractive.getInstance();
        return SysMLFactory.eINSTANCE;
    }
    private static final Method EXPORT;

    static {
        try {
            EXPORT = PilotStdlibExporter.class.getDeclaredMethod("exportDocument", Path.class, ResourceSet.class);
            EXPORT.setAccessible(true);
        } catch (ReflectiveOperationException error) {
            throw new ExceptionInInitializerError(error);
        }
    }

    public static void main(String[] args) throws Exception {
        expectRejected(new ResourceSetImpl(), "no library resources");
        ResourceSet emptyRoots = new ResourceSetImpl();
        addResource(emptyRoots, "One.kerml").getContents().add(FACTORY.createNamespace());
        addResource(emptyRoots, "Two.kerml").getContents().add(FACTORY.createNamespace());
        expectRejected(emptyRoots, "empty anonymous resource roots");

        ResourceSet disconnected = new ResourceSetImpl();
        org.omg.sysml.lang.sysml.Package named = FACTORY.createPackage();
        named.setDeclaredName("Disconnected");
        addResource(disconnected, "Disconnected.kerml").getContents().add(named);
        expectRejected(disconnected, "named content without retained relationships");

        ResourceSet minimal = new ResourceSetImpl();
        Namespace root = FACTORY.createNamespace();
        addResource(minimal, "Minimal.kerml").getContents().add(root);
        org.omg.sysml.lang.sysml.Package declaration = FACTORY.createPackage();
        declaration.setDeclaredName("Minimal");
        OwningMembership membership = FACTORY.createOwningMembership();
        root.getOwnedRelationship().add(membership);
        membership.getOwnedRelatedElement().add(declaration);
        EXPORT.invoke(null, ROOT, minimal);
        System.out.println("Passed 4 structural export controls: empty set, empty roots, disconnected content, minimal linked content");
    }

    private static Resource addResource(ResourceSet resources, String name) {
        Resource resource = new ResourceImpl(URI.createFileURI(ROOT.resolve("Kernel Libraries").resolve(name).toString()));
        resources.getResources().add(resource);
        return resource;
    }

    private static void expectRejected(ResourceSet resources, String label) throws Exception {
        try {
            EXPORT.invoke(null, ROOT, resources);
            throw new AssertionError("Accepted incomplete export: " + label);
        } catch (InvocationTargetException error) {
            Throwable cause = error.getCause();
            if (!(cause instanceof IllegalStateException)
                || !cause.getMessage().startsWith("Incomplete Pilot library export:")) {
                throw new AssertionError("Unexpected failure for " + label, cause);
            }
        }
    }
}
