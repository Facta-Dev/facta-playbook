package dev.<scope>.<app>.evals

import java.io.File
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonArray
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * The contract tests for the `.claude` layer. Pure JVM: the layer is text and
 * needs no device. The leak detector assembles its tokens at runtime so the
 * rename pass cannot rewrite the detector itself.
 */
class KitContractTest {
    private val root = File(System.getProperty("kit.root") ?: ".").absoluteFile

    private val divisions = setOf("client", "security", "testing", "core")
    private val agentSections = listOf(
        "Core Mission", "Expertise Areas", "Output Format", "Quality Standards", "Guiding Principles",
    )
    private val skillSections = listOf(
        "When to Activate", "Core Principles", "Rules", "Research References", "Output Format",
    )

    private val tokens = listOf(
        listOf("<", "scope", ">").joinToString(""),
        listOf("<", "APP", ">").joinToString(""),
        listOf("<", "app", ">").joinToString(""),
    )

    private fun file(path: String) = File(root, path)

    private fun parse(path: String): Triple<Map<String, String>, String, String> =
        parseText(file(path).readText(), path)

    private fun parseText(text: String, path: String): Triple<Map<String, String>, String, String> {
        val lines = text.lines()
        require(lines.firstOrNull()?.trim() == "---") { "$path: missing frontmatter opening ---" }
        val closing = (1 until lines.size).firstOrNull { lines[it].trim() == "---" }
        requireNotNull(closing) { "$path: missing frontmatter closing ---" }
        val frontmatter = LinkedHashMap<String, String>()
        for (index in 1 until closing) {
            val line = lines[index]
            val colon = line.indexOf(':')
            if (colon > 0) frontmatter[line.substring(0, colon).trim()] = line.substring(colon + 1).trim().trim('"')
        }
        return Triple(frontmatter, lines.drop(closing + 1).joinToString("\n"), text)
    }

    private fun required(frontmatter: Map<String, String>, vararg keys: String): List<String> =
        keys.filter { frontmatter[it].isNullOrBlank() }.map { "missing frontmatter key: $it" }

    private fun wordCount(body: String) = body.split(Regex("\\s+")).count { it.isNotBlank() }

    private fun missingSections(body: String, sections: List<String>): List<String> =
        sections.filter { !body.contains("# $it") }.map { "missing body section: $it" }

    // The failure modes the schemas declare for every artifact kind: a CRLF
    // file and an empty frontmatter block are both rejected by the linters,
    // so the contract tests have to reject them too.
    private fun textErrors(raw: String, frontmatter: Map<String, String>): List<String> = buildList {
        if (raw.contains("\r")) add("CRLF line endings — convert to LF")
        if (frontmatter.isEmpty()) add("empty frontmatter")
    }

    private fun validateAgent(frontmatter: Map<String, String>, body: String, raw: String, directory: String?): List<String> {
        val errors = textErrors(raw, frontmatter).toMutableList()
        errors += required(frontmatter, "name", "description", "color", "division")
        frontmatter["division"]?.let {
            if (it !in divisions) errors += "division '$it' is not one of $divisions"
            if (directory != null && directory != it) errors += "directory '$directory' != division '$it'"
        }
        errors += missingSections(body, agentSections)
        if (wordCount(body) < 50) errors += "body is under 50 words"
        return errors
    }

    private fun validateSkill(frontmatter: Map<String, String>, body: String, raw: String): List<String> {
        val errors = textErrors(raw, frontmatter).toMutableList()
        errors += required(frontmatter, "name", "description", "division")
        frontmatter["division"]?.let { if (it !in divisions) errors += "division '$it' is not one of $divisions" }
        errors += missingSections(body, skillSections)
        if (wordCount(body) < 50) errors += "body is under 50 words"
        return errors
    }

    private fun validateCommand(frontmatter: Map<String, String>, body: String, raw: String, slug: String, skills: Set<String>): List<String> {
        val errors = textErrors(raw, frontmatter).toMutableList()
        errors += required(frontmatter, "name", "description", "division")
        frontmatter["division"]?.let { if (it !in divisions) errors += "division '$it' is not one of $divisions" }
        if (!body.contains("# Delegate")) errors += "missing body section: Delegate"
        val name = frontmatter["name"]
        if (name != null && name != slug) errors += "name '$name' != command slug '$slug' (must equal the skill slug)"
        if (slug !in skills) errors += "command '$slug' has no matching skill"
        return errors
    }

    private fun placeholderErrors(text: String): List<String> =
        tokens.filter { text.contains(it) }.map { "literal placeholder leak: $it" }

    // --- the tree ---------------------------------------------------------------

