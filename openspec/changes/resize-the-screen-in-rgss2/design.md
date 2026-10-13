## Context

- The scene has a logical size that every frame packet carries; the host, the compositors
  and a headless dump follow the packet. A game of RGSS2 starts at 544 by 416.
- Measured on the original runtime of VX (both DLL versions): `resize_screen` takes
  exactly two integers (a Float is truncated; `nil` and a String raise `TypeError`),
  returns `nil` and shows no frame. Each value is limited to 1 to 640 and 1 to 480:
  (641, 481) and (1000, 1000) give 640 by 480, (0, 0) and (-1, -1) give 1 by 1,
  (100, 50) gives 100 by 50. `Graphics.width` and `height` return the size set, and
  `snap_to_bitmap` returns a bitmap of that size (320 by 240 after
  `resize_screen(320, 240)`). `Graphics._reset` leaves the size.
- Not measured: what the window of the original does when the size changes, and where in
  the frame the scene is anchored (the top left corner is assumed, as elements keep their
  coordinates).

## Goals / Non-Goals

**Goals:** the function with the limits, values and frame sizes of the original.

**Non-Goals:** `Graphics._reset` of RGSS2 (it disposes nothing in the original; what a
reset with F12 does there is not measured yet); a window that changes its size on the
desktop when the game calls the function.

## Decisions

- The scene gets a writer for its logical size, limited by the caller; the binding applies
  the limits of RGSS2. The next extracted packet carries the new size.
- Elements keep their coordinates; the frame is the rectangle from (0, 0) with the new
  size.
- The host shows a frame of another size as it shows any frame: scaled into the window as
  it is. The window on the desktop keeps the size the player gave it. If the host or the
  Vulkan backend keeps something sized by the first frame, it is made to follow the
  packet, and that is recorded here.
- The function is defined in RGSS2 only.
