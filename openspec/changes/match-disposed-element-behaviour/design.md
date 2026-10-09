# Design

## Context

- Measured with two probe scripts in the original runtime (RGSS103J) and in vkxp. The
  original's output is in this clone at `local/measurements/disposed-objects.original.txt`
  (ignored by git): one line per method of each disposed class with `ok` or the exception, then
  the viewport and disposed-bitmap cases. vkxp agreed on every line but those the proposal
  lists.
- `vkxp-binding`'s element classes check for disposal at the start of every method and raise
  the class's message; constructors raise for a disposed viewport. `vkxp-core`'s scene treats
  elements of a disposed viewport as not drawn but leaves them alive.
- In the original a disposed bitmap that a sprite or window still refers to raises nothing at
  the next frame, and assigning a disposed bitmap is allowed; vkxp already agrees.

## Decisions

- The two readers skip the disposal check and return the object the instance variable holds.
- `Scene::dispose_viewport` disposes the elements that were created in the viewport, so their
  Ruby objects report `disposed?` through the existing path. An element created in a viewport
  that is already disposed is not disposed by that: it stays alive and undrawn.
- Constructors store the disposed viewport's Ruby object and create the element with a viewport
  handle that draws nothing.

## Follow-ups

- Whether an element created in a disposed viewport is drawn if it is given a visible viewport
  later cannot arise: RGSS1 has no viewport writer.