    @Test
    fun agentsFollowTheSchema() {
        val directory = file(".claude/agents")
        val files = directory.listFiles().orEmpty()
            .filter { it.isDirectory && it.name in divisions }
            .flatMap { it.listFiles().orEmpty().filter { entry -> entry.extension == "md" } }
        assertTrue("no agent files found", files.isNotEmpty())
        val errors = files.flatMap { agent ->
            val (frontmatter, body, raw) = parse(agent.relativeTo(root).path)
            validateAgent(frontmatter, body, raw, agent.parentFile.name).map { "${agent.name}: $it" }
        }
        assertTrue(errors.joinToString("\n"), errors.isEmpty())
    }

    @Test
    fun skillsFollowTheSchema() {
        val files = file(".claude/skills").listFiles().orEmpty()
            .filter { it.isDirectory }
            .map { File(it, "SKILL.md") }
        assertTrue("no skills found", files.isNotEmpty())
        val errors = files.filter { !it.exists() }.map { "${it.parentFile.name}: missing SKILL.md" }.toMutableList()
        errors += files.filter { it.exists() }.flatMap { skill ->
            val (frontmatter, body, raw) = parse(skill.relativeTo(root).path)
            validateSkill(frontmatter, body, raw).map { "${skill.parentFile.name}: $it" }
        }
        assertTrue(errors.joinToString("\n"), errors.isEmpty())
    }

    @Test
    fun commandsDelegateToOneSkillEach() {
        val skills = file(".claude/skills").listFiles().orEmpty().filter { it.isDirectory }.map { it.name }.toSet()
        val files = file(".claude/commands").listFiles().orEmpty().filter { it.extension == "md" }
        assertTrue("no commands found", files.isNotEmpty())
        val errors = files.flatMap { command ->
            val (frontmatter, body, raw) = parse(command.relativeTo(root).path)
            validateCommand(frontmatter, body, raw, command.nameWithoutExtension, skills)
                .map { "${command.name}: $it" }
        }
        assertTrue(errors.joinToString("\n"), errors.isEmpty())
    }

    @Test
    fun everySkillHasOneCommand() {
        val skills = file(".claude/skills").listFiles().orEmpty().filter { it.isDirectory }.map { it.name }.toSet()
        val commands = file(".claude/commands").listFiles().orEmpty()
            .filter { it.extension == "md" }.map { it.nameWithoutExtension }.toSet()
        assertEquals(skills, commands)
    }

    @Test
    fun placeholdersSurviveOnlyBeforeTheRename() {
        val settings = file("settings.gradle.kts").readText()
        val renamed = tokens.none { settings.contains(it) }
        val guarded = file(".claude").walkTopDown().filter { it.isFile }.toList() +
            listOf(file("AGENTS.md"), file("README.md")) +
            file("docs").walkTopDown().filter { it.isFile }.toList()
        val errors = guarded.filter { it.isFile }.flatMap { entry ->
            placeholderErrors(entry.readText()).map { "${entry.relativeTo(root).path}: $it" }
        }
        if (!renamed) return // the shipped template keeps its tokens by design
        assertTrue(errors.joinToString("\n"), errors.isEmpty())
    }

    @Test
    fun requiredTreeExists() {
        val paths = listOf(
            "settings.gradle.kts", "build.gradle.kts", "gradle.properties",
            "gradle/libs.versions.toml", "gradle/wrapper/gradle-wrapper.properties",
            "gradlew", "gradlew.bat",
            "app/build.gradle.kts", "app/src/main/AndroidManifest.xml",
            "core/kernel/build.gradle.kts", "core/ui-core/build.gradle.kts",
            "feature/notes/contracts/build.gradle.kts", "feature/notes/domain/build.gradle.kts",
            "feature/notes/data/build.gradle.kts", "feature/notes/ui/build.gradle.kts",
            "feature/notes/ui/src/main/res/values/strings.xml",
            "evals/build.gradle.kts", "scripts/check-architecture.sh",
            ".claude/settings.json", ".claude/manifest.json", "AGENTS.md", "MEMORY.md", "Makefile",
        )
        val missing = paths.filter { !file(it).exists() }
        assertTrue("missing: $missing", missing.isEmpty())
    }

    @Test
    fun settingsAndManifestAreValidJson() {
        for (path in listOf(".claude/settings.json", ".claude/manifest.json")) {
            val text = file(path).readText()
            assertTrue("$path is not a JSON object", Json.parseToJsonElement(text) is JsonObject)
        }
        val settings = Json.parseToJsonElement(file(".claude/settings.json").readText()).jsonObject
        val preToolUse = settings["hooks"]!!.jsonObject["PreToolUse"]!!.jsonArray
        val command = preToolUse.joinToString(" ") { it.jsonObject["hooks"].toString() }
        assertTrue("the Bash guard is not wired in .claude/settings.json", command.contains("guard-bash.sh"))
    }

