# Spec Delta

## ADDED Requirements

### Requirement: Exact values on sRGB surfaces
On a surface that offers only sRGB formats, where the device supports views of a swapchain
image in another format, the presenter SHALL render through a `UNORM` view of the image so
that the 8-bit values of the frame are stored unchanged. Where the device does not support
that, the presenter SHALL still present, compensating for the format's encoding in the shader.

#### Scenario: sRGB only, views supported
- **WHEN** the surface offers only `B8G8R8A8_SRGB` and the device has the mutable-format extension
- **THEN** the swapchain is created for views in `B8G8R8A8_UNORM`, the pass renders through such a view and the shader applies no transfer function

#### Scenario: sRGB only, views not supported
- **WHEN** the same surface is used on a device without the extension
- **THEN** the presenter uses the sRGB format with the inverse transfer in the shader

#### Scenario: Forced path under validation
- **WHEN** the presenter is made to take the sRGB format on a surface that has one, frames are presented and the surface is resized
- **THEN** all presents succeed and the validation layer reports no error

### Requirement: Last-resort surface format
When a surface offers none of the preferred formats, the presenter SHALL take a format in the
sRGB non-linear colour space if there is one.

#### Scenario: Unknown formats
- **WHEN** the surface offers a wide-gamut format first and a 10-bit format in the sRGB non-linear colour space second
- **THEN** the second is chosen
