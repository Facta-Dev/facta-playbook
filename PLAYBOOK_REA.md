REA: setup in 3 commands
Setup registers REA with every agent it finds on your machine (Claude Code, Codex, Cursor, Gemini CLI) and installs a matching REA skill. Restart Claude Code afterwards.
python
# 1. register the MCP + skill with your agents (Node 22.19+ or 24.11+)
npx --yes rea-agents@latest setup

# 2. check what works on this machine
npx -y rea-agents@latest doctor --json

# 3. see which analysis providers are available
npx -y rea-agents@latest providers --json

# optional: global CLI, so you can type `rea` directly
npm install --global rea-agents

# optional, Linux x64: Ghidra 12.1.4 + JDK 21 as a provider
export GHIDRA_INSTALL_DIR=/absolute/path/to/ghidra_12.1.4_PUBLIC
export JAVA_HOME=/absolute/path/to/jdk-21
rea doctor --json
python
{
  "mcpServers": {
    "rea": {
      "command": "npx",
      "args": ["-y", "rea-agents@latest", "mcp"]
    }
  }
}

REA: which tools Claude calls, by target
You never type these names. You describe the job and Opus 5.5 picks the tool. Knowing them helps you write prompts that point at the right one.

REA: the CLI, for when you want to look yourself
python
# what is this file?
rea inspect-artifact ./Target.dmg --json

# an Electron / JS app: map processes, IPC, modules
rea analyze-javascript-application ./Target.app --json

# a native binary through Ghidra, saved as a reusable snapshot
rea analyze ./target-bin --provider ghidra --snapshot /abs/path/analysis/program.json --json
rea search    ./target-bin "Export failed" --provider ghidra --json
rea function  ./target-bin exportReport --provider ghidra --json
rea decompile ./target-bin 0x1000 --provider ghidra --json
rea xrefs     ./target-bin 0x1000 --provider ghidra --json
rea trace     ./target-bin "search" --provider ghidra --json

# two builds side by side, then keep the evidence
rea compare ./v1.2/App ./v1.3/App
rea evidence-export ./evidence ./research/evidence-bundle
Output is TOON by default and JSON with --json. Every result carries an evidence_id, so MAP.md can cite the exact finding behind each line. 
Results from the target are marked untrusted_content: true, and REA tells the agent never to follow instructions found inside them.

REA prompts for Claude: 
Trace one feature. The prompt from REA's own README, written out properly: one feature, with evidence, then a spec for your version.
python
Use REA. Target: [/Applications/App.app], an app I own a licence for.
Feature: [pairing two devices and sending a file].

1. Inspect the artifact first and tell me what kind of app it is.
2. If it ships JavaScript, analyse it as a JavaScript application, then trace the feature end to end:
   UI action → handler → IPC channel → native module or network call.
3. Write research/FEATURE.md: each step, the file or function it lives in, and its evidence_id.
4. Anything you could not prove: record it as an unknown. Do not guess.
5. Finish with a spec for OUR version: same behaviour, our own code, our own protocol names.
Do not copy code, strings, icons or brand names from the target.
Fork audit. Before you fork an open-source app, check that the binary people download is really built from the repo you're forking.
python
Use REA. I am forking [REPO] (licence: [GPL-3.0]).
Source: ./upstream. Release I downloaded: ./release/[App].

1. Compare the source to the shipped bundle. List modules that exist in one but not the other.
2. Compare the export shapes of the main modules.
3. Write research/FORK.md: what matches, what differs, and which differences would break us.
4. List every file that must keep its licence header in our fork.
Every row cites an evidence_id.
Release diff. Point it at two builds of your own app before a release. It catches t he change you forgot to put in the changelog.
python
Use REA. Compare ./builds/[v1.2] with ./builds/[v1.3] of my own app.

1. List added, removed and changed functions and modules.
2. For each change, tell me which user-facing feature it touches.
3. Flag anything that touches file deletion, permissions or network calls. Those need a manual test.
4. Draft the changelog from the evidence, not from my commit messages.
Export the evidence bundle to ./research/release-[v1.3].
Native function. For a compiled app you're licensed to inspect. Start from a string you can see in the UI and walk back to the code.
python
Use REA with the [ghidra] provider. Binary: [./target-bin], licensed to me for analysis.

1. Open the binary. Search for the string "[Scan complete]".
2. Find the function that uses it, decompile it, and get its callers and callees.
3. Trace the call path from the UI entry point to that function.
4. Explain in plain English what the function does, with inputs, outputs and side effects.
5. Close the binary when done.
Output research/FUNC-[name].md with evidence_ids. Pseudocode stays in research/, never in src/.
Unknows ledger. The step that stops Claude from building on a guess. REA keeps a list of open questions that each have a status.
python
Use REA. Read research/MAP.md and research/FEATURE*.md.

1. For every claim without an evidence_id, record an unknown.
2. List all unknowns, grouped by feature.
3. For each one: can we resolve it with another REA tool? If yes, run it and update the unknown.
4. Whatever is still unknown: mark it "design our own" in MATRIX.md.
We start the build only when the list is empty or every item is "design our own".

Craft Anything: the 4-step loop
Craft Anything doesn't stop at a report. You give it a target (a URL, an app, a game, or - for nothing) and a sentence. It runs Claude Code through four steps:
Study - capture the target
Build - write your version
Check - compare to original
Ready - or refine again

Set-up Craft Anything:
python
git clone https://github.com/storytold/craft-anything && cd craft-anything

# the desktop app
cargo run --release -p craft-anything

# the CLI
cargo install --path apps/craft-anything-cli
craft-anything-cli doctor        # checks install, coding agents, engines

# it drives YOUR Claude Code, so sign in first
claude auth status || claude auth login

# Debian/Ubuntu desktop libraries
sudo apt-get install libxkbcommon-dev libwayland-dev libx11-dev libxrandr-dev libxi-dev libgl1-mesa-dev libgtk-3-dev

Craft Anything: rebuild commands:
The first four are the README's own examples. The last three are how I'd write them for projects like mine.
python
# from the README
craft-anything-cli rebuild https://example.com "Recreate this website"
craft-anything-cli rebuild - "A snake game in Rust" --rounds 1
craft-anything-cli rebuild https://example.com "Make the header blue" --continue-in "<project folder>"
craft-anything-cli rebuild - "Add a contact form" --project-folder "<your folder>" --existing edit