    @Test
    fun theSevenPlaybooksShipLocally() {
        val playbooks = listOf(
            "PLAYBOOK_MOBILE_ANDROID_KOTLIN.md", "PLAYBOOK_TESTS.md", "PLAYBOOK_SECURITY.md",
            "PLAYBOOK_FRONTEND.md", "PLAYBOOK_URLS.md", "PLAYBOOK_AUDIT.md", "PLAYBOOK_CLAUDE_CODE.md",
        )
        val problems = playbooks.filter { path ->
            val playbook = file(path)
            !playbook.exists() || playbook.length() < 5_000 || !playbook.readText().startsWith("# ")
        }
        assertTrue("playbook problems: $problems", problems.isEmpty())
    }

    @Test
    fun theModuleGraphIsDeclaredAndSealed() {
        val settings = file("settings.gradle.kts").readText()
        val modules = listOf(
            ":app", ":core:kernel", ":core:ui-core", ":feature:notes:contracts",
            ":feature:notes:domain", ":feature:notes:data", ":feature:notes:ui", ":evals",
        )
        val missing = modules.filter { !settings.contains("include(\"$it\")") }
        assertTrue("modules not included: $missing", missing.isEmpty())

        val kernel = file("core/kernel/build.gradle.kts").readText()
        assertTrue("the kernel must depend on nothing internal", !kernel.contains("project("))

        val domain = file("feature/notes/domain/build.gradle.kts").readText()
        assertTrue("the domain must stay pure JVM", !domain.contains("com.android"))

        val ui = file("feature/notes/ui/build.gradle.kts").readText()
        assertTrue("the ui must not depend on data", !ui.contains(":feature:notes:data"))

        val data = file("feature/notes/data/build.gradle.kts").readText()
        assertTrue("data must not depend on ui", !data.contains(":feature:notes:ui"))

        val appliers = listOf("app", "core/kernel", "core/ui-core", "feature/notes/contracts",
            "feature/notes/domain", "feature/notes/data", "feature/notes/ui", "evals")
            .filter { File(root, "$it/build.gradle.kts").readText().contains("libs.plugins.android.application") }
        assertEquals(listOf("app"), appliers)
    }

    // --- the fixtures prove the validators --------------------------------------

    @Test
    fun validFixturesPass() {
        val agent = parse("evals/fixtures/valid/agent.md")
        assertTrue(validateAgent(agent.first, agent.second, agent.third, "core").toString(), validateAgent(agent.first, agent.second, agent.third, "core").isEmpty())
        val skill = parse("evals/fixtures/valid/skill.md")
        assertTrue(validateSkill(skill.first, skill.second, skill.third).toString(), validateSkill(skill.first, skill.second, skill.third).isEmpty())
        val command = parse("evals/fixtures/valid/command.md")
        assertTrue(validateCommand(command.first, command.second, command.third, "demo-android-author", setOf("demo-android-author")).isEmpty())
    }

    @Test
    fun invalidFixturesFail() {
        val badDivision = parse("evals/fixtures/invalid/bad-division.md")
        assertTrue(validateSkill(badDivision.first, badDivision.second, badDivision.third).isNotEmpty())

        val missingSection = parse("evals/fixtures/invalid/missing-body-section.md")
        assertTrue(validateSkill(missingSection.first, missingSection.second, missingSection.third).isNotEmpty())

        val missingKey = parse("evals/fixtures/invalid/missing-frontmatter-key.md")
        assertTrue(validateSkill(missingKey.first, missingKey.second, missingKey.third).isNotEmpty())

        val leak = parse("evals/fixtures/invalid/literal-placeholder-leak.md")
        assertTrue(placeholderErrors(leak.second).isNotEmpty())

        val nameMismatch = parse("evals/fixtures/invalid/command-name-mismatch.md")
        val mismatch = validateCommand(nameMismatch.first, nameMismatch.second, nameMismatch.third, "command-name-mismatch", setOf("command-name-mismatch"))
        assertTrue(mismatch.toString(), mismatch.any { it.contains("!= command slug") })
    }

    @Test
    fun crlfFails() {
        val text = file("evals/fixtures/valid/skill.md").readText().replace("\n", "\r\n")
        val (frontmatter, body, raw) = parseText(text, "crlf-skill.md")
        val errors = validateSkill(frontmatter, body, raw)
        assertTrue(errors.toString(), errors.any { it.contains("CRLF") })
    }

    @Test
    fun emptyFrontmatterFails() {
        val (frontmatter, body, raw) = parseText("---\n---\n\n# demo\n", "empty-frontmatter.md")
        val errors = validateSkill(frontmatter, body, raw)
        assertTrue(errors.toString(), errors.any { it.contains("empty frontmatter") })
    }
}
