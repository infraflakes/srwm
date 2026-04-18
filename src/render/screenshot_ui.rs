use std::cmp::{max, min};
use std::collections::HashMap;

use smithay::backend::allocator::Fourcc;
use smithay::backend::renderer::ExportMem;
use smithay::backend::renderer::element::Kind;
use smithay::backend::renderer::element::memory::{
    MemoryRenderBuffer, MemoryRenderBufferRenderElement,
};
use smithay::backend::renderer::element::texture::{TextureBuffer, TextureRenderElement};
use smithay::backend::renderer::element::utils::RescaleRenderElement;
use smithay::backend::renderer::gles::{GlesRenderer, GlesTexture};
use smithay::input::keyboard::{Keysym, ModifiersState};
use smithay::output::{Output, WeakOutput};
use smithay::utils::{Physical, Point, Rectangle, Size, Transform};

use crate::render::OutputRenderElements;
use srwc::config::Action;

const SELECTION_BORDER: i32 = 2;

// ─── Core types ──────────────────────────────────────────────────────────────

#[allow(clippy::large_enum_variant)]
pub enum ScreenshotUi {
    Closed {
        last_selection: Option<(WeakOutput, Rectangle<i32, Physical>)>,
    },
    Open {
        selection: (Output, Point<i32, Physical>, Point<i32, Physical>),
        output_data: HashMap<Output, OutputData>,
        button: Button,
        show_pointer: bool,
    },
}

pub struct OutputData {
    pub size: Size<i32, Physical>,
    /// Frozen screenshot WITH cursor baked in.  
    pub texture_with_pointer: GlesTexture,
    pub buffer_with_pointer: TextureBuffer<GlesTexture>,
    /// Frozen screenshot WITHOUT cursor.  
    pub texture_without_pointer: GlesTexture,
    pub buffer_without_pointer: TextureBuffer<GlesTexture>,
    /// [0..3] = white selection border, [4..7] = dim overlay  
    pub rects: [(Rectangle<i32, Physical>, [f32; 4]); 8],
}

pub enum Button {
    Up,
    Down { last_pos: Point<i32, Physical> },
}

impl Button {
    fn is_down(&self) -> bool {
        matches!(self, Self::Down { .. })
    }
}

// ─── Construction & core state ───────────────────────────────────────────────

impl ScreenshotUi {
    pub fn new() -> Self {
        Self::Closed {
            last_selection: None,
        }
    }

    pub fn is_open(&self) -> bool {
        matches!(self, ScreenshotUi::Open { .. })
    }

    #[allow(clippy::mutable_key_type)]
    pub fn open(
        &mut self,
        renderer: &GlesRenderer,
        screenshots: HashMap<Output, (GlesTexture, GlesTexture)>, // (with_pointer, without_pointer)
        default_output: Output,
        show_pointer: bool,
    ) -> bool {
        if screenshots.is_empty() {
            return false;
        }

        let Self::Closed { last_selection } = self else {
            return false;
        };

        let prev = last_selection
            .take()
            .and_then(|(weak, sel)| weak.upgrade().map(|o| (o, sel)));
        let sel = match prev {
            Some(s) if screenshots.contains_key(&s.0) => s,
            _ => {
                let output = default_output;
                let Some(mode) = output.current_mode() else {
                    return false;
                };
                let transform = output.current_transform();
                let size = transform.transform_size(mode.size);
                (
                    output,
                    Rectangle::new(
                        Point::from((size.w / 4, size.h / 4)),
                        Size::from((size.w / 2, size.h / 2)),
                    ),
                )
            }
        };

        let selection = (
            sel.0,
            sel.1.loc,
            sel.1.loc + sel.1.size - Size::from((1, 1)),
        );

        #[allow(clippy::mutable_key_type)]
        let output_data: HashMap<_, _> = screenshots
            .into_iter()
            .filter_map(|(output, (tex_with, tex_without))| {
                let mode = output.current_mode()?;
                let transform = output.current_transform();
                let size = transform.transform_size(mode.size);
                let buf_with = TextureBuffer::from_texture(
                    renderer,
                    tex_with.clone(),
                    1,
                    Transform::Normal,
                    None,
                );
                let buf_without = TextureBuffer::from_texture(
                    renderer,
                    tex_without.clone(),
                    1,
                    Transform::Normal,
                    None,
                );
                Some((
                    output,
                    OutputData {
                        size,
                        texture_with_pointer: tex_with,
                        buffer_with_pointer: buf_with,
                        texture_without_pointer: tex_without,
                        buffer_without_pointer: buf_without,
                        rects: Default::default(),
                    },
                ))
            })
            .collect();

        *self = Self::Open {
            selection,
            output_data,
            button: Button::Up,
            show_pointer,
        };

        self.update_buffers();
        true
    }

    /// Extends the selection to the full output size.
    pub fn select_all(&mut self) {
        if let Self::Open {
            selection,
            output_data,
            ..
        } = self
            && let Some(data) = output_data.get(&selection.0)
        {
            let rect = Rectangle::from_size(data.size.to_logical(1).to_physical(1));
            selection.1 = rect.loc;
            selection.2 = rect.loc + rect.size - Size::from((1, 1));
        }
    }

