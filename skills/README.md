# Prism agent skills

[Agent Skills](https://agentskills.io) that give a coding agent the *judgment* to use Prism well — the
MCP server and the CLI give it *access* (tools + data); these skills give it the workflow rules,
selection criteria, and gotchas to use that access correctly.

| Skill | Teaches | Pairs with |
|---|---|---|
| [`prism-code-navigation`](prism-code-navigation/SKILL.md) | When to use which `nav_*` tool, the orient→seed→expand workflow, seeding mechanics, the name-based-resolution and truncation gotchas. | the `prism-mcp` MCP server ([`../docs/MCP.md`](../docs/MCP.md)) |
| [`prism-code-slicing`](prism-code-slicing/SKILL.md) | Picking the right slicing algorithm for a reviewer's question, the diff-driven workflow, output formats, the git-history and C/C++ type-info gotchas. | the `slicing` CLI ([`../README.md`](../README.md)) |

Each skill is a directory with a `SKILL.md` (YAML frontmatter + Markdown), per the
[Agent Skills specification](https://agentskills.io/specification). They are provider-agnostic.

## Installing

- **Claude Code** — copy or symlink a skill directory into `~/.claude/skills/` (user-level) or
  `<repo>/.claude/skills/` (project-level):
  ```bash
  ln -s "$PWD/skills/prism-code-navigation" ~/.claude/skills/
  ln -s "$PWD/skills/prism-code-slicing"   ~/.claude/skills/
  ```
- **Codex / Kiro / other** — point your skills loader at `skills/<name>/SKILL.md` (the standard layout),
  or vendor the directories into wherever that agent discovers skills.

A skill activates on its `description`; once active the agent loads `SKILL.md` and applies the workflow.
For the MCP connection these skills assume, see [`../docs/MCP.md`](../docs/MCP.md).
