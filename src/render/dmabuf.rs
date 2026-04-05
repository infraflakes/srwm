use anyhow::{Context, ensure};
use smithay::backend::allocator::dmabuf::Dmabuf;
use smithay::backend::allocator::{Buffer, Fourcc};
use smithay::backend::renderer::element::RenderElement;
use smithay::backend::renderer::gles::{GlesMapping, GlesRenderer, GlesTarget, GlesTexture};
use smithay::backend::renderer::sync::SyncPoint;
use smithay::backend::renderer::{Bind, Color32F, ExportMem, Frame, Offscreen, Renderer};
use smithay::utils::{Physical, Rectangle, Scale, Size, Transform};

/// Render elements into a DMA-BUF target.
pub fn render_to_dmabuf(
    renderer: &mut GlesRenderer,
    mut dmabuf: Dmabuf,
    size: Size<i32, Physical>,
    scale: Scale<f64>,
    transform: Transform,
    elements: impl Iterator<Item = impl RenderElement<GlesRenderer>>,
) -> anyhow::Result<SyncPoint> {
    let mut target = renderer.bind(&mut dmabuf).context("error binding dmabuf")?;
    render_elements(renderer, &mut target, size, scale, transform, elements)
}

/// Clear a DMA-BUF to transparent.
pub fn clear_dmabuf(renderer: &mut GlesRenderer, mut dmabuf: Dmabuf) -> anyhow::Result<SyncPoint> {
    let size = dmabuf.size();
    let size = size.to_logical(1, Transform::Normal).to_physical(1);
    let mut target = renderer.bind(&mut dmabuf).context("error binding dmabuf")?;
    let mut frame = renderer
        .render(&mut target, size, Transform::Normal)
        .context("error starting frame")?;
    frame
        .clear(Color32F::TRANSPARENT, &[Rectangle::from_size(size)])
        .context("error clearing")?;
    frame.finish().context("error finishing frame")
}

/// Render elements to a texture then download to CPU memory.
pub fn render_and_download(
    renderer: &mut GlesRenderer,
    size: Size<i32, Physical>,
    scale: Scale<f64>,
    transform: Transform,
    fourcc: Fourcc,
    elements: impl Iterator<Item = impl RenderElement<GlesRenderer>>,
) -> anyhow::Result<GlesMapping> {
    let buffer_size = size.to_logical(1).to_buffer(1, Transform::Normal);

    let mut texture: GlesTexture = renderer
        .create_buffer(fourcc, buffer_size)
        .context("error creating texture")?;

    {
        let mut target = renderer
            .bind(&mut texture)
            .context("error binding texture")?;
        render_elements(renderer, &mut target, size, scale, transform, elements)?;
    }

    let target = renderer
        .bind(&mut texture)
        .context("error binding texture for copy")?;
    let mapping = renderer
        .copy_framebuffer(&target, Rectangle::from_size(buffer_size), fourcc)
        .context("error copying framebuffer")?;
    Ok(mapping)
}

fn render_elements(
    renderer: &mut GlesRenderer,
    target: &mut GlesTarget,
    size: Size<i32, Physical>,
    scale: Scale<f64>,
    transform: Transform,
    elements: impl Iterator<Item = impl RenderElement<GlesRenderer>>,
) -> anyhow::Result<SyncPoint> {
    let transform = transform.invert();
    let output_rect = Rectangle::from_size(transform.transform_size(size));

    let mut frame = renderer
        .render(target, size, transform)
        .context("error starting frame")?;

    frame
        .clear(Color32F::TRANSPARENT, &[output_rect])
        .context("error clearing")?;

    for element in elements {
        let src = element.src();
        let dst = element.geometry(scale);

        if let Some(mut damage) = output_rect.intersection(dst) {
            damage.loc -= dst.loc;
            element
                .draw(&mut frame, src, dst, &[damage], &[], None)
                .context("error drawing element")?;
        }
    }

    frame.finish().context("error finishing frame")
}

/// Compute the encompassing geometry of a set of elements.
pub fn encompassing_geo(
    scale: Scale<f64>,
    elements: impl Iterator<Item = impl smithay::backend::renderer::element::Element>,
) -> Rectangle<i32, Physical> {
    elements
        .map(|ele| ele.geometry(scale))
        .reduce(|a, b| a.merge(b))
        .unwrap_or_default()
}