    pub fn close(&mut self) {
        let Self::Open { selection, .. } = self else {
            return;
        };
        let last_selection = Some((
            selection.0.downgrade(),
            rect_from_corner_points(selection.1, selection.2),
        ));
        *self = Self::Closed { last_selection };
    }

    pub fn toggle_pointer(&mut self) {
        if let Self::Open { show_pointer, .. } = self {
            *show_pointer = !*show_pointer;
        }
    }

    // ─── Key → action mapping ────────────────────────────────────────────

    pub fn action(&self, raw: Keysym, mods: ModifiersState) -> Option<Action> {
        let Self::Open { button, .. } = self else {
            return None;
        };
        if matches!(button, Button::Down { .. }) && raw == Keysym::space {
            return None;
        }
        action(raw, mods)
    }

    // ─── Pointer handling ────────────────────────────────────────────────

    pub fn pointer_motion(&mut self, point: Point<i32, Physical>, output: &Output) {
        let Self::Open {
            selection,
            button: Button::Down { last_pos },
            ..
        } = self
        else {
            return;
        };

        *last_pos = point;
        if selection.0 == *output {
            selection.2 = point;
        }
        self.update_buffers();
    }

    pub fn pointer_down(&mut self, output: &Output, point: Point<i32, Physical>) {
        let Self::Open {
            selection, button, ..
        } = self
        else {
            return;
        };
        if button.is_down() {
            return;
        }

        *button = Button::Down { last_pos: point };
        *selection = (output.clone(), point, point);
        self.update_buffers();
    }

    pub fn pointer_up(&mut self) {
        let Self::Open {
            selection,
            output_data,
            button,
            ..
        } = self
        else {
            return;
        };

        let Button::Down { .. } = button else {
            return;
        };
        *button = Button::Up;

        // Expand zero-sized selections to a small default rectangle.
        let (output, a, b) = selection;
        let mut rect = rect_from_corner_points(*a, *b);
        if rect.size.is_empty() || rect.size == Size::from((1, 1)) {
            let data = &output_data[output];
            rect = Rectangle::new(
                Point::from((rect.loc.x - 16, rect.loc.y - 16)),
                Size::from((32, 32)),
            )
            .intersection(Rectangle::from_size(data.size))
            .unwrap_or_default();
            *a = rect.loc;
            *b = rect.loc + rect.size - Size::from((1, 1));
        }

        self.update_buffers();
    }

    // ─── Buffer / rectangle math ─────────────────────────────────────────

    fn update_buffers(&mut self) {
        let Self::Open {
            selection,
            output_data,
            ..
        } = self
        else {
            return;
        };

        let (sel_output, a, b) = selection;
        let mut rect;

        for (output, data) in output_data {
            let size = data.size;

            if output == sel_output {
                // Clamp selection into output bounds.
                a.x = a.x.clamp(0, size.w - 1);
                a.y = a.y.clamp(0, size.h - 1);
                b.x = b.x.clamp(0, size.w - 1);
                b.y = b.y.clamp(0, size.h - 1);
                rect = rect_from_corner_points(*a, *b);

                let bd = SELECTION_BORDER;

                // White border strips: top, bottom, left, right
                data.rects[0] = (
                    Rectangle::new(
                        Point::from((rect.loc.x - bd, rect.loc.y - bd)),
                        Size::from((rect.size.w + bd * 2, bd)),
                    ),
                    [1., 1., 1., 1.],
                );
                data.rects[1] = (
                    Rectangle::new(
                        Point::from((rect.loc.x - bd, rect.loc.y + rect.size.h)),
                        Size::from((rect.size.w + bd * 2, bd)),
                    ),
                    [1., 1., 1., 1.],
                );
                data.rects[2] = (
                    Rectangle::new(
                        Point::from((rect.loc.x - bd, rect.loc.y)),
                        Size::from((bd, rect.size.h)),
                    ),
                    [1., 1., 1., 1.],
                );
                data.rects[3] = (
                    Rectangle::new(
                        Point::from((rect.loc.x + rect.size.w, rect.loc.y)),
                        Size::from((bd, rect.size.h)),
                    ),
                    [1., 1., 1., 1.],
                );

                // Dark dim: above, below, left-of, right-of
                data.rects[4] = (
                    Rectangle::new(Point::from((0, 0)), Size::from((size.w, rect.loc.y))),
                    [0., 0., 0., 0.5],
                );
                data.rects[5] = (
                    Rectangle::new(
                        Point::from((0, rect.loc.y + rect.size.h)),
                        Size::from((size.w, size.h - rect.loc.y - rect.size.h)),
                    ),
                    [0., 0., 0., 0.5],
                );
                data.rects[6] = (
                    Rectangle::new(
                        Point::from((0, rect.loc.y)),
                        Size::from((rect.loc.x, rect.size.h)),
                    ),
                    [0., 0., 0., 0.5],
                );
                data.rects[7] = (
                    Rectangle::new(
                        Point::from((rect.loc.x + rect.size.w, rect.loc.y)),
                        Size::from((size.w - rect.loc.x - rect.size.w, rect.size.h)),
                    ),
                    [0., 0., 0., 0.5],
                );
            } else {
                // Non-selected output: full dim
                for r in &mut data.rects {
                    *r = (Rectangle::default(), [0., 0., 0., 0.]);
                }
                data.rects[4] = (Rectangle::new(Point::from((0, 0)), size), [0., 0., 0., 0.5]);
            }
        }
    }

