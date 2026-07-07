use macroquad::prelude::*;

use crate::game::map::Tile;
use crate::game::npc::NpcId;

/// The fixed logical resolution every screen is drawn at, then blitted to
/// the real window at the largest clean integer scale (see `render::mod`).
/// 480x270 is 16:9 and fits every chapter's 28x10 map at native 16px tiles
/// with room for a HUD strip below.
pub const CANVAS_WIDTH: f32 = 480.0;
pub const CANVAS_HEIGHT: f32 = 270.0;

/// Both Kenney sheets share this grid: 16x16 tiles with a 1px margin, so
/// consecutive cells are 17px apart.
const CELL: f32 = 17.0;
const TILE: f32 = 16.0;

fn cell(col: u32, row: u32) -> Rect {
    Rect::new(col as f32 * CELL, row as f32 * CELL, TILE, TILE)
}

/// Holds every loaded texture the renderer needs, plus the offscreen canvas
/// every screen draws into before it's scaled up to the real window.
pub struct Assets {
    pub tiles: Texture2D,
    pub characters: Texture2D,
    pub font: Font,
    pub canvas: RenderTarget,
    pub text_material: Material,
}

impl Assets {
    pub async fn load() -> Self {
        let tiles = Texture2D::from_file_with_format(
            include_bytes!("../../assets/roguelike_rpg_pack.png"),
            None,
        );
        tiles.set_filter(FilterMode::Nearest);
        let characters = Texture2D::from_file_with_format(
            include_bytes!("../../assets/roguelike_characters.png"),
            None,
        );
        characters.set_filter(FilterMode::Nearest);

        // Nearest-filtered so the glyph atlas scales crisply alongside the
        // tile art instead of the default Linear filter turning it to mush
        // under the canvas's integer-scale blit. Kenney Mini Square (not
        // Kenney Pixel) — its strokes are drawn on a tighter, squarer grid
        // that holds up much better once thresholded by the hard-edge text
        // shader; Kenney Pixel's rounder, unevenly-weighted strokes still
        // looked ragged even with AA removed.
        let mut font =
            load_ttf_font_from_bytes(include_bytes!("../../assets/kenney_mini_square.ttf"))
                .expect("bundled font must parse");
        font.set_filter(FilterMode::Nearest);

        let canvas = render_target(CANVAS_WIDTH as u32, CANVAS_HEIGHT as u32);
        canvas.texture.set_filter(FilterMode::Nearest);

        let text_material = load_material(
            ShaderSource::Glsl {
                vertex: TEXT_VERTEX_SHADER,
                fragment: TEXT_FRAGMENT_SHADER,
            },
            MaterialParams {
                // macroquad's default pipeline enables standard alpha blending
                // explicitly (see `quad_gl::PipelinesStorage::new`) — without
                // replicating it here, `PipelineParams::default()` blends
                // nothing, so a glyph's zeroed-out (0,0,0,0) fragments
                // overwrite the background as opaque black instead of staying
                // transparent, painting a solid black box over every letter.
                pipeline_params: PipelineParams {
                    color_blend: Some(miniquad::BlendState::new(
                        miniquad::Equation::Add,
                        miniquad::BlendFactor::Value(miniquad::BlendValue::SourceAlpha),
                        miniquad::BlendFactor::OneMinusValue(miniquad::BlendValue::SourceAlpha),
                    )),
                    ..Default::default()
                },
                // `Projection`/`Model`/`_Time` are already seeded into every
                // pipeline's ShaderMeta by `make_pipeline` — listing them
                // again here would duplicate the uniform buffer layout and
                // corrupt it (this is what silently zeroed all geometry on
                // the first attempt). `uniforms` is only for *extra* custom
                // uniforms beyond those three.
                ..Default::default()
            },
        )
        .expect("hard-edge text shader must compile");

        Self {
            tiles,
            characters,
            font,
            canvas,
            text_material,
        }
    }
}

/// Same vertex shader macroquad's default pipeline uses (see
/// `macroquad::quad_gl::shader::VERTEX`) — only the fragment shader differs.
const TEXT_VERTEX_SHADER: &str = r#"#version 100
attribute vec3 position;
attribute vec2 texcoord;
attribute vec4 color0;
attribute vec4 normal;

varying lowp vec2 uv;
varying lowp vec4 color;

uniform mat4 Model;
uniform mat4 Projection;

void main() {
    gl_Position = Projection * Model * vec4(position, 1);
    color = color0 / 255.0;
    uv = texcoord;
}"#;

/// Thresholds the font atlas's anti-aliased alpha to fully opaque/transparent,
/// so glyph edges read hard like the Nearest-filtered pixel-art tiles instead
/// of carrying a soft grayscale fringe.
const TEXT_FRAGMENT_SHADER: &str = r#"#version 100
varying lowp vec4 color;
varying lowp vec2 uv;

uniform sampler2D Texture;

void main() {
    lowp vec4 texColor = texture2D(Texture, uv);
    lowp float a = step(0.5, texColor.a);
    gl_FragColor = vec4(color.rgb, color.a * a);
}"#;

/// Atlas rect (in `tiles`) for a map tile. Wall is drawn as a tree so the
/// map's impassable border reads as a forest edge rather than a literal
/// brick wall — closer to the Pokémon-overworld feel this pass is going for.
pub fn tile_rect(tile: Tile) -> Rect {
    match tile {
        Tile::Floor => cell(6, 25),
        Tile::Wall => cell(16, 8),
        Tile::TallGrass => cell(9, 25),
        Tile::Town => cell(6, 0),
        Tile::BossLair => cell(12, 25),
    }
}

/// Atlas rect (in `characters`) for the player's overworld sprite.
pub fn player_rect() -> Rect {
    cell(0, 5)
}

/// Atlas rect (in `characters`) for a given NPC.
pub fn npc_rect(id: NpcId) -> Rect {
    match id {
        NpcId::OldHerbalist => cell(0, 9),
        NpcId::WoundedScout => cell(0, 6),
        NpcId::AshenPilgrim => cell(1, 11),
        NpcId::Blacksmith => cell(1, 9),
    }
}
