use macroquad::prelude::*;

use crate::game::character::AllocStat;
use crate::game::item::Rarity;
use crate::render::assets::{CANVAS_HEIGHT, CANVAS_WIDTH};

/// A queued piece of text, positioned/sized in logical canvas space. Text is
/// never rasterized into the low-res canvas render target — a pixel font
/// drawn at the tiny literal sizes that fit a 480x270 canvas (7-10px) is
/// illegible no matter the filtering, since the glyph rasterizer barely has
/// enough pixels to express a letterform. Instead every screen queues its
/// text here during its normal (canvas-space) layout pass, and `flush_text`
/// draws it afterwards directly in real screen space, at its true final
/// size — the font atlas is rasterized fresh at a readable size instead of
/// being blockily magnified as an image.
pub struct TextCmd {
    text: String,
    x: f32,
    y: f32,
    size: f32,
    color: Color,
}

pub fn push_text(cmds: &mut Vec<TextCmd>, text: impl Into<String>, x: f32, y: f32, size: f32, color: Color) {
    cmds.push(TextCmd {
        text: text.into(),
        x,
        y,
        size,
        color,
    });
}

/// The scale/offset the logical canvas is currently being letterboxed at —
/// shared by the canvas blit and by `flush_text` so queued text lines up
/// exactly with the canvas-space art it was laid out against.
pub fn canvas_transform() -> (f32, f32, f32) {
    let scale = (screen_width() / CANVAS_WIDTH)
        .min(screen_height() / CANVAS_HEIGHT)
        .floor()
        .max(1.0);
    let x = (screen_width() - CANVAS_WIDTH * scale) / 2.0;
    let y = (screen_height() - CANVAS_HEIGHT * scale) / 2.0;
    (scale, x, y)
}

/// Draws every queued text command in real screen space, after the canvas
/// has been blitted — must run after `set_default_camera()`. Uses the
/// hard-edge alpha-threshold material so glyph edges read crisp like the
/// Nearest-filtered tile art instead of carrying fontdue's usual AA fringe.
pub fn flush_text(font: &Font, material: &Material, cmds: &[TextCmd]) {
    let (scale, ox, oy) = canvas_transform();
    gl_use_material(material);
    for cmd in cmds {
        draw_text_ex(
            &cmd.text,
            ox + cmd.x * scale,
            oy + cmd.y * scale,
            TextParams {
                font: Some(font),
                font_size: (cmd.size * scale).round() as u16,
                color: cmd.color,
                ..Default::default()
            },
        );
    }
    gl_use_default_material();
}

/// Color for an HP bar/number based on remaining fraction — shared by the
/// explore and combat screens so party/enemy health reads consistently.
pub fn hp_color(ratio: f64) -> Color {
    if ratio > 0.5 {
        GREEN
    } else if ratio > 0.2 {
        YELLOW
    } else {
        RED
    }
}

/// Color for a weapon/armor/ring's rarity tier — climbs from a plain gray
/// (Common) to a striking gold (Legendary).
pub fn rarity_color(rarity: Rarity) -> Color {
    match rarity {
        Rarity::Common => GRAY,
        Rarity::Uncommon => GREEN,
        Rarity::Rare => SKYBLUE,
        Rarity::Epic => MAGENTA,
        Rarity::Legendary => YELLOW,
    }
}

/// Draws a sprite icon (from `tiles` or `characters`) at canvas-space `x, y`,
/// scaled to `size`x`size` — drawn in the normal per-screen canvas pass
/// alongside tile/character art, not through `TextCmd`/`flush_text`.
pub fn draw_icon(texture: &Texture2D, rect: Rect, x: f32, y: f32, size: f32) {
    draw_icon_tinted(texture, rect, x, y, size, WHITE);
}

/// Same as `draw_icon`, but multiplies the texture by `color` instead of
/// always drawing it untinted — used for the procedural monster silhouettes
/// (`render::assets::monster_rect`), which are plain white so they can be
/// recolored per-species/elite at draw time via `combat::species_color`,
/// the same way a Kenney icon would be tinted.
pub fn draw_icon_tinted(texture: &Texture2D, rect: Rect, x: f32, y: f32, size: f32, color: Color) {
    draw_texture_ex(
        texture,
        x,
        y,
        color,
        DrawTextureParams {
            dest_size: Some(vec2(size, size)),
            source: Some(rect),
            ..Default::default()
        },
    );
}

/// Top-left `y` to draw an `icon_size`-tall icon at so its vertical center
/// lines up with the cap-height center of a text line baselined at `text_y`
/// with font size `text_size` — text is baseline-anchored (`flush_text`)
/// while `draw_icon` is top-left-anchored, so pairing them at the same raw
/// `y` reads as visibly misaligned without this conversion.
pub fn icon_y_for_text(text_y: f32, text_size: f32, icon_size: f32) -> f32 {
    let text_center = text_y - text_size * 0.35;
    text_center - icon_size / 2.0
}

/// Fixed color per allocatable stat — no matching heart/droplet/boot/clover
/// art exists in the bundled sheets, so stats get a colored label instead of
/// a sprite icon.
pub fn stat_color(stat: AllocStat) -> Color {
    match stat {
        AllocStat::MaxHp => RED,
        AllocStat::MaxMp => SKYBLUE,
        AllocStat::Attack => ORANGE,
        AllocStat::Defense => GRAY,
        AllocStat::Speed => GREEN,
        AllocStat::Luck => PURPLE,
    }
}

/// Index range to render so `cursor` stays visible within a `visible`-row
/// window — the small canvas can't fit long bag/shop lists in full, unlike
/// the old ratatui `List` widget which auto-scrolled.
pub fn scroll_window(len: usize, cursor: usize, visible: usize) -> std::ops::Range<usize> {
    if len <= visible {
        return 0..len;
    }
    let start = cursor.saturating_sub(visible - 1).min(len - visible);
    start..(start + visible)
}
