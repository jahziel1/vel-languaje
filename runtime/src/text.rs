use std::sync::Arc;

use skrifa::instance::{LocationRef, Size};
use skrifa::{FontRef, MetadataProvider};
use vello::kurbo::Affine;
use vello::peniko::{Blob, Brush, Color, Fill, Font};
use vello::{Glyph, Scene};

pub struct TextPainter {
    data: Arc<dyn AsRef<[u8]> + Send + Sync>,
    font: Font,
}

impl TextPainter {
    pub fn load() -> Option<Self> {
        let candidates: &[&str] = if cfg!(windows) {
            &[
                r"C:\Windows\Fonts\segoeui.ttf",
                r"C:\Windows\Fonts\arial.ttf",
                r"C:\Windows\Fonts\calibri.ttf",
            ]
        } else if cfg!(target_os = "macos") {
            &[
                "/System/Library/Fonts/Supplemental/Arial.ttf",
                "/Library/Fonts/Arial.ttf",
            ]
        } else {
            &[
                "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
                "/usr/share/fonts/TTF/DejaVuSans.ttf",
            ]
        };

        for path in candidates {
            if let Ok(bytes) = std::fs::read(path) {
                // Coerce Arc<Vec<u8>> → Arc<dyn AsRef<[u8]> + Send + Sync> at assignment.
                let data: Arc<dyn AsRef<[u8]> + Send + Sync> = Arc::new(bytes);
                let font = Font::new(Blob::new(data.clone()), 0);
                return Some(Self { data, font });
            }
        }
        None
    }

    fn raw(&self) -> &[u8] {
        (*self.data).as_ref()
    }

    /// Draw `text` at (`x`, `baseline_y`) into the scene.
    pub fn draw(
        &self,
        scene: &mut Scene,
        text: &str,
        font_size: f32,
        x: f64,
        baseline_y: f64,
        color: Color,
    ) {
        let Ok(font_ref) = FontRef::new(self.raw()) else {
            return;
        };
        let charmap = font_ref.charmap();
        let glyph_metrics = font_ref.glyph_metrics(Size::new(font_size), LocationRef::default());

        let mut glyphs: Vec<Glyph> = Vec::with_capacity(text.len());
        let mut cursor_x: f32 = 0.0;
        for ch in text.chars() {
            let gid = charmap.map(ch as u32).unwrap_or_default();
            let advance = glyph_metrics.advance_width(gid).unwrap_or(font_size * 0.5);
            glyphs.push(Glyph {
                id: gid.to_u32(),
                x: cursor_x,
                y: 0.0,
            });
            cursor_x += advance;
        }

        scene
            .draw_glyphs(&self.font)
            .font_size(font_size)
            .transform(Affine::translate((x, baseline_y)))
            .brush(&Brush::Solid(color))
            .draw(Fill::NonZero, glyphs.iter().copied());
    }

    /// Compute the total advance width of `text` at the given font size.
    // Used by the layout engine (not yet built).
    #[allow(dead_code)]
    pub fn text_width(&self, text: &str, font_size: f32) -> f32 {
        let Ok(font_ref) = FontRef::new(self.raw()) else {
            return 0.0;
        };
        let charmap = font_ref.charmap();
        let glyph_metrics = font_ref.glyph_metrics(Size::new(font_size), LocationRef::default());
        text.chars()
            .map(|ch| {
                let gid = charmap.map(ch as u32).unwrap_or_default();
                glyph_metrics.advance_width(gid).unwrap_or(font_size * 0.5)
            })
            .sum()
    }
}
