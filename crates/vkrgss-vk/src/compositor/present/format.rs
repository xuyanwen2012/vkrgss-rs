//! The format a swapchain is created with, and its extent.

use ash::vk;
use vkrgss_proto::Size;

/// The format a swapchain is created with and how the frame's values get into its images.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct PresentFormat {
    /// The surface format the swapchain is created with.
    pub(super) surface: vk::SurfaceFormatKHR,
    /// The format of the views the pass draws through and of the pipeline. Where it is not
    /// the surface's, the swapchain is created for views in both.
    pub(super) view: vk::Format,
    /// Whether the views encode what is written to them, so that the shader applies the
    /// inverse.
    pub(super) srgb_target: bool,
}

impl PresentFormat {
    /// The surface format as it is, drawn through views of the same format.
    pub(super) fn plain(surface: vk::SurfaceFormatKHR, srgb_target: bool) -> PresentFormat {
        PresentFormat {
            surface,
            view: surface.format,
            srgb_target,
        }
    }

    /// Whether the pass draws through views of another format than the swapchain's.
    pub(super) fn is_mutable(&self) -> bool {
        self.view != self.surface.format
    }
}

/// The `UNORM` formats that pass the frame's values through, in order of preference.
pub(super) const UNORM_FORMATS: [vk::Format; 3] = [
    vk::Format::B8G8R8A8_UNORM,
    vk::Format::R8G8B8A8_UNORM,
    vk::Format::A8B8G8R8_UNORM_PACK32,
];

/// The sRGB twin of each of [`UNORM_FORMATS`], at the same index.
pub(super) const SRGB_FORMATS: [vk::Format; 3] = [
    vk::Format::B8G8R8A8_SRGB,
    vk::Format::R8G8B8A8_SRGB,
    vk::Format::A8B8G8R8_SRGB_PACK32,
];

/// The surface format to present in and the format to draw through, on a device that has
/// `VK_KHR_swapchain_mutable_format` enabled or not.
///
/// A `UNORM` format passes the frame's values through unchanged and is preferred. With only
/// sRGB formats on offer the pass draws through a `UNORM` view of the image where the device
/// has the extension, which stores the values as they are; without it the shader undoes the
/// encoding the format applies. Of anything else the first format in the sRGB non-linear
/// colour space is taken, then the first of any, as it comes.
pub(super) fn choose_format(
    formats: &[vk::SurfaceFormatKHR],
    mutable_format: bool,
) -> Option<PresentFormat> {
    let offered = |format: vk::Format| {
        formats.iter().copied().find(|offered| {
            offered.format == format && offered.color_space == vk::ColorSpaceKHR::SRGB_NONLINEAR
        })
    };
    let unorm = || {
        let surface = UNORM_FORMATS.into_iter().find_map(offered)?;
        Some(PresentFormat::plain(surface, false))
    };
    let srgb = || {
        let (surface, twin) = std::iter::zip(SRGB_FORMATS, UNORM_FORMATS)
            .find_map(|(srgb, twin)| Some((offered(srgb)?, twin)))?;
        Some(match mutable_format {
            true => PresentFormat {
                surface,
                view: twin,
                srgb_target: false,
            },
            false => PresentFormat::plain(surface, true),
        })
    };
    let last_resort = || {
        let surface = formats
            .iter()
            .find(|offered| offered.color_space == vk::ColorSpaceKHR::SRGB_NONLINEAR)
            .or(formats.first())?;
        Some(PresentFormat::plain(*surface, false))
    };
    unorm().or_else(srgb).or_else(last_resort)
}

/// The extent of a swapchain for a surface with `capabilities` in a window of `requested`
/// pixels. A zero dimension means nothing can be presented now.
pub(super) fn swapchain_extent(
    capabilities: &vk::SurfaceCapabilitiesKHR,
    requested: Size,
) -> vk::Extent2D {
    // The special value says the surface takes whatever size the swapchain has.
    if capabilities.current_extent.width != u32::MAX {
        return capabilities.current_extent;
    }
    let (min, max) = (capabilities.min_image_extent, capabilities.max_image_extent);
    vk::Extent2D {
        width: requested.width.clamp(min.width, max.width.max(min.width)),
        height: requested
            .height
            .clamp(min.height, max.height.max(min.height)),
    }
}
