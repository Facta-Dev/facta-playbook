// contracts_test.cpp — native contract evals for the desktop template.
//
// Validates, from disk:
//   (a) every .claude/agents file: the required frontmatter keys, a division
//       in the allowed enum, every required body section, and the division
//       matching the directory the file lives in;
//   (b) every .claude/skills/<slug>/SKILL.md and .claude/commands/<cmd>.md:
//       the same contract against its schema, plus a 1:1 command->skill
//       mapping (every command has a matching skill directory, and the
//       command division matches the delegated skill division);
//   (c) no residual template placeholder outside the frozen fixtures, the
//       scripts/ machinery and this evals/ tree;
//   (d) the rename-state marker: evals/CMakeLists.txt carries the project
//       name that scripts/rename.sh substituted.
//
// The schema docs under .claude/schemas/*.schema.md are the source of truth:
// they are parsed at runtime (the same sections the bash linters read), so
// the evaluator cannot drift from the schema. The bash linters
// (scripts/lint-agents.sh, lint-skills.sh, lint-commands.sh) parse them too.
//
// Zero third-party dependencies: the frontmatter parser, the section checker
// and the tree walk are hand-rolled on the standard library.
//
// This file carries NO literal placeholder token: scripts/rename.sh rewrites
// evals/contracts_test.cpp (only evals/fixtures/ is frozen), and a literal
// token here would be substituted away, silently defusing the leak
// detector. The tokens are assembled at runtime in placeholderTokens().
#include <QtTest>

#include <algorithm>
#include <cctype>
#include <filesystem>
#include <fstream>
#include <regex>
#include <sstream>
#include <string>
#include <vector>

namespace fs = std::filesystem;

