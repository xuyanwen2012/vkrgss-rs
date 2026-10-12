# Tasks

## 1. Decisions

- [ ] 1.1 `docs/decisions.md`: "Goal and scope" rewritten for the three runtimes; a section for the decisions on several runtimes, the corpus, the state before V3, VX Ace and the process; the sentence on `Library=` says that it also tells the runtime; the head of the file names this change as the one that changed the scope. Verify: scenarios "Scope", "Standard" and "Each decision is there".
- [ ] 1.2 The rows V0 to V3 in "Milestones", status "Not started", and a sentence that the confirmation of 2026-10-09 concerns M0 to M6. Verify: scenario "Milestone table".

## 2. Public documents

- [ ] 2.1 The first paragraph of `README.md`, the first sentence of `AGENTS.md` and one sentence of `docs/compatibility.md`. Verify: scenario "README"; `grep -rn -i "VX" README.md AGENTS.md docs/compatibility.md` shows no claim that a VX or VX Ace game runs.

## 3. Integration check

- [ ] 3.1 Run the gate. Verify: `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate` exits 0 (the checks of the documents and their links included) and `openspec validate extend-scope-to-vx-and-vx-ace --strict` passes.
