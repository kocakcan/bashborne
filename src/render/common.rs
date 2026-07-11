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

/// Draws a bg+fill bar and, if `label` is given, centers that text both
/// horizontally and vertically on the bar — safe to overlay directly since
/// `flush_text` draws all text in a later, real-screen-space pass on top of
/// whatever's baked into the canvas. Shared by the explore and combat party
/// panels so HP/MP bars aren't hand-rolled per screen.
pub fn draw_bar(
    cmds: &mut Vec<TextCmd>,
    font: &Font,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    ratio: f64,
    fill: Color,
    bg: Color,
    label: Option<(&str, f32, Color)>,
) {
    let ratio = ratio.clamp(0.0, 1.0) as f32;
    draw_rectangle(x, y, w, h, bg);
    draw_rectangle(x, y, w * ratio, h, fill);
    if let Some((text, size, color)) = label {
        let d = measure_text(text, Some(font), size as u16, 1.0);
        let tx = x + (w - d.width) / 2.0;
        let ty = y + h / 2.0 + size * 0.35;
        push_text(cmds, text, tx, ty, size, color);
    }
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

/// Draws an opaque rectangle in real screen space, converting from
/// canvas-space coordinates via `canvas_transform()` — usable only after
/// `set_default_camera()` (i.e. post-blit), unlike the normal canvas-space
/// `draw_rectangle` calls every screen otherwise uses. Lets a modal overlay
/// redraw its own panel *after* the underlying screen's text has already
/// been flushed, so that text can't bleed through the panel the way it does
/// when the panel is only baked into the canvas beneath a single later,
/// undifferentiated text-flush pass.
pub fn draw_screen_rect(x: f32, y: f32, w: f32, h: f32, color: Color) {
    let (scale, ox, oy) = canvas_transform();
    draw_rectangle(ox + x * scale, oy + y * scale, w * scale, h * scale, color);
}

/// Screen-space equivalent of `draw_rectangle_lines`, see `draw_screen_rect`.
pub fn draw_screen_rect_lines(x: f32, y: f32, w: f32, h: f32, thickness: f32, color: Color) {
    let (scale, ox, oy) = canvas_transform();
    draw_rectangle_lines(
        ox + x * scale,
        oy + y * scale,
        w * scale,
        h * scale,
        thickness * scale,
        color,
    );
}

/// Column widths for `push_gear_row` — fixed regardless of how long the
/// name/rarity text is, so ATK/DEF/price line up into real table columns
/// across rows instead of drifting with each row's name length the way a
/// single concatenated `format!` string would.
pub const GEAR_NAME_COL_W: f32 = 150.0;
pub const GEAR_ATK_COL_W: f32 = 42.0;
pub const GEAR_DEF_COL_W: f32 = 42.0;
/// Max pixel width for a wrapped description sitting under an "Item" column
/// cell — `GEAR_NAME_COL_W` minus a small safety margin so wrapped lines
/// stop short of the ATK column's gridline instead of running under it.
pub const GEAR_DESC_COL_W: f32 = GEAR_NAME_COL_W - 6.0;

/// Draws one row of a headerless gear table: name, then ATK/price cells at
/// fixed x-offsets from `x0` (see the `GEAR_*_COL_W` constants) regardless
/// of the name column's actual text length. `atk`/`def` are `None` to skip
/// that cell entirely (e.g. a sell-list row with no stat to show); pass
/// `Some((0, dim_color))` rather than `None` when the row's item just
/// happens to have a zero bonus, so the column stays visually populated
/// like a real table cell instead of leaving a hole.
pub fn push_gear_row(
    cmds: &mut Vec<TextCmd>,
    x0: f32,
    y: f32,
    size: f32,
    name: impl Into<String>,
    name_color: Color,
    atk: Option<(i32, Color)>,
    def: Option<(i32, Color)>,
    trailing: Option<(impl Into<String>, Color)>,
) {
    push_text(cmds, name, x0, y, size, name_color);
    let atk_x = x0 + GEAR_NAME_COL_W;
    if let Some((v, color)) = atk {
        push_text(cmds, format!("ATK+{v}"), atk_x, y, size, color);
    }
    let def_x = atk_x + GEAR_ATK_COL_W;
    if let Some((v, color)) = def {
        push_text(cmds, format!("DEF+{v}"), def_x, y, size, color);
    }
    if let Some((text, color)) = trailing {
        let trailing_x = def_x + GEAR_DEF_COL_W;
        push_text(cmds, text, trailing_x, y, size, color);
    }
}

/// Column x-offsets matching `push_gear_row`'s layout, so a header row and
/// vertical gridlines can be drawn at the exact same positions the data
/// cells use.
fn gear_col_x(x0: f32) -> (f32, f32, f32) {
    let atk = x0 + GEAR_NAME_COL_W;
    let def = atk + GEAR_ATK_COL_W;
    let price = def + GEAR_DEF_COL_W;
    (atk, def, price)
}

/// Column-header row for a gear table ("Item"/"ATK"/"DEF"/"Cost"), lined up
/// with `push_gear_row`'s cells below it — an actual spreadsheet reads as a
/// grid because of the header + dividers, not just aligned columns, so this
/// (plus `draw_gear_col_dividers` and `draw_gear_row_divider`) is what turns
/// the gear lists into a literal Excel-style table rather than just tidy text.
pub fn push_gear_table_header(cmds: &mut Vec<TextCmd>, x0: f32, y: f32, show_price: bool) {
    let (atk_x, def_x, price_x) = gear_col_x(x0);
    push_text(cmds, "Item", x0, y, 7.0, GRAY);
    push_text(cmds, "ATK", atk_x, y, 7.0, GRAY);
    push_text(cmds, "DEF", def_x, y, 7.0, GRAY);
    if show_price {
        push_text(cmds, "Cost", price_x, y, 7.0, GRAY);
    }
}

/// Thin vertical divider lines at each column boundary, spanning `top..bottom`
/// in canvas space (a single row's height, or the whole list's) — drawn in
/// the normal canvas-space pass alongside the panel's own border, not through
/// `TextCmd`.
pub fn draw_gear_col_dividers(x0: f32, top: f32, bottom: f32, show_price: bool) {
    let (atk_x, def_x, price_x) = gear_col_x(x0);
    let color = Color::new(1.0, 1.0, 1.0, 0.15);
    draw_line(atk_x - 4.0, top, atk_x - 4.0, bottom, 1.0, color);
    draw_line(def_x - 4.0, top, def_x - 4.0, bottom, 1.0, color);
    if show_price {
        draw_line(price_x - 4.0, top, price_x - 4.0, bottom, 1.0, color);
    }
}

/// Full-width horizontal divider separating one table row (including its
/// wrapped description) from the next.
pub fn draw_gear_row_divider(x0: f32, x1: f32, y: f32) {
    draw_line(x0, y, x1, y, 1.0, Color::new(1.0, 1.0, 1.0, 0.15));
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
