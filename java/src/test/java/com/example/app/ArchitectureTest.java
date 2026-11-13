package com.example.app;

import static com.tngtech.archunit.lang.syntax.ArchRuleDefinition.noClasses;

import com.tngtech.archunit.core.domain.JavaClasses;
import com.tngtech.archunit.core.importer.ClassFileImporter;
import org.junit.jupiter.api.Test;

class ArchitectureTest {
    @Test
    void domainDoesNotDependOnJakartaOrHibernate() {
        JavaClasses classes = new ClassFileImporter().importPackages("com.example.app");
        noClasses()
                .that().resideInAPackage("..widgets.domain..")
                .should().dependOnClassesThat().resideInAnyPackage("jakarta..", "org.hibernate..")
                .check(classes);
    }
}