namespace {

// --- tiny text helpers -------------------------------------------------------

std::string trim(const std::string& s) {
  const auto notSpace = [](unsigned char c) { return !std::isspace(c) != 0; };
  auto begin = s.begin();
  while (begin != s.end() && !notSpace(static_cast<unsigned char>(*begin)))
    ++begin;
  auto end = s.end();
  while (end != begin && !notSpace(static_cast<unsigned char>(*(end - 1))))
    --end;
  return std::string(begin, end);
}

std::string lower(std::string s) {
  std::transform(s.begin(), s.end(), s.begin(),
                 [](unsigned char c) { return static_cast<char>(std::tolower(c)); });
  return s;
}

bool readFile(const fs::path& path, std::string& out) {
  std::ifstream in(path, std::ios::binary);
  if (!in)
    return false;
  std::ostringstream buffer;
  buffer << in.rdbuf();
  out = buffer.str();
  return true;
}

bool isBinary(const std::string& data) {
  return data.find('\0') != std::string::npos;
}

QString qstr(const std::string& s) { return QString::fromStdString(s); }

// --- schema model ------------------------------------------------------------

// Schema is the parsed content of .claude/schemas/<kind>.schema.md: the
// required frontmatter fields, the required body sections and the allowed
// divisions. The parser mirrors scripts/lint-agents.sh: bullets under the
// requested "## " heading, stopping at the first non-blank non-bullet line.
struct Schema {
  std::vector<std::string> frontmatter;
  std::vector<std::string> sections;
  std::vector<std::string> divisions;
};

std::vector<std::string> schemaBullets(const std::string& text, const std::string& heading) {
  std::vector<std::string> bullets;
  std::istringstream in(text);
  std::string line;
  bool capturing = false;
  while (std::getline(in, line)) {
    if (!capturing) {
      if (line == "## " + heading)
        capturing = true;
      continue;
    }
    const std::string t = trim(line);
    if (t.rfind("- ", 0) == 0) {
      bullets.push_back(t.substr(2));
      continue;
    }
    if (t.empty())
      continue;
    break;
  }
  return bullets;
}

bool loadSchema(const fs::path& path, Schema& schema) {
  std::string text;
  if (!readFile(path, text))
    return false;
  schema.frontmatter = schemaBullets(text, "Required Frontmatter Fields");
  schema.sections = schemaBullets(text, "Required Body Sections");
  schema.divisions = schemaBullets(text, "Allowed Divisions");
  return !schema.frontmatter.empty() && !schema.sections.empty() &&
         !schema.divisions.empty();
}

// --- frontmatter parser ------------------------------------------------------

struct Field {
  std::string key;
  std::string value;
};

struct Document {
  std::vector<Field> fields;
  std::string body;
  bool closed = false;
};

// parseFrontmatter reads the leading "---" fenced block and returns the
// fields plus the body after the closing fence.
Document parseFrontmatter(const std::string& text) {
  Document doc;
  std::istringstream in(text);
  std::string line;
  if (!std::getline(in, line))
    return doc;
  if (trim(line) != "---")
    return doc;
  while (std::getline(in, line)) {
    if (trim(line) == "---") {
      doc.closed = true;
      break;
    }
    const std::string t = trim(line);
    if (t.empty() || t.rfind("#", 0) == 0)
      continue;
    const auto colon = line.find(':');
    if (colon == std::string::npos)
      continue;
    Field field;
    field.key = trim(line.substr(0, colon));
    field.value = trim(line.substr(colon + 1));
    if (field.value.size() >= 2) {
      const char first = field.value.front();
      const char last = field.value.back();
      if ((first == '"' || first == '\'') && first == last)
        field.value = field.value.substr(1, field.value.size() - 2);
    }
    doc.fields.push_back(field);
  }
  std::ostringstream body;
  body << in.rdbuf();
  doc.body = body.str();
  return doc;
}

bool hasField(const Document& doc, const std::string& key) {
  return std::any_of(doc.fields.begin(), doc.fields.end(),
                     [&key](const Field& f) { return f.key == key; });
}

std::string fieldValue(const Document& doc, const std::string& key) {
  for (const Field& f : doc.fields) {
    if (f.key == key)
      return f.value;
  }
  return {};
}

// hasSection reports whether the body carries a heading whose text contains
// the section name (case-insensitive). The containment rule matches the
// bash linters, which grep the body for "# <section>".
bool hasSection(const std::string& body, const std::string& section) {
  std::istringstream in(body);
  std::string line;
  while (std::getline(in, line)) {
    std::string t = trim(line);
    if (t.empty() || t.front() != '#')
      continue;
    while (!t.empty() && t.front() == '#')
      t.erase(t.begin());
    t = trim(t);
    if (!t.empty() && t.back() == ':')
      t.pop_back();
    t = trim(t);
    if (lower(t).find(lower(section)) != std::string::npos)
      return true;
  }
  return false;
}

// --- placeholder tokens ------------------------------------------------------

// The three template substitution tokens, assembled from pieces so this file
// never spells one out literally (scripts/rename.sh rewrites this file).
const std::vector<std::string>& placeholderTokens() {
  static const std::vector<std::string> tokens = {
      std::string("<") + "app" + ">",
      std::string("<") + "APP" + ">",
      std::string("<") + "scope" + ">"};
  return tokens;
}

bool hasPlaceholder(const std::string& text) {
  for (const std::string& token : placeholderTokens()) {
    if (text.find(token) != std::string::npos)
      return true;
  }
  return false;
}

// --- validator ---------------------------------------------------------------

struct Result {
  bool ok = true;
  std::vector<std::string> errors;

  void fail(const std::string& message) {
    ok = false;
    errors.push_back(message);
  }

