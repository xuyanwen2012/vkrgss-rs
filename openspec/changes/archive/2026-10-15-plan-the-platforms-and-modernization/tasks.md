# Tasks

## 1. Decisions

- [x] 1.1 `docs/decisions.md`: "Goal and scope" names Linux x86_64 as supported now and the planned platforms in order; a section "Platforms" with the decisions of `design.md` per platform, test devices included; the decisions on floats in saves, Vulkan, display, audio, Linux packages, CI and the order of work in the sections where they belong; the risks named in `design.md` (CORE-MATH coverage, Flatpak and Landlock, Android feasibility) under "Known risks". Verify: scenarios "Scope", "Windows without save states" and "Each decision is there".
- [x] 1.2 Rows P1 to P4 in "Milestones", each "Not started", with outcome and acceptance. Verify: scenarios "Milestone rows" and "ARM Linux".

## 2. Public documents

- [x] 2.1 One sentence in `docs/compatibility.md` that names the planned platforms and that only Linux x86_64 is supported now. Verify: read it.

## 3. Integration check

- [x] 3.1 Run the gate. Verify: `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate` exits 0 and `openspec validate plan-the-platforms-and-modernization --strict` passes.