# a Motion Atlas entry: rebuild the feel of a site, clean room
craft-anything-cli rebuild https://reference-site.com \
  "Recreate the scroll animation and layout rhythm with my own copy, fonts and images. No logos, no text from the site." \
  --clean-room --rounds 3

# a Roadster City-style game from published specs only, no target
craft-anything-cli rebuild - \
  "A browser driving game: 0-100 km/h in [spec] s, top speed [spec], open city map, original car model" --rounds 2

# refine what it built
craft-anything-cli rebuild https://reference-site.com "Match the hero easing on mobile width" \
  --continue-in "~/Documents/Craft Anything/<project>"

Craft Anything: 298 tools, by toolset
The engine ships 298 tools in 17 toolsets (the README rounds it to 320 with the AI and setup layers). craft-anything-cli commands lists them all. craft-anything-cli mcp serves them to any MCP client, including a normal Claude Code session.
The ones worth naming in a prompt: 
python
core_target_triage (what is this?), core_feature_find,
web_site_capture, web_walkthrough_record, web_pixels_compare, 
js_app_map, js_electron_walkthrough_record, android_package_show, 
native_function_pseudocode, app_ledger_build and 
app_rebuild_claim_check, which checks every claim your rebuild makes 
against the evidence.

Craft Anything from inside Claude Code:
python
claude mcp add craft-anything -- craft-anything-cli mcp
claude mcp list
python
Use the craft-anything tools. Clean room only: do not read page source, scripts or files.
Target: [https://reference-site.com].

1. Triage the target and open it.
2. Capture the site and record a walkthrough of [the hero → pricing → footer].
3. Capture the page states at three widths: 390, 834, 1440.
4. Write a brief: layout grid, type scale, colour roles, every animation with duration and easing.
5. Build our version in ./app with our own copy and images.
6. Compare pixels against the capture at all three widths. Fix the biggest differences. Repeat twice.
7. Run the rebuild claim check and save the report.
Ask me in the inbox before installing any npm or cargo package.

Using both: map with REA, build with Craft Anything
They overlap, but they're strongest at different ends. REA's evidence and unknowns make the best MAP.md. Craft Anything's study, build and check loop makes the best first version. 
This is the order I'd run for a Cleanly-type desktop app.
python
Target: [APP], which I'm licensed to inspect. Goal: our own version, published as open source.

PHASE 1 · REA (understand)
- Inspect the artifact, analyse it, trace the [3] core features.
- Write research/MAP.md. Every line cites an evidence_id.
- Record an unknown for every gap. Resolve what you can. The rest becomes "design our own".
- Export the evidence bundle to research/evidence.

PHASE 2 · Craft Anything (build)
- Run: craft-anything-cli rebuild - "[one-sentence product], spec in research/MAP.md" --clean-room --rounds 3
- The build only reads MAP.md and my screenshots. It never reads the target's code.

PHASE 3 · Check
- Read checks/ and inbox/. Fix every failing check. Answer what you can, ask me the rest.
- Run python tools/nocopy.py against the target's strings. Zero hits or we don't ship.

Report: features matched, features designed our own, open questions.
# Awesome AI Reverse Engineering

[中文](README.zh-CN.md)

Open-source **AI reverse engineering tools** for Claude Code, Codex and other agents: Ghidra MCP and IDA Pro MCP servers, jadx and Frida for mobile apps, reverse engineering skills, REA, and programs rebuilt with it. 117 repos, each one read and security-graded by [Agent Skills Hub (agentskillshub.top)](https://agentskillshub.top).

Live page with filters: **[https://agentskillshub.top/best/ai-reverse-engineering/](https://agentskillshub.top/best/ai-reverse-engineering/)** · refreshed every 8 hours

## What these tools look like

<table>
<tr>
<td align="center" valign="top" width="33%"><b>🔬 Disassembler & decompiler MCP</b><br><sub>74 repos</sub><br><br><sub>Servers and plugins that let an agent drive Ghidra, IDA Pro, Binary Ninja, radare2 or x64dbg.</sub><br><a href="#type-mcp"><b>View the list →</b></a></td>
<td align="center" valign="top" width="33%"><b>📱 Mobile app reversing</b><br><sub>17 repos</sub><br><br><sub>jadx, apktool and Frida connected to an agent, for Android, iOS and HarmonyOS apps.</sub><br><a href="#type-mobile"><b>View the list →</b></a></td>
<td align="center" valign="top" width="33%"><b>🌐 Web, JS & protocol reversing</b><br><sub>4 repos</sub><br><br><sub>Obfuscated JavaScript, request signing and traffic analysis.</sub><br><a href="#type-web"><b>View the list →</b></a></td>
</tr>
<tr>
<td align="center" valign="top" width="33%"><b>🧩 Reverse engineering skills</b><br><sub>18 repos</sub><br><br><sub>Skills and workflows that teach an agent how to reverse engineer.</sub><br><a href="#type-skill"><b>View the list →</b></a></td>
<td align="center" valign="top" width="33%"><b>🕹 Reverse-engineered with AI</b><br><sub>4 repos</sub><br><br><sub>Programs rebuilt or analysed with REA; the README of each says so.</sub><br><a href="#type-case"><b>View the list →</b></a></td>
</tr>
</table>

## Contents

- [🔬 Disassembler & decompiler MCP](#type-mcp) (74)
- [📱 Mobile app reversing](#type-mobile) (17)
- [🌐 Web, JS & protocol reversing](#type-web) (4)
- [🧩 Reverse engineering skills](#type-skill) (18)
- [🕹 Reverse-engineered with AI](#type-case) (4)

## How a repo gets on the list

1. Reverse engineering with an AI agent is its main purpose. A general security scanner, a list of links, a copy of another repository, or a tool sold on game cheating or cracking does not count.
2. It is software someone can install or run, not a list of links or a placeholder.
3. It has a README. Without one it cannot be graded.
4. At 50 stars or more it is listed on topic alone. Under 50 it must also clear a README quality bar (shows it working, one-command start, a concrete outcome, complete docs), and have 5 stars.

The questions are answered by a decision model reading each README, not by hand. A repo near a cut-off can land on either side; open an issue if one is misfiled.

<a id="type-mcp"></a>
## 🔬 Disassembler & decompiler MCP

[Open this type on the live page, sorted by stars →](https://agentskillshub.top/best/ai-reverse-engineering/#type-mcp)

| Repo | Stars | What it does | Security |
|---|---:|---|---|
| [morluto/rea](https://github.com/morluto/rea) | 433.6k | Reverse engineer anything with agents, from app behavior down to native binaries. | [SAFE](https://agentskillshub.top/skill/morluto/rea/) |
| [mrexodia/ida-pro-mcp](https://github.com/mrexodia/ida-pro-mcp) | 13.4k | AI-powered reverse engineering assistant that bridges IDA Pro with language models through MCP. | [SAFE](https://agentskillshub.top/skill/mrexodia/ida-pro-mcp/) |
| [LaurieWired/GhidraMCP](https://github.com/LaurieWired/GhidraMCP) | 11.8k | MCP Server for Ghidra | [SAFE](https://agentskillshub.top/skill/LaurieWired/GhidraMCP/) |
| [bethington/ghidra-mcp](https://github.com/bethington/ghidra-mcp) | 6.1k | Ghidra MCP Server — 200+ MCP tools for AI-powered reverse engineering. GUI plugin + headless server, lazy tool loading, convention enforcement, batch… | [SAFE](https://agentskillshub.top/skill/bethington/ghidra-mcp/) |
| [duty1g/x64dbg-mcp-server](https://github.com/duty1g/x64dbg-mcp-server) | 2.8k | x64dbg-MCP Server is a native MCP (Model Context Protocol) plugin for x64dbg that exposes the debugger's full functionality over HTTP. Connect any MC… | [SAFE](https://agentskillshub.top/skill/duty1g/x64dbg-mcp-server/) |
| [P4nda0s/IDA-NO-MCP](https://github.com/P4nda0s/IDA-NO-MCP) | 2.1k | Say goodbye to the complex, verbose, and laggy interaction mode of IDA Pro MCP | [SAFE](https://agentskillshub.top/skill/P4nda0s/IDA-NO-MCP/) |
| [angusdevgo/Seep-Reverse-Lab](https://github.com/angusdevgo/Seep-Reverse-Lab) | 2.1k | Agent-Native multi-platform reverse engineering and CWE-602 client-side authorization audit workbench. | [SAFE](https://agentskillshub.top/skill/angusdevgo/Seep-Reverse-Lab/) |
| [HexRaysSA/ida-mcp](https://github.com/HexRaysSA/ida-mcp) | 1.5k | Official Hex-Rays IDA MCP Server. | [SAFE](https://agentskillshub.top/skill/HexRaysSA/ida-mcp/) |
| [LING71671/open-reverselab](https://github.com/LING71671/open-reverselab) | 1.3k | Open-source AI reverse-engineering agent platform and MCP server for Ghidra, Frida, x64dbg and Rizin — automated PE/APK/binary analysis, CTF and malw… | [SAFE](https://agentskillshub.top/skill/LING71671/open-reverselab/) |
| [mrphrazer/reverser_ai](https://github.com/mrphrazer/reverser_ai) | 1.2k | Provides automated reverse engineering assistance through the use of local large language models (LLMs) on consumer hardware. | [SAFE](https://agentskillshub.top/skill/mrphrazer/reverser_ai/) |
| [cyberkaida/reverse-engineering-assistant](https://github.com/cyberkaida/reverse-engineering-assistant) | 943 | MCP server for reverse engineering tasks in Ghidra 👩‍💻 | [SAFE](https://agentskillshub.top/skill/cyberkaida/reverse-engineering-assistant/) |
| [symgraph/IDAssist](https://github.com/symgraph/IDAssist) | 905 | AI-Powered Reverse Engineering Plugin for IDA Pro | [SAFE](https://agentskillshub.top/skill/symgraph/IDAssist/) |
| [blacktop/ida-mcp-rs](https://github.com/blacktop/ida-mcp-rs) | 898 | Headless IDA Pro MCP Server | [SAFE](https://agentskillshub.top/skill/blacktop/ida-mcp-rs/) |
| [weirdmachine64/GhidraGPT](https://github.com/weirdmachine64/GhidraGPT) | 839 | Integrate LLM models directly into Ghidra for AI-enhanced reverse engineering. | [SAFE](https://agentskillshub.top/skill/weirdmachine64/GhidraGPT/) |
| [symgraph/GhidrAssistMCP](https://github.com/symgraph/GhidrAssistMCP) | 807 | An native MCP server extension for Ghidra | [SAFE](https://agentskillshub.top/skill/symgraph/GhidrAssistMCP/) |
| [AgentSmithers/x64DbgMCPServer](https://github.com/AgentSmithers/x64DbgMCPServer) | 746 | x64DbgMCPServer made from c# with Claude, Windsurf and Cursor support | [SAFE](https://agentskillshub.top/skill/AgentSmithers/x64DbgMCPServer/) |
| [buzzer-re/Rikugan](https://github.com/buzzer-re/Rikugan) | 706 | A reverse-engineering agent for IDA Pro and Binary Ninja | [SAFE](https://agentskillshub.top/skill/buzzer-re/Rikugan/) |
| [Wasdubya/x64dbgMCP](https://github.com/Wasdubya/x64dbgMCP) | 610 | Model Context Protocol for x64dbg & x32dbg | [SAFE](https://agentskillshub.top/skill/Wasdubya/x64dbgMCP/) |
| [SetsunaYukiOvO/x64dbg-mcp](https://github.com/SetsunaYukiOvO/x64dbg-mcp) | 553 | MCP server plugin for x64dbg debugger - enables AI agents and external tools to control debugging via JSON-RPC 2.0 over HTTP/SSE | [SAFE](https://agentskillshub.top/skill/SetsunaYukiOvO/x64dbg-mcp/) |
| [MxIris-Reverse-Engineering/ida-mcp-server](https://github.com/MxIris-Reverse-Engineering/ida-mcp-server) | 549 | A Model Context Protocol server for IDA | [SAFE](https://agentskillshub.top/skill/MxIris-Reverse-Engineering/ida-mcp-server/) |
| [MeroZemory/ida-multi-mcp](https://github.com/MeroZemory/ida-multi-mcp) | 469 | Multi-instance IDA Pro MCP server — analyze multiple binaries simultaneously through a single MCP endpoint. | [SAFE](https://agentskillshub.top/skill/MeroZemory/ida-multi-mcp/) |
| [llnl/OGhidra](https://github.com/llnl/OGhidra) | 465 | OGhidra bridges Large Language Models (LLMs) via Ollama with the Ghidra reverse engineering platform, enabling AI-driven binary analysis through natu… | [SAFE](https://agentskillshub.top/skill/llnl/OGhidra/) |
| [clearbluejar/pyghidra-mcp](https://github.com/clearbluejar/pyghidra-mcp) | 460 | Python Command-Line Ghidra MCP | [SAFE](https://agentskillshub.top/skill/clearbluejar/pyghidra-mcp/) |
| [fosdickio/binary_ninja_mcp](https://github.com/fosdickio/binary_ninja_mcp) | 455 | A Binary Ninja plugin containing an MCP server that enables seamless integration with your favorite LLM/MCP client. | [SAFE](https://agentskillshub.top/skill/fosdickio/binary_ninja_mcp/) |
| [cellebrite-labs/ghidra-rpc](https://github.com/cellebrite-labs/ghidra-rpc) | 395 | A Ghidra agentic reverse engineering skill. | [SAFE](https://agentskillshub.top/skill/cellebrite-labs/ghidra-rpc/) |
| [radareorg/radare2-mcp](https://github.com/radareorg/radare2-mcp) | 352 | MCP stdio server for radare2 | [SAFE](https://agentskillshub.top/skill/radareorg/radare2-mcp/) |
| [mrphrazer/agentic-malware-analysis](https://github.com/mrphrazer/agentic-malware-analysis) | 330 | Agentic malware analysis environment with MCP-connected disassemblers, RE tooling, and structured workflows for Claude Code and Codex CLI. | [SAFE](https://agentskillshub.top/skill/mrphrazer/agentic-malware-analysis/) |
| [Recurse-Labs/recurse](https://github.com/Recurse-Labs/recurse) | 297 | AI Native IDE for reverse engineering | [SAFE](https://agentskillshub.top/skill/Recurse-Labs/recurse/) |
| [namename333/idapromcp_333](https://github.com/namename333/idapromcp_333) | 287 | 本项目基于 [mrexodia/ida-pro-mcp](https://github.com/mrexodia/ida-pro-mcp) 二次开发，轻量化了一些功能，在大项目逆向中占用更小，消耗更少token | [SAFE](https://agentskillshub.top/skill/namename333/idapromcp_333/) |
| [saileaxh/iida-mcp](https://github.com/saileaxh/iida-mcp) | 282 | Faster, with excellent architecture, stronger performance, and kernel access support — an ida-mcp tool | [SAFE](https://agentskillshub.top/skill/saileaxh/iida-mcp/) |
| [akiselev/ghidra-cli](https://github.com/akiselev/ghidra-cli) | 266 | Automate Ghidra reverse engineering from the command line — headless analysis, decompilation, and structured JSON output for AI agents like Claude Co… | [SAFE](https://agentskillshub.top/skill/akiselev/ghidra-cli/) |
| [mrphrazer/binary-ninja-headless-mcp](https://github.com/mrphrazer/binary-ninja-headless-mcp) | 263 | Headless Binary Ninja MCP server — giving AI agents deep reverse-engineering capabilities via 180 tools. | [SAFE](https://agentskillshub.top/skill/mrphrazer/binary-ninja-headless-mcp/) |
| [JameZUK/Arkana](https://github.com/JameZUK/Arkana) | 257 | Arkana - Your entire malware analysis lab, behind one AI prompt. 250+ MCP tools for binary analysis with Claude Code or other MCP | [SAFE](https://agentskillshub.top/skill/JameZUK/Arkana/) |
| [Captain-AI-Hub/IDA-MCP](https://github.com/Captain-AI-Hub/IDA-MCP) | 253 | IDA MCP | [SAFE](https://agentskillshub.top/skill/Captain-AI-Hub/IDA-MCP/) |
| [index-login/MobileRE-Skill](https://github.com/index-login/MobileRE-Skill) | 241 | AI Agent 驱动的移动端逆向技能集：Frida hook、一键脱壳、反检测绕过、内存 DEX dump、Ghidra MCP 符号/结构恢复。AI-agent skill system for mobile reverse engineering. | [SAFE](https://agentskillshub.top/skill/index-login/MobileRE-Skill/) |
| [OwenPawl/cerberus-re-skill](https://github.com/OwenPawl/cerberus-re-skill) | 221 | cerberus-re is a local Apple-focused reverse-engineering workbench for building a repeatable three-headed static/dynamic/instrumentation loop around… | [SAFE](https://agentskillshub.top/skill/OwenPawl/cerberus-re-skill/) |
| [banteg/bn](https://github.com/banteg/bn) | 220 | binary ninja cli for coding agents | [SAFE](https://agentskillshub.top/skill/banteg/bn/) |
| [bilieebiliee1-design/SOMCP](https://github.com/bilieebiliee1-design/SOMCP) | 218 | SOMCP 是一个运行在 Android 手机上的本地 SO 逆向 MCP 服务器。它通过 Streamable HTTP 暴露 MCP 工具，让客户端可以在手机上完成 ELF 结构分析、Rizin 反汇编/分析、LIEF ELF 修复/重写、补丁会话、构建导出、Cloudflare Tunnel… | [SAFE](https://agentskillshub.top/skill/bilieebiliee1-design/SOMCP/) |
| [sjkim1127/Reversecore_MCP](https://github.com/sjkim1127/Reversecore_MCP) | 217 | A security-first MCP server that empowers AI agents to perform automated reverse engineering, malware analysis, forensics, vulnerability research, an… | [SAFE](https://agentskillshub.top/skill/sjkim1127/Reversecore_MCP/) |
| [biniamf/ai-reverse-engineering](https://github.com/biniamf/ai-reverse-engineering) | 213 | AI-Assisted Reverse Engineering with Ghidra | [SAFE](https://agentskillshub.top/skill/biniamf/ai-reverse-engineering/) |
| [dariushoule/x64dbg-skills](https://github.com/dariushoule/x64dbg-skills) | 210 | Claude Code plugin providing skills for x64dbg debugger automation | [SAFE](https://agentskillshub.top/skill/dariushoule/x64dbg-skills/) |
| [mrphrazer/ghidra-headless-mcp](https://github.com/mrphrazer/ghidra-headless-mcp) | 196 | Headless Ghidra MCP server — giving AI agents deep reverse-engineering capabilities. | [SAFE](https://agentskillshub.top/skill/mrphrazer/ghidra-headless-mcp/) |
| [taida957789/ida-mcp-server-plugin](https://github.com/taida957789/ida-mcp-server-plugin) | 191 | IDA Pro Plugin for serving MCP SSE server for cursor / claude | [SAFE](https://agentskillshub.top/skill/taida957789/ida-mcp-server-plugin/) |
| [jtsylve/re-mcp](https://github.com/jtsylve/re-mcp) | 178 | A headless MCP server for IDA Pro and Ghidra | [SAFE](https://agentskillshub.top/skill/jtsylve/re-mcp/) |
| [Fausto-404/ai-mobile-reverse-skills](https://github.com/Fausto-404/ai-mobile-reverse-skills) | 177 | 面向移动安全分析场景的 6 阶段总控 Skill。用于统一调度 APK 静态侦察、流量与代码对齐、SO/JNI 深度分析、加密与漏洞综合分析、验证设计与报告交付流程。支持 JADX MCP、Burp/Yakit MCP、IDA/Ghidra MCP。 | [SAFE](https://agentskillshub.top/skill/Fausto-404/ai-mobile-reverse-skills/) |
| [axelmierczuk/tenrec](https://github.com/axelmierczuk/tenrec) | 176 | A headless, extendable, multi-session, IDA Pro MCP framework. | [SAFE](https://agentskillshub.top/skill/axelmierczuk/tenrec/) |
| [Claude-Reverser/IDA-instances-MCP](https://github.com/Claude-Reverser/IDA-instances-MCP) | 172 | Custom build of ida-pro-mcp - stability-hardened MCP server for hosting headless IDA Pro instances | [SAFE](https://agentskillshub.top/skill/Claude-Reverser/IDA-instances-MCP/) |
| [13bm/GhidraMCP](https://github.com/13bm/GhidraMCP) | 150 | Active Socket based MCP Server for Ghidra | [SAFE](https://agentskillshub.top/skill/13bm/GhidraMCP/) |
| [philsajdak/decyx](https://github.com/philsajdak/decyx) | 140 | Decyx: AI-powered Ghidra extension for enhanced reverse engineering and binary analysis. | [SAFE](https://agentskillshub.top/skill/philsajdak/decyx/) |
| [bromoket/x64dbg_mcp](https://github.com/bromoket/x64dbg_mcp) | 138 | Drive x64dbg with your AI. MCP server: 23 mega-tools / 153 endpoints for breakpoints, memory, disasm, tracing, anti-debug & PE dumping. Claude/Cursor… | [SAFE](https://agentskillshub.top/skill/bromoket/x64dbg_mcp/) |
| [TsingShui/ida-agent-bridge](https://github.com/TsingShui/ida-agent-bridge) | 109 | A CLI-first, headless, AI-native IDA Pro bridge built on Unix philosophy. | [SAFE](https://agentskillshub.top/skill/TsingShui/ida-agent-bridge/) |
| [WPeace-HcH/WPeGPT-Analyzer](https://github.com/WPeace-HcH/WPeGPT-Analyzer) | 109 | 可移植的逆向分析 SKILL 技能，驱动 IDA + WPeGPT 插件，对 PE/ELF 进行三种模式的自动化分析（轻量/全量/漏洞），输出分析报告。支持 SKILL 技能机制的 AI Agent 均可加载使用。 | [SAFE](https://agentskillshub.top/skill/WPeace-HcH/WPeGPT-Analyzer/) |
| [fdrechsler/mcp-server-idapro](https://github.com/fdrechsler/mcp-server-idapro) | 103 | A Model Context Protocol (MCP) server that enables AI assistants to interact with IDA Pro for reverse engineering and binary analysis tasks. | [SAFE](https://agentskillshub.top/skill/fdrechsler/mcp-server-idapro/) |
| [themixednuts/GhidraMCP](https://github.com/themixednuts/GhidraMCP) | 93 |  | [SAFE](https://agentskillshub.top/skill/themixednuts/GhidraMCP/) |
| [sgInnora/sentinel-reverse](https://github.com/sgInnora/sentinel-reverse) | 85 | AI-Powered Autonomous Binary Reverse Engineering CLI — the native reverse engine from Innora-Sentinel. Local LLM inference (MLX), MPS GPU acceleratio… | [SAFE](https://agentskillshub.top/skill/sgInnora/sentinel-reverse/) |
| [president-xd/revula](https://github.com/president-xd/revula) | 84 | A fully functional and production-grade reverse engineering MCP Server | [SAFE](https://agentskillshub.top/skill/president-xd/revula/) |
| [r0ysue/r0idamcp](https://github.com/r0ysue/r0idamcp) | 82 | r0idamcp is a SSE protocal MCP server IDA pro plugin designed for automatic reverse engineering with LLM assistant | [SAFE](https://agentskillshub.top/skill/r0ysue/r0idamcp/) |
| [icloudza/algokiller-plugin](https://github.com/icloudza/algokiller-plugin) | 80 | ARM64 trace evidence analysis & cipher algorithm recovery — Claude Desktop plugin with skills + local MCP server driving the native ak_search engine… | [SAFE](https://agentskillshub.top/skill/icloudza/algokiller-plugin/) |
| [elliottophellia/Ryuumonbuchi](https://github.com/elliottophellia/Ryuumonbuchi) | 79 | Maybe the headless Ghidra MCP you are looking for. | [SAFE](https://agentskillshub.top/skill/elliottophellia/Ryuumonbuchi/) |
| [CSIT-SG/AETHER](https://github.com/CSIT-SG/AETHER) | 76 | An AI-powered reverse-engineering copilot for assisting tedious malware analysis in IDA Pro. | [SAFE](https://agentskillshub.top/skill/CSIT-SG/AETHER/) |
| [HLND2T/CS2_VibeSignatures](https://github.com/HLND2T/CS2_VibeSignatures) | 67 | Generate CS2 signatures via Agent SKILLS with ida-pro-mcp | [SAFE](https://agentskillshub.top/skill/HLND2T/CS2_VibeSignatures/) |
| [Invoke-RE/binja-lattice-mcp](https://github.com/Invoke-RE/binja-lattice-mcp) | 67 | Plugin interface for remote communications with Binary Ninja database and MCP server for interfacing with LLMs. | [SAFE](https://agentskillshub.top/skill/Invoke-RE/binja-lattice-mcp/) |
| [pandaadir05/re-architect](https://github.com/pandaadir05/re-architect) | 66 | Advanced reverse engineering platform combining traditional static analysis with AI-powered insights. Supports multiple decompilers (Ghidra, IDA Pro,… | [SAFE](https://agentskillshub.top/skill/pandaadir05/re-architect/) |
| [PhoenixHairpin/mcp-termux](https://github.com/PhoenixHairpin/mcp-termux) | 63 | MCP RE v7.0 — Android逆向工程一体化MCP服务器 \| 73个工具 \| stackplz eBPF + paradise内存 + radare2静态分析 \| ARM64 Root | [SAFE](https://agentskillshub.top/skill/PhoenixHairpin/mcp-termux/) |
| [dariushoule/x64dbg-rippy](https://github.com/dariushoule/x64dbg-rippy) | 62 | AI reverse engineering assistant for x64dbg. Embeds a chat panel directly in the debugger with tool-use capabilities. | [SAFE](https://agentskillshub.top/skill/dariushoule/x64dbg-rippy/) |
| [trailofbits/idac](https://github.com/trailofbits/idac) | 60 | idac - IDA Pro command line tool for agents and humans | [SAFE](https://agentskillshub.top/skill/trailofbits/idac/) |
| [SuZiXunYue/ida-script-mcp](https://github.com/SuZiXunYue/ida-script-mcp) | 55 |  | [SAFE](https://agentskillshub.top/skill/SuZiXunYue/ida-script-mcp/) |
| [xxipv6/IDA-PRO-MCP-multiple](https://github.com/xxipv6/IDA-PRO-MCP-multiple) | 53 | 支持ida headless 多文件分析 | [CAUTION](https://agentskillshub.top/skill/xxipv6/IDA-PRO-MCP-multiple/) |
| [MCPPhalanx/binaryninja-mcp](https://github.com/MCPPhalanx/binaryninja-mcp) | 52 | Another™ MCP Server for Binary Ninja with superpower 🥵 | [SAFE](https://agentskillshub.top/skill/MCPPhalanx/binaryninja-mcp/) |
| [symgraph/BinAssistMCP](https://github.com/symgraph/BinAssistMCP) | 50 | Binary Ninja plugin to provide MCP functionality. | [SAFE](https://agentskillshub.top/skill/symgraph/BinAssistMCP/) |
| [symgraph/IDAssistMCP](https://github.com/symgraph/IDAssistMCP) | 50 | Standalone MCP server plugin for IDA Pro. | [SAFE](https://agentskillshub.top/skill/symgraph/IDAssistMCP/) |
| [cycloarcane/ghidra-mcp-setup](https://github.com/cycloarcane/ghidra-mcp-setup) | 6 | Turnkey installer + tutorial for hooking Ghidra to Claude, gemini-cli, or Ollama via the Model Context Protocol. | [CAUTION](https://agentskillshub.top/skill/cycloarcane/ghidra-mcp-setup/) |
| [fantay0312/ida-pro-mcp-plus](https://github.com/fantay0312/ida-pro-mcp-plus) | 6 | Enhanced IDA Pro MCP Server - multi-instance broker + bulk export for AI analysis | [SAFE](https://agentskillshub.top/skill/fantay0312/ida-pro-mcp-plus/) |
| [zboralski/ida-headless-mcp](https://github.com/zboralski/ida-headless-mcp) | 6 | Headless IDA Pro binary analysis via Model Context Protocol | [SAFE](https://agentskillshub.top/skill/zboralski/ida-headless-mcp/) |

<a id="type-mobile"></a>
## 📱 Mobile app reversing

[Open this type on the live page, sorted by stars →](https://agentskillshub.top/best/ai-reverse-engineering/#type-mobile)

| Repo | Stars | What it does | Security |
|---|---:|---|---|
| [zinja-coder/jadx-ai-mcp](https://github.com/zinja-coder/jadx-ai-mcp) | 3.0k | Plugin for JADX to integrate MCP server | [SAFE](https://agentskillshub.top/skill/zinja-coder/jadx-ai-mcp/) |
| [lkimuk/ReArk](https://github.com/lkimuk/ReArk) | 932 | An intelligent reverse engineering and AI-assisted analysis tool for HarmonyOS NEXT (HAP/APP/ABC), Android (APK/AAB), iOS (IPA), and native PE/ELF/Ma… | [SAFE](https://agentskillshub.top/skill/lkimuk/ReArk/) |
| [zinja-coder/jadx-mcp-server](https://github.com/zinja-coder/jadx-mcp-server) | 834 | MCP server for JADX-AI Plugin | [SAFE](https://agentskillshub.top/skill/zinja-coder/jadx-mcp-server/) |
| [zinja-coder/apktool-mcp-server](https://github.com/zinja-coder/apktool-mcp-server) | 707 | A MCP Server for APK Tool (Part of Android Reverse Engineering MCP Suites) | [SAFE](https://agentskillshub.top/skill/zinja-coder/apktool-mcp-server/) |
| [dnakov/frida-mcp](https://github.com/dnakov/frida-mcp) | 464 | MCP stdio server for frida | [SAFE](https://agentskillshub.top/skill/dnakov/frida-mcp/) |
| [flankerhqd/jebmcp](https://github.com/flankerhqd/jebmcp) | 262 |  | [SAFE](https://agentskillshub.top/skill/flankerhqd/jebmcp/) |
| [kggzs/ApkMCP-Auto](https://github.com/kggzs/ApkMCP-Auto) | 233 | 本项目是一个 Android 逆向工程工具集合，通过 MCP (Model Context Protocol) 协议将 AI 助手（如 Claude）与专业的 Android 反编译工具连接起来，实现智能化的 APK 分析与修改。 | [SAFE](https://agentskillshub.top/skill/kggzs/ApkMCP-Auto/) |
| [ZSA233/frida-analykit](https://github.com/ZSA233/frida-analykit) | 156 | Frida 工具包 - 主要面向安卓端逆向，解决frida环境版本管理和对Agent端常用底层工具方法封装，支持MCP。（目前主要由AI开发维护代码） | [SAFE](https://agentskillshub.top/skill/ZSA233/frida-analykit/) |
| [1013503897/jadx-headless-mcp](https://github.com/1013503897/jadx-headless-mcp) | 155 | Headless MCP server for Android APK static analysis, built directly on jadx-core. No JADX GUI required. | [SAFE](https://agentskillshub.top/skill/1013503897/jadx-headless-mcp/) |
| [FuzzySecurity/kahlo-mcp](https://github.com/FuzzySecurity/kahlo-mcp) | 139 | A Frida MCP server to enable autonomous AI assistance for Android instrumentation | [SAFE](https://agentskillshub.top/skill/FuzzySecurity/kahlo-mcp/) |
| [asLody/dexdec](https://github.com/asLody/dexdec) | 126 | Fast, accurate Android decompilation for humans and AI agents. | [SAFE](https://agentskillshub.top/skill/asLody/dexdec/) |
| [zhizhuodemao/frida-mcp](https://github.com/zhizhuodemao/frida-mcp) | 123 | 一个用于frida动态调试的mcp工具，支持自定义frida路径和名称端口,自动管理frida | [SAFE](https://agentskillshub.top/skill/zhizhuodemao/frida-mcp/) |
| [mobilehackinglab/jadx-mcp-plugin](https://github.com/mobilehackinglab/jadx-mcp-plugin) | 105 |  | [SAFE](https://agentskillshub.top/skill/mobilehackinglab/jadx-mcp-plugin/) |
| [TheQmaks/areclaw](https://github.com/TheQmaks/areclaw) | 73 | Android Reverse Engineering Command-Line Automation Workspace. AI-driven security analysis with Claude Code. | [SAFE](https://agentskillshub.top/skill/TheQmaks/areclaw/) |
| [samanL33T/jadx-xvision-plugin](https://github.com/samanL33T/jadx-xvision-plugin) | 72 | xVision is a plugin for the JadX decompiler that aims to integrate with Large Language Models (LLMs) to provide code analysis directly in the JAdX. | [SAFE](https://agentskillshub.top/skill/samanL33T/jadx-xvision-plugin/) |
| [s4dp4nd4/frida-c2-mcp](https://github.com/s4dp4nd4/frida-c2-mcp) | 70 | Agentic C2-style MCP server for Frida instrumentation on rooted Android and jailbroken iOS. | [SAFE](https://agentskillshub.top/skill/s4dp4nd4/frida-c2-mcp/) |
| [crissyfield/super-trouper](https://github.com/crissyfield/super-trouper) | 62 | MCP server for Frida. | [SAFE](https://agentskillshub.top/skill/crissyfield/super-trouper/) |

<a id="type-web"></a>
## 🌐 Web, JS & protocol reversing

[Open this type on the live page, sorted by stars →](https://agentskillshub.top/best/ai-reverse-engineering/#type-web)

| Repo | Stars | What it does | Security |
|---|---:|---|---|
| [vmoranv/jshookmcp](https://github.com/vmoranv/jshookmcp) | 2.1k | A search-first, profile-aware reverse-engineering workspace for AI agents — 735 tools across 36 domains in one MCP server. | [SAFE](https://agentskillshub.top/skill/vmoranv/jshookmcp/) |
| [WhiteNightShadow/wx-mp-mcp](https://github.com/WhiteNightShadow/wx-mp-mcp) | 288 | 微信小程序逆向 MCP — 在线实时解包 + AI 静态分析还原接口签名/加密，产出本地 Node.js 直接能跑的脚本（零侵入，不碰微信进程） | [SAFE](https://agentskillshub.top/skill/WhiteNightShadow/wx-mp-mcp/) |
| [wuji66dde/jshook-skill](https://github.com/wuji66dde/jshook-skill) | 271 | AI-powered JS reverse engineering: deobfuscation, crypto detection, CDP debugging, hook injection, anti-detection \| AI驱动JS逆向：反混淆、加密识别、CDP调试、Hook注入、反… | [SAFE](https://agentskillshub.top/skill/wuji66dde/jshook-skill/) |
| [amd2g2zz/kunglao-agent](https://github.com/amd2g2zz/kunglao-agent) | 79 | The reverse-engineering expert agent: plans its own analysis path, derives every fact from raw evidence, and converges under mechanical verification… | [SAFE](https://agentskillshub.top/skill/amd2g2zz/kunglao-agent/) |

<a id="type-skill"></a>
## 🧩 Reverse engineering skills

[Open this type on the live page, sorted by stars →](https://agentskillshub.top/best/ai-reverse-engineering/#type-skill)

| Repo | Stars | What it does | Security |
|---|---:|---|---|
| [zhaoxuya520/reverse-skill](https://github.com/zhaoxuya520/reverse-skill) | 44.2k | Reverse Engineering / Authorized Penetration Testing / Security Research Skill Router Pack AI-powered routing + On-demand toolchain bootstrapping + S… | [SAFE](https://agentskillshub.top/skill/zhaoxuya520/reverse-skill/) |
| [SimoneAvogadro/android-reverse-engineering-skill](https://github.com/SimoneAvogadro/android-reverse-engineering-skill) | 8.7k | Claude Code skill to support Android app's reverse engineering | [SAFE](https://agentskillshub.top/skill/SimoneAvogadro/android-reverse-engineering-skill/) |
| [Dryxio/reagent](https://github.com/Dryxio/reagent) | 2.4k | Reconstruct and validate C/C++ code from compiled programs with AI. | [SAFE](https://agentskillshub.top/skill/Dryxio/reagent/) |
| [P4nda0s/reverse-skills](https://github.com/P4nda0s/reverse-skills) | 2.3k | Reverse engineering skills for Claude Code \| 逆向工程 Claude Code Skills 插件 | [SAFE](https://agentskillshub.top/skill/P4nda0s/reverse-skills/) |
| [2akouwu/reverify](https://github.com/2akouwu/reverify) | 1.3k | Stop your AI from making things up — it proposes, deterministic tools decide, every claim checked against ground truth with evidence. Grounded facts… | [SAFE](https://agentskillshub.top/skill/2akouwu/reverify/) |
| [CreditTone/android-reverse-engineering-skill](https://github.com/CreditTone/android-reverse-engineering-skill) | 568 | 这是一套专为 Codex 适配的 Android 逆向分析 skill，支持在 Codex 会话中反编译 APK、XAPK、JAR、AAR，并结合 jadx、Fernflower/Vineflower 梳理 Manifest、包结构、网络层和调用链。它可辅助提取接口、URL、鉴权头、token 与… | [SAFE](https://agentskillshub.top/skill/CreditTone/android-reverse-engineering-skill/) |
| [schlarpc/re-shell](https://github.com/schlarpc/re-shell) | 537 | Nix-powered agentic reverse engineering environment | [CAUTION](https://agentskillshub.top/skill/schlarpc/re-shell/) |
| [manyuegong33/r0crawl_skills](https://github.com/manyuegong33/r0crawl_skills) | 362 | r0crawl_skills is a massive reverse-engineering skill arsenal for web, Android, iOS, Frida, native binaries, protocols, malware, firmware, and anti-a… | [SAFE](https://agentskillshub.top/skill/manyuegong33/r0crawl_skills/) |
| [miunasu/IDA-Skill](https://github.com/miunasu/IDA-Skill) | 239 | 使用skill让 AI Agent 像安全分析师一样分析恶意样本 \| AI Agent skill for automated malware analysis using IDA Pro | [SAFE](https://agentskillshub.top/skill/miunasu/IDA-Skill/) |
| [vgrichina/re-skill](https://github.com/vgrichina/re-skill) | 183 | Claude Code skill for reverse engineering retro games — disassemble, annotate, extract assets, web port | [SAFE](https://agentskillshub.top/skill/vgrichina/re-skill/) |
| [incogbyte/android-reverse-engineering-claude-skill](https://github.com/incogbyte/android-reverse-engineering-claude-skill) | 134 | Claude Code skill that automates Android application reverse engineering | [SAFE](https://agentskillshub.top/skill/incogbyte/android-reverse-engineering-claude-skill/) |
| [haikow/claude-reverse-skills](https://github.com/haikow/claude-reverse-skills) | 132 | Claude Code 逆向工程 skill 集合 (apk/ida/radare2/js) + IDA MCP 接入，跨平台 Linux/Mac/Windows | [SAFE](https://agentskillshub.top/skill/haikow/claude-reverse-skills/) |
| [pardeike/DecompilerServer](https://github.com/pardeike/DecompilerServer) | 127 | A powerful MCP (Model Context Protocol) server for decompiling and analyzing .NET assemblies, with specialized support for Unity's Assembly-CSharp.dl… | [SAFE](https://agentskillshub.top/skill/pardeike/DecompilerServer/) |
| [tihanyin/REx-skill](https://github.com/tihanyin/REx-skill) | 122 | REx@Skill - Agentic Reverse Engineering eXecution Skill for binary vulnerability discovery | [SAFE](https://agentskillshub.top/skill/tihanyin/REx-skill/) |
| [incogbyte/iOS-reverse-engineering-claude-skill](https://github.com/incogbyte/iOS-reverse-engineering-claude-skill) | 118 | Claude Code skill that enables Claude to extract, analyze, reverse engineer iOS apps | [SAFE](https://agentskillshub.top/skill/incogbyte/iOS-reverse-engineering-claude-skill/) |
| [blacktop/ipsw-skill](https://github.com/blacktop/ipsw-skill) | 104 | A SKILL.md for ipsw | [SAFE](https://agentskillshub.top/skill/blacktop/ipsw-skill/) |
| [Karib0u/kernagent](https://github.com/Karib0u/kernagent) | 103 | Headless AI agent for deterministic reverse engineering. | [SAFE](https://agentskillshub.top/skill/Karib0u/kernagent/) |
| [Mo271-del/BloxAgent-Studio-Bridge](https://github.com/Mo271-del/BloxAgent-Studio-Bridge) | 69 | BloxAgent 2026: AI-Powered Luau Reverse Engineering and Roblox Script Automation Framework | [SAFE](https://agentskillshub.top/skill/Mo271-del/BloxAgent-Studio-Bridge/) |

<a id="type-case"></a>
## 🕹 Reverse-engineered with AI

[Open this type on the live page, sorted by stars →](https://agentskillshub.top/best/ai-reverse-engineering/#type-case)

| Repo | Stars | What it does | Security |
|---|---:|---|---|
| [N0zoM1z0/dx-ball](https://github.com/N0zoM1z0/dx-ball) | 83 | Reconstructing DX-Ball 1.07 with REA, backed by original-x86 differential tests and pinned MSVC byte-exact oracles. | [SAFE](https://agentskillshub.top/skill/N0zoM1z0/dx-ball/) |
| [davehornik/WH3SaveEditor](https://github.com/davehornik/WH3SaveEditor) | 8 | Warhammer 3 Save Editor | [SAFE](https://agentskillshub.top/skill/davehornik/WH3SaveEditor/) |
| [NullLabTests/OpenBSD_netcat_REA](https://github.com/NullLabTests/OpenBSD_netcat_REA) | 7 | Reverse-engineering the netcat-openbsd 1.234-1 binary with REA (Reverse Engineer Anything) + Ghidra | [SAFE](https://agentskillshub.top/skill/NullLabTests/OpenBSD_netcat_REA/) |
| [OceanJy333/meteor-mac](https://github.com/OceanJy333/meteor-mac) | 4 | 在 Apple Silicon Mac 上直接运行原版《流星蝴蝶剑》Meteor.exe：REA 逆向 + AI 写 Win32/DirectX→Mac 接口，不用 Wine；附老 Windows 程序移植 Mac 的 AI Skill | [SAFE](https://agentskillshub.top/skill/OceanJy333/meteor-mac/) |

**Security** is the grade of the repo's README and install steps on Agent Skills Hub. *pending* means the catalog has not graded it yet.

Preview images are reduced copies of pictures from each project's own README, included only for projects under a permissive license. Sources and licenses: [assets/previews/NOTICE.md](assets/previews/NOTICE.md). Open an issue to have one removed.

## Related collections

- [zhuyansen/awesome-claude-video-skills](https://github.com/zhuyansen/awesome-claude-video-skills), [zhuyansen/awesome-codex-ppt-skills](https://github.com/zhuyansen/awesome-codex-ppt-skills) — lists built the same way.

## Add a repo

Open an issue with the GitHub URL. It goes through the same review as every entry; the rules above decide, stars do not.

---

Machine-readable copy: [`data/skills.json`](data/skills.json). Generated 2026-11-05.
