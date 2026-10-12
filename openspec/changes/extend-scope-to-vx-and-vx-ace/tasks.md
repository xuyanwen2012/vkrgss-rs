# Tasks

## 1. Decisions

- [ ] 1.1 `docs/decisions.md`: "Goal and scope" rewritten for the three runtimes; a section for the decisions on several runtimes, the corpus, the state before V3, VX Ace and the process; the sentence on `Library=` says that it is planned to tell the runtime too and tells only the font and the language today; the head of the file names this change as the one that changed the scope; the risks and the follow-up of `design.md` are in "Known risks". Verify: scenarios "Scope", "Standard" and "Each decision is there"; read the head of the file and the sentence on `Library=` against this task.
- [ ] 1.2 The rows V0 to V3 in "Milestones", each "Not started" as of this change, and a sentence that the confirmation of 2026-10-09 concerns M0 to M6. Verify: scenarios "Milestone table" and "Status follows evidence"; read that sentence against this task.

## 2. Public documents

- [ ] 2.1 The first paragraph of `README.md`, the first sentence of `AGENTS.md` and one sentence of `docs/compatibility.md`. As of this change: XP supported, VX and VX Ace planned. Verify: scenarios "README" and "Agent guide and compatibility", by reading the three places.

## 3. Integration check

- [ ] 3.1 Run the gate. Verify: `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate` exits 0 (the checks of the documents and their links included) and `openspec validate extend-scope-to-vx-and-vx-ace --strict` passes.