  QString joined() const {
    QStringList out;
    for (const std::string& e : errors)
      out << qstr(e);
    return out.join(QStringLiteral("\n"));
  }
};

// validate checks one document against its schema. checkLeak is false for
// the pre-rename template state, where placeholders are legitimate.
Result validate(const std::string& text, const Schema& schema, bool checkLeak) {
  Result result;
  if (text.find('\r') != std::string::npos)
    result.fail("CRLF line endings - convert to LF");

  const Document doc = parseFrontmatter(text);
  if (!doc.closed) {
    result.fail("missing or malformed frontmatter (no closing ---)");
    return result;
  }
  if (doc.fields.empty())
    result.fail("empty frontmatter");

  for (const std::string& field : schema.frontmatter) {
    if (!hasField(doc, field))
      result.fail("missing frontmatter key '" + field + "'");
  }

  if (hasField(doc, "division")) {
    const std::string division = fieldValue(doc, "division");
    if (std::find(schema.divisions.begin(), schema.divisions.end(), division) ==
        schema.divisions.end())
      result.fail("bad division '" + division + "'");
  }

  for (const std::string& section : schema.sections) {
    if (!hasSection(doc.body, section))
      result.fail("missing body section '" + section + "'");
  }

  if (checkLeak && hasPlaceholder(text))
    result.fail("literal placeholder leak - rename.sh must substitute all placeholders");

  return result;
}

// --- tree helpers ------------------------------------------------------------

// evalsDir is the directory this translation unit lives in; the kit root is
// its parent. Resolved at compile time so the evals read the tree they were
// built from (the staged copy under scripts/verify-template.sh included).
fs::path evalsDir() {
#ifdef DESKTOP_EVALS_DIR
  return fs::path(DESKTOP_EVALS_DIR);
#else
  return fs::current_path();
#endif
}

fs::path kitRoot() { return evalsDir().parent_path(); }

fs::path schemaPath(const std::string& kind) {
  return kitRoot() / ".claude" / "schemas" / (kind + ".schema.md");
}

// projectName extracts the name of the first project() call of a CMake file.
std::string projectName(const fs::path& path) {
  std::string text;
  if (!readFile(path, text))
    return {};
  const std::regex re(R"(project\s*\(\s*([^\s)]+))");
  std::smatch match;
  if (!std::regex_search(text, match, re))
    return {};
  return match[1].str();
}

// slugPrefix drops the trailing "-<suffix>" of a CMake project name, so
// "demo-desktop" and "demo-evals" share the slug "demo".
std::string slugPrefix(const std::string& project) {
  const auto dash = project.rfind('-');
  if (dash == std::string::npos)
    return project;
  return project.substr(0, dash);
}

// renamedTree reports whether the tree has already been through
// scripts/rename.sh: the root project name carries no placeholder token.
bool renamedTree() {
  const std::string root = projectName(kitRoot() / "CMakeLists.txt");
  return !root.empty() && !hasPlaceholder(root);
}

} // namespace

class ContractsTest : public QObject {
  Q_OBJECT

private slots:
  void validAgentFixturePasses();
  void validSkillFixturePasses();
  void validCommandFixturePasses();
  void missingFrontmatterKeyFails();
  void badDivisionFails();
  void missingBodySectionFails();
  void placeholderLeakFails();
  void emptyFrontmatterFails();
  void missingFrontmatterFails();
  void crlfFails();

  void agentsTreeMatchesSchema();
  void skillsTreeMatchesSchema();
  void commandsTreeMatchesSchema();
  void everyCommandHasAMatchingSkill();

  void noPlaceholderLeaksOutsideFrozenDirs();
  void evalsProjectNameTracksRename();
};

// --- fixture level ------------------------------------------------------------

void ContractsTest::validAgentFixturePasses() {
  Schema schema;
  QVERIFY2(loadSchema(schemaPath("agents"), schema), "agents schema not found");
  std::string text;
  QVERIFY(readFile(evalsDir() / "fixtures" / "valid" / "agent.md", text));
  const Result result = validate(text, schema, true);
  QVERIFY2(result.ok, qPrintable(result.joined()));
}

void ContractsTest::validSkillFixturePasses() {
  Schema schema;
  QVERIFY2(loadSchema(schemaPath("skills"), schema), "skills schema not found");
  std::string text;
  QVERIFY(readFile(evalsDir() / "fixtures" / "valid" / "skill.md", text));
  const Result result = validate(text, schema, true);
  QVERIFY2(result.ok, qPrintable(result.joined()));
}

void ContractsTest::validCommandFixturePasses() {
  Schema schema;
  QVERIFY2(loadSchema(schemaPath("commands"), schema), "commands schema not found");
  std::string text;
  QVERIFY(readFile(evalsDir() / "fixtures" / "valid" / "command.md", text));
  const Result result = validate(text, schema, true);
  QVERIFY2(result.ok, qPrintable(result.joined()));
}

