# mdtype-rules-obsidian

Workspace rules for [`mdtype`](https://github.com/serejke/mdtype) that resolve
Obsidian-flavored wikilinks (`[[Target]]`, `[[Target|Alias]]`,
`[[Target#Heading]]`) using Obsidian's exact-path-then-shortest-path policy.

Ships one rule today: [`links.obsidian_vault`](src/links_obsidian_vault.rs).

Register the crate's factories with your YAML loader to enable workspace-rule
ids from this crate in `.mdtype.yaml` schemas.
