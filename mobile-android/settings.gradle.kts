pluginManagement {
    repositories {
        google()
        mavenCentral()
        gradlePluginPortal()
    }
}

dependencyResolutionManagement {
    repositoriesMode.set(RepositoriesMode.FAIL_ON_PROJECT_REPOS)
    repositories {
        google()
        mavenCentral()
    }
}

rootProject.name = "<app>"

include(":app")
include(":core:kernel")
include(":core:ui-core")
include(":feature:notes:contracts")
include(":feature:notes:domain")
include(":feature:notes:data")
include(":feature:notes:ui")
include(":evals")