void ContractsTest::missingFrontmatterKeyFails() {
  Schema schema;
  QVERIFY2(loadSchema(schemaPath("skills"), schema), "skills schema not found");
  std::string text;
  QVERIFY(readFile(evalsDir() / "fixtures" / "invalid" / "missing-frontmatter-key.md", text));
  const Result result = validate(text, schema, true);
  QVERIFY(!result.ok);
  QVERIFY2(result.joined().contains(QStringLiteral("missing frontmatter key")),
           qPrintable(result.joined()));
}

void ContractsTest::badDivisionFails() {
  Schema schema;
  QVERIFY2(loadSchema(schemaPath("skills"), schema), "skills schema not found");
  std::string text;
  QVERIFY(readFile(evalsDir() / "fixtures" / "invalid" / "bad-division.md", text));
  const Result result = validate(text, schema, true);
  QVERIFY(!result.ok);
  QVERIFY2(result.joined().contains(QStringLiteral("bad division")),
           qPrintable(result.joined()));
}

void ContractsTest::missingBodySectionFails() {
  Schema schema;
  QVERIFY2(loadSchema(schemaPath("skills"), schema), "skills schema not found");
  std::string text;
  QVERIFY(readFile(evalsDir() / "fixtures" / "invalid" / "missing-body-section.md", text));
  const Result result = validate(text, schema, true);
  QVERIFY(!result.ok);
  QVERIFY2(result.joined().contains(QStringLiteral("missing body section")),
           qPrintable(result.joined()));
}

void ContractsTest::placeholderLeakFails() {
  Schema schema;
  QVERIFY2(loadSchema(schemaPath("skills"), schema), "skills schema not found");
  std::string text;
  QVERIFY(readFile(evalsDir() / "fixtures" / "invalid" / "literal-placeholder-leak.md", text));
  const Result result = validate(text, schema, true);
  QVERIFY(!result.ok);
  QVERIFY2(result.joined().contains(QStringLiteral("literal placeholder leak")),
           qPrintable(result.joined()));
}

void ContractsTest::emptyFrontmatterFails() {
  Schema schema;
  QVERIFY2(loadSchema(schemaPath("skills"), schema), "skills schema not found");
  const Result result = validate("---\n---\n\n# body\n", schema, true);
  QVERIFY(!result.ok);
  QVERIFY2(result.joined().contains(QStringLiteral("empty frontmatter")),
           qPrintable(result.joined()));
}

void ContractsTest::missingFrontmatterFails() {
  Schema schema;
  QVERIFY2(loadSchema(schemaPath("skills"), schema), "skills schema not found");
  const Result result = validate("# No frontmatter here\n\n## When to Activate\n", schema, true);
  QVERIFY(!result.ok);
  QVERIFY2(result.joined().contains(QStringLiteral("missing or malformed frontmatter")),
           qPrintable(result.joined()));
}

void ContractsTest::crlfFails() {
  Schema schema;
  QVERIFY2(loadSchema(schemaPath("skills"), schema), "skills schema not found");
  std::string text;
  QVERIFY(readFile(evalsDir() / "fixtures" / "valid" / "skill.md", text));
  std::string crlf = text;
  std::replace(crlf.begin(), crlf.end(), '\n', '\r');
  const Result result = validate(crlf, schema, true);
  QVERIFY(!result.ok);
  QVERIFY2(result.joined().contains(QStringLiteral("CRLF")), qPrintable(result.joined()));
}

// --- tree level ---------------------------------------------------------------

void ContractsTest::agentsTreeMatchesSchema() {
  Schema schema;
  QVERIFY2(loadSchema(schemaPath("agents"), schema), "agents schema not found");
  const fs::path root = kitRoot() / ".claude" / "agents";
  if (!fs::is_directory(root))
    QSKIP("no .claude/agents tree in this checkout");

  std::vector<fs::path> files;
  for (const auto& entry : fs::recursive_directory_iterator(root)) {
    if (entry.is_regular_file() && entry.path().extension() == ".md")
      files.push_back(entry.path());
  }
  if (files.empty())
    QSKIP("no agent files in .claude/agents");

  const bool checkLeak = renamedTree();
  QStringList failures;
  for (const fs::path& file : files) {
    std::string text;
    if (!readFile(file, text)) {
      failures << qstr(file.string()) + QStringLiteral(": unreadable");
      continue;
    }
    const Result result = validate(text, schema, checkLeak);
    if (!result.ok)
      failures << qstr(file.string()) + QStringLiteral(":\n") + result.joined();
    // The directory an agent lives in must equal its own division.
    const Document doc = parseFrontmatter(text);
    const std::string division = fieldValue(doc, "division");
    const std::string dir = file.parent_path().filename().string();
    if (!division.empty() && division != dir)
      failures << qstr(file.string()) +
                QStringLiteral(": division '") + qstr(division) +
                QStringLiteral("' does not match its directory '") + qstr(dir) +
                QStringLiteral("'");
  }
  QVERIFY2(failures.isEmpty(), qPrintable(failures.join(QStringLiteral("\n"))));
}

