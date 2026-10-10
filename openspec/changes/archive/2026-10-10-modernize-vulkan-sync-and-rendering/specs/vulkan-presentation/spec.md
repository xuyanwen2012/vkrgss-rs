# Spec Delta

## ADDED Requirements

### Requirement: Presentation pass without render pass objects
The presentation pass and the off-screen scaling pass SHALL render with dynamic rendering. The
backend SHALL NOT create render pass or framebuffer objects.

#### Scenario: Resize
- **WHEN** frames are presented, the surface is resized and more frames are presented
- **THEN** all presents succeed, the validation layer reports no error, and no framebuffer was created for the new swapchain

#### Scenario: Scaled frame for a dump
- **WHEN** a scaled frame is read back at each scaling filter
- **THEN** it equals the frame the same test produced before this change
