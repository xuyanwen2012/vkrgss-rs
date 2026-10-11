# Design

## Decisions

### Forgetting the binding

`Presenter` gains `forget_bound`, which sets `bound` to `None`, and
`VkCompositor::ensure_target` calls it after destroying the old target, beside the two
places that already clear the record for the held frame. The comment says why: the view the
set points at is gone and the new one may get its handle.

### The tests without the validation layer

The gate gains a step `test-unvalidated` after `test`: `cargo test -p vkxp-vk -p vkxp-host
--locked` with `VK_LOADER_LAYERS_DISABLE='*validation*'` added to the environment (the
loader then does not load the layer although it is installed; the tests already run when
the layer is absent and say so). The two failing tests named in the proposal are the test
of the fix: they fail in this step without it. `docs/development.md`, `AGENTS.md` and the
comment in `xtask` that list the steps are updated; the CI workflow runs the gate and needs
no edit unless it lists the steps.

A test in the harness that should fail when the layer is disabled but expected (one that
asserts a deliberate validation error is recorded, for example
`deliberate_validation_error_is_recorded`) skips itself with a note when the layer is not
active, if it does not do so already.

### Not releasing on a headless surface of an old Mesa

`Surface` learns whether it was made for `VK_EXT_headless_surface`. `release_undrawn`
returns `false`, without calling the driver, when the surface is headless and the driver is
Mesa older than 25.3.4: the driver name of `VkPhysicalDeviceDriverProperties` (or the
driver information the device already keeps) names Mesa, and Mesa packs `driverVersion` as
`VK_MAKE_VERSION(major, minor, patch)`. The caller then marks the swapchain stale, the path
it takes without swapchain maintenance. The decision is one small pure function with a unit
test over versions (25.2.8 and 25.3.3 do not release; 25.3.4 and 26.2.4 do; another driver
does). The test `present_after_a_failed_draw_succeeds_with_and_without_swapchain_maintenance`
says in its note which path the device took.

### The false report of layers without present-fence tracking

In the test harness only: when the validation layer's specification version, from
`vkEnumerateInstanceLayerProperties`, is below 1.4.313, a message whose id is
`VUID-vkDestroySemaphore-semaphore-05149` is dropped from the log if the compositor under
test uses present fences, and the harness prints one line that it dropped it and why. No
`vkDeviceWaitIdle` is added to the compositor for the layer's sake: it would undo what the
fences are for, and the test asserts that the swapchain was not idled.

The harness judges a log, often after the compositor was dropped, so the compositor leaves
the one fact it needs there: `ValidationLog` records that a swapchain whose presents carry
fences was made. Nothing outside the tests reads it. The id is looked for in the message
text, which holds it with the old layers (in the prefix) and with the new ones (in the link).

### What cannot be verified in this run

The second and third decision concern a stack that is not on the development machine and
cannot be started in the sandbox (no container). They are verified here by their unit
tests and by the unchanged behaviour on the local stack (Mesa 26.2.4, layer 1.4.341, where
neither is active). The owner runs the tests in a container of Ubuntu 24.04 after the run.

## Follow-ups

None yet.