void ContractsTest::skillsTreeMatchesSchema() {
  Schema schema;
  QVERIFY2(loadSchema(schemaPath("skills"), schema), "skills schema not found");
  const fs::path root = kitRoot() / ".claude" / "skills";
  if (!fs::is_directory(root))
    QSKIP("no .claude/skills tree in this checkout");

  std::vector<fs::path> files;
  for (const auto& entry : fs::recursive_directory_iterator(root)) {
    if (entry.is_regular_file() && entry.path().filename() == "SKILL.md")
      files.push_back(entry.path());
  }
  if (files.empty())
    QSKIP("no SKILL.md in .claude/skills");

  const bool checkLeak = renamedTree();
  QStringList failures;
  for (const fs::path& file : files) {
    std::string text;
    if (!readFile(file, text)) {
      failures << qstr(file.string()) + QStringLiteral(": unreadable");
      continue;
    }
    const Result result = validate(text, schema, checkLeak);
    if (!result.ok)
      failures << qstr(file.string()) + QStringLiteral(":\n") + result.joined();
  }
  QVERIFY2(failures.isEmpty(), qPrintable(failures.join(QStringLiteral("\n"))));
}

void ContractsTest::commandsTreeMatchesSchema() {
  Schema schema;
  QVERIFY2(loadSchema(schemaPath("commands"), schema), "commands schema not found");
  const fs::path root = kitRoot() / ".claude" / "commands";
  if (!fs::is_directory(root))
    QSKIP("no .claude/commands tree in this checkout");

  std::vector<fs::path> files;
  for (const auto& entry : fs::directory_iterator(root)) {
    if (entry.is_regular_file() && entry.path().extension() == ".md")
      files.push_back(entry.path());
  }
  if (files.empty())
    QSKIP("no command files in .claude/commands");

  const bool checkLeak = renamedTree();
  QStringList failures;
  for (const fs::path& file : files) {
    std::string text;
    if (!readFile(file, text)) {
      failures << qstr(file.string()) + QStringLiteral(": unreadable");
      continue;
    }
    const Result result = validate(text, schema, checkLeak);
    if (!result.ok) {
      failures << qstr(file.string()) + QStringLiteral(":\n") + result.joined();
      continue;
    }
    const std::string slug = file.stem().string();
    const Document doc = parseFrontmatter(text);
    const std::string name = fieldValue(doc, "name");
    if (!name.empty() && name != slug)
      failures << qstr(file.string()) + QStringLiteral(": name '") + qstr(name) +
                QStringLiteral("' does not match the command slug '") + qstr(slug) +
                QStringLiteral("'");
  }
  QVERIFY2(failures.isEmpty(), qPrintable(failures.join(QStringLiteral("\n"))));
}