    // ─── Rendering ───────────────────────────────────────────────────────

    pub fn render_output(
        &self,
        renderer: &mut GlesRenderer,
        output: &Output,
    ) -> Vec<OutputRenderElements> {
        let Self::Open {
            output_data,
            show_pointer,
            ..
        } = self
        else {
            return vec![];
        };
        let Some(data) = output_data.get(output) else {
            return vec![];
        };

        let mut elements: Vec<OutputRenderElements> = Vec::new();

        // Overlay rectangles on top (borders + dim).
        for (rect, color) in &data.rects {
            if rect.size.is_empty() {
                continue;
            }
            if let Some(elem) = solid_color_element(renderer, *rect, *color) {
                elements.push(elem);
            }
        }

        // Frozen screenshot underneath — pick texture based on show_pointer.
        let buffer = if *show_pointer {
            &data.buffer_with_pointer
        } else {
            &data.buffer_without_pointer
        };
        let tex_elem = TextureRenderElement::from_texture_buffer(
            Point::from((0.0, 0.0)),
            buffer,
            None,
            None,
            None,
            Kind::Unspecified,
        );
        elements.push(OutputRenderElements::Blur(tex_elem));

        elements
    }

    // ─── Capture (extract pixels from selection) ─────────────────────────

    #[allow(clippy::type_complexity)]
    pub fn capture(
        &self,
        renderer: &mut GlesRenderer,
    ) -> Result<(Size<i32, Physical>, Vec<u8>), Box<dyn std::error::Error>> {
        let Self::Open {
            selection,
            output_data,
            show_pointer,
            ..
        } = self
        else {
            return Err("screenshot UI not open".into());
        };

        let data = output_data.get(&selection.0).ok_or("output not found")?;
        let rect = rect_from_corner_points(selection.1, selection.2);

        // copy_texture expects BufferCoord space.
        // With scale=1 & transform=Normal the physical coords == buffer coords.
        let buf_rect = rect
            .to_logical(1)
            .to_buffer(1, Transform::Normal, &data.size.to_logical(1));

        let texture = if *show_pointer {
            &data.texture_with_pointer
        } else {
            &data.texture_without_pointer
        };
        let mapping = renderer.copy_texture(texture, buf_rect, Fourcc::Abgr8888)?;
        let copy = renderer.map_texture(&mapping)?;

        Ok((rect.size, copy.to_vec()))
    }
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn action(raw: Keysym, mods: ModifiersState) -> Option<Action> {
    if raw == Keysym::Escape {
        return Some(Action::CancelScreenshot);
    }
    if mods.alt || mods.shift {
        return None;
    }
    if !mods.ctrl && (raw == Keysym::space || raw == Keysym::Return) {
        return Some(Action::ConfirmScreenshot);
    }
    if mods.ctrl && raw == Keysym::c {
        return Some(Action::CancelScreenshot);
    }
    if !mods.ctrl && raw == Keysym::p {
        return Some(Action::ScreenshotTogglePointer);
    }
    None
}

pub fn rect_from_corner_points(
    a: Point<i32, Physical>,
    b: Point<i32, Physical>,
) -> Rectangle<i32, Physical> {
    let x1 = min(a.x, b.x);
    let y1 = min(a.y, b.y);
    let x2 = max(a.x, b.x);
    let y2 = max(a.y, b.y);
    Rectangle::from_extremities((x1, y1), (x2 + 1, y2 + 1))
}

/// Create a solid-color overlay element using a 1×1 MemoryRenderBuffer.
fn solid_color_element(
    renderer: &mut GlesRenderer,
    rect: Rectangle<i32, Physical>,
    color: [f32; 4],
) -> Option<OutputRenderElements> {
    let pixel: [u8; 4] = [
        (color[0] * 255.0) as u8,
        (color[1] * 255.0) as u8,
        (color[2] * 255.0) as u8,
        (color[3] * 255.0) as u8,
    ];
    let buffer = MemoryRenderBuffer::from_slice(
        &pixel,
        Fourcc::Abgr8888,
        (1, 1),
        1,
        Transform::Normal,
        None,
    );

    let location: Point<f64, Physical> = rect.loc.to_f64();
    let mem_elem = MemoryRenderBufferRenderElement::from_buffer(
        renderer,
        location,
        &buffer,
        Some(color[3]),
        None,
        Some(Size::from((rect.size.w, rect.size.h))),
        Kind::Unspecified,
    )
    .ok()?;

    Some(OutputRenderElements::Decoration(
        RescaleRenderElement::from_element(mem_elem, Point::from((0, 0)), 1.0),
    ))
}
