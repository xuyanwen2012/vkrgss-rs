# Contributing

## Build and gate

[docs/building.md](docs/building.md) lists the prerequisites. One command checks everything a
change must pass: format, clippy with warnings denied, the tests of the workspace, the
documentation with warnings denied and `openspec validate --all`.

```sh
VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate
```

The tests render with Vulkan and refuse to run unless the loader is restricted to chosen
drivers; `*lvp*` is lavapipe, the software rasteriser of Mesa.
[docs/development.md](docs/development.md) describes the steps and the checks outside the
gate.

## The OpenSpec workflow

1. Behaviour is specified in `openspec/specs`, one directory per capability.
2. A change starts as a directory under `openspec/changes`: proposal, design, the
   requirements it adds or modifies, and tasks.
3. The tasks are implemented in order; a task is ticked when its stated verification passes.
4. The gate passes before the change is done.
5. A finished change is merged into the specifications and moved to
   `openspec/changes/archive`.

[docs/decisions.md](docs/decisions.md) records decisions that are settled, and
[AGENTS.md](AGENTS.md) the rules of the repository in short.

## What is never committed

No original DLL, no asset of the run-time package (RTP) and no Ruby source of Enterbrain is
ever committed, and neither is a game or a picture of one. Tests use procedurally generated
images only.

## Reporting a game that misbehaves

Open an issue at <https://github.com/xuyanwen2012/vkrgss-rs/issues> and state:

- what the game does in vkrgss and what it does in the original runtime;
- the command line, and the output of the same run with `--verbose`;
- the steps that lead there, or better a recording made with `--record`, which a headless
  run replays (see [docs/running.md](docs/running.md));
- the version of vkrgss, the GPU and its driver.

Do not attach the game, its assets or a DLL of the original runtime.