void ContractsTest::everyCommandHasAMatchingSkill() {
  const fs::path commands = kitRoot() / ".claude" / "commands";
  const fs::path skills = kitRoot() / ".claude" / "skills";
  if (!fs::is_directory(commands))
    QSKIP("no .claude/commands tree in this checkout");
  if (!fs::is_directory(skills))
    QSKIP("no .claude/skills tree in this checkout");

  QStringList failures;
  for (const auto& entry : fs::directory_iterator(commands)) {
    if (!entry.is_regular_file() || entry.path().extension() != ".md")
      continue;
    const std::string slug = entry.path().stem().string();
    const fs::path skill = skills / slug / "SKILL.md";
    if (!fs::is_regular_file(skill)) {
      failures << QStringLiteral("command '") + qstr(slug) +
                QStringLiteral("' has no matching skill directory (") +
                qstr(skill.string()) + QStringLiteral(")");
      continue;
    }
    std::string commandText;
    std::string skillText;
    if (!readFile(entry.path(), commandText) || !readFile(skill, skillText))
      continue;
    const std::string commandDivision =
        fieldValue(parseFrontmatter(commandText), "division");
    const std::string skillDivision = fieldValue(parseFrontmatter(skillText), "division");
    if (!commandDivision.empty() && !skillDivision.empty() &&
        commandDivision != skillDivision)
      failures << QStringLiteral("command '") + qstr(slug) +
                QStringLiteral("' division '") + qstr(commandDivision) +
                QStringLiteral("' does not match its skill division '") +
                qstr(skillDivision) + QStringLiteral("'");
  }
  QVERIFY2(failures.isEmpty(), qPrintable(failures.join(QStringLiteral("\n"))));
}

// --- placeholder sweep --------------------------------------------------------

void ContractsTest::noPlaceholderLeaksOutsideFrozenDirs() {
  if (!renamedTree())
    QSKIP("pre-rename tree: the placeholders are the template state, not a leak");

  const fs::path root = kitRoot();
  // Frozen data and pattern carriers are excluded by design: evals/fixtures
  // keeps its deliberate leak fixture, evals/ keeps this file's runtime
  // token assembly, and scripts/ keeps the gates' own grep patterns.
  const std::vector<std::string> skipDirs = {".git", "scripts", "evals", "fixtures",
                                             "node_modules", "bin", "target",
                                             "CMakeFiles"};
  const std::vector<std::string> binaryExts = {".png", ".jpg", ".jpeg", ".ico", ".gif",
                                               ".woff", ".woff2", ".ttf", ".otf", ".eot"};

  QStringList leaks;
  for (fs::recursive_directory_iterator it(root, fs::directory_options::skip_permission_denied),
       end;
       it != end; ++it) {
    if (it->is_directory()) {
      const std::string name = it->path().filename().string();
      // Build trees are generated, not authored: skip them and their content.
      if (std::find(skipDirs.begin(), skipDirs.end(), name) != skipDirs.end() ||
          name.rfind("build", 0) == 0)
        it.disable_recursion_pending();
      continue;
    }
    const auto& entry = *it;
    if (!entry.is_regular_file())
      continue;
    const std::string ext = entry.path().extension().string();
    if (std::find(binaryExts.begin(), binaryExts.end(), ext) != binaryExts.end())
      continue;
    std::string text;
    if (!readFile(entry.path(), text))
      continue;
    if (isBinary(text))
      continue;
    if (hasPlaceholder(text))
      leaks << qstr(fs::relative(entry.path(), root).string());
  }
  QVERIFY2(leaks.isEmpty(), qPrintable(leaks.join(QStringLiteral("\n"))));
}

// --- rename marker ------------------------------------------------------------

void ContractsTest::evalsProjectNameTracksRename() {
  const std::string root = projectName(kitRoot() / "CMakeLists.txt");
  const std::string evals = projectName(evalsDir() / "CMakeLists.txt");
  QVERIFY2(!root.empty(), "no project() in the kit root CMakeLists.txt");
  QVERIFY2(!evals.empty(), "no project() in evals/CMakeLists.txt");

  if (!renamedTree()) {
    // Pre-rename: the marker still carries the template placeholder.
    QVERIFY2(hasPlaceholder(evals),
             "evals/CMakeLists.txt must carry the placeholder before rename.sh runs");
    QSKIP("pre-rename tree: the marker check runs after scripts/rename.sh");
  }

  QVERIFY2(!hasPlaceholder(evals),
           "evals/CMakeLists.txt still carries a placeholder after rename.sh");
  QVERIFY2(evals.rfind("-evals", std::string::npos) != std::string::npos,
           "the evals project name must end with -evals");
  QVERIFY2(slugPrefix(root) == slugPrefix(evals),
           qPrintable(QStringLiteral("root project '") + qstr(root) +
                      QStringLiteral("' and evals project '") + qstr(evals) +
                      QStringLiteral("' disagree")));
}

QTEST_APPLESS_MAIN(ContractsTest)
#include "contracts_test.moc"
