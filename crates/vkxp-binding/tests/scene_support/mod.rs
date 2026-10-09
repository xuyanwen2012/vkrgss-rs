//! What the tests of the scene classes need besides the harness: the Ruby functions
//! `pixel(x, y)` and `scene_count(kind)`.
//!
//! `pixel` stands in for `Graphics.update` and a screen: it extracts a frame from the scene,
//! draws it with the reference compositor and returns the pixel at `x`, `y` as
//! `[red, green, blue, alpha]`, or `nil` outside the screen. It works on a copy of the scene
//! and of the bitmap arena and uploads every bitmap the frame refers to whole, so looking at
//! the scene counts no frame and takes no bitmap change away from a real one.
//!
//! `solid(width, height, red, green, blue)` makes an opaque bitmap of one colour.
//!
//! [`rendered`] is for the frames a scripted host recorded: it draws them in order with one
//! reference compositor, which is how a test reads the pixels of frames that were sent while
//! a script waited.
//!
//! `scene_count` returns the number of live scene elements made by Ruby objects of one kind:
//! `:sprite`, `:viewport`, `:window`, `:plane` or `:tilemap`.
//!
//! The scene is shared by every test of a test binary, and snippets of different tests are
//! evaluated in any order. A snippet that draws therefore disposes what it made before it
//! ends.

#![allow(dead_code)]

use std::sync::OnceLock;

use vkxp_binding::{scene_testing, testing};
use vkxp_compose_ref::{Compositor, Frame};
use vkxp_core::ElementKind;
use vkxp_proto::{BitmapDelta, FramePacket, Op};
use vkxp_ruby_sys::ffi::{self, VALUE};
use vkxp_ruby_sys::{Arity, HostResult, Ruby};

/// The Ruby side: the host functions return plain integers, and these make of them what a
/// snippet reads well.
const RUBY_SIDE: &str = "
def pixel(x, y)
  packed = __vkxp_pixel(x, y)
  packed && [packed >> 24, (packed >> 16) & 255, (packed >> 8) & 255, packed & 255]
end

def solid(width, height, red, green, blue)
  bitmap = Bitmap.new(width, height)
  bitmap.fill_rect(0, 0, width, height, Color.new(red, green, blue))
  bitmap
end

def scene_count(kind)
  __vkxp_scene_count([:sprite, :viewport, :window, :plane, :tilemap].index(kind))
end
";

const KINDS: [ElementKind; 5] = [
    ElementKind::Sprite,
    ElementKind::Viewport,
    ElementKind::Window,
    ElementKind::Plane,
    ElementKind::Tilemap,
];

/// The scene as the reference compositor draws it now.
fn frame() -> Frame {
    scene_testing::with_scene(|scene, bitmaps| {
        let mut scene = scene.clone();
        let mut bitmaps = bitmaps.clone();
        let mut packet = scene.extract(&mut bitmaps);
        let mut referenced = Vec::new();
        for op in &packet.ops {
            if let Op::Quad(quad) = op
                && !referenced.contains(&quad.bitmap)
            {
                referenced.push(quad.bitmap);
            }
        }
        packet.bitmaps = referenced
            .into_iter()
            .flat_map(|id| {
                let rect = bitmaps.rect(id).expect("a frame refers to live bitmaps");
                let rgba = bitmaps.rgba(id).expect("a live bitmap").to_vec();
                let (width, height) = (rect.width.unsigned_abs(), rect.height.unsigned_abs());
                [
                    BitmapDelta::Create { id, width, height },
                    BitmapDelta::Update {
                        id,
                        rect: vkxp_proto::Rect::new(0, 0, width, height),
                        rgba,
                    },
                ]
            })
            .collect();
        Compositor::new()
            .render(&packet)
            .expect("the reference compositor accepts every extracted packet")
    })
}

/// The frames of a recording as the reference compositor draws them, one after another.
///
/// The arena is shared by the tests of a binary, so a recording can start with the disposal
/// of bitmaps an earlier test made. Those were never announced to this compositor and are
/// left out.
pub fn rendered(packets: &[&FramePacket]) -> Vec<Frame> {
    let mut compositor = Compositor::new();
    let mut created = Vec::new();
    packets
        .iter()
        .map(|packet| {
            let mut packet = (*packet).clone();
            packet.bitmaps.retain(|delta| match delta {
                BitmapDelta::Create { id, .. } => {
                    created.push(*id);
                    true
                }
                BitmapDelta::Destroy { id } => created.contains(id),
                _ => true,
            });
            compositor
                .render(&packet)
                .expect("the reference compositor accepts every extracted packet")
        })
        .collect()
}

/// A `Fixnum` argument that is at least 0.
fn index(value: VALUE) -> Option<u32> {
    if !ffi::fixnum_p(value) {
        return None;
    }
    u32::try_from(ffi::fix2long(value)).ok()
}

/// The pixel as `0xRRGGBBAA`, or `nil` outside the screen.
fn pixel(_ruby: &Ruby, _recv: VALUE, args: &[VALUE]) -> HostResult {
    let (Some(x), Some(y)) = (index(args[0]), index(args[1])) else {
        return Ok(ffi::Qnil);
    };
    let frame = frame();
    if x >= frame.size.width || y >= frame.size.height {
        return Ok(ffi::Qnil);
    }
    let packed = u32::from_be_bytes(frame.pixel(x, y));
    Ok(ffi::long2fix(packed.into()))
}

/// The number of elements of the kind at this position of [`KINDS`], or `nil`.
fn scene_count(_ruby: &Ruby, _recv: VALUE, args: &[VALUE]) -> HostResult {
    let Some(&kind) = index(args[0]).and_then(|at| KINDS.get(at as usize)) else {
        return Ok(ffi::Qnil);
    };
    let count = scene_testing::scene_count(kind);
    Ok(ffi::long2fix(count.try_into().unwrap_or(i64::MAX >> 2)))
}

/// Defines `pixel`, `solid` and `scene_count` in the harness's interpreter, once.
pub fn install() {
    static INSTALLED: OnceLock<()> = OnceLock::new();
    INSTALLED.get_or_init(|| {
        testing::run(|ruby| {
            ruby.define_global_function_with_arity(c"__vkxp_pixel", Arity::Fixed(2), pixel)
                .expect("a new function");
            ruby.define_global_function_with_arity(
                c"__vkxp_scene_count",
                Arity::Fixed(1),
                scene_count,
            )
            .expect("a new function");
        });
        testing::eval(RUBY_SIDE).expect("the Ruby side of the helpers is defined");
    });
}
