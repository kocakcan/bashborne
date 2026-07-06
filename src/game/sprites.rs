use ratatui::style::Color;

/// How many animation frames every sprite has. The combat screen cycles
/// `0..ANIM_FRAMES` on a timer (see `World::anim_frame`) so enemies idle in
/// place — a slime squashes, a bat flaps — instead of standing frozen.
pub const ANIM_FRAMES: usize = 2;

/// Small ASCII sprites for the combat screen. Keep entries narrow (~12 chars)
/// and short (~6 lines) so two or three fit comfortably side by side.
/// Keyed on the enemy's display name, which currently doubles as its species tag.
///
/// Two rules keep the animation from looking broken:
/// - both frames of a species have the *same* number of lines, so the
///   HP bar under the sprite never jumps vertically between frames;
/// - every line within a frame is padded to the same width, because the
///   combat screen centers each line independently — a line one character
///   short of its neighbors visibly drifts half a cell sideways (this is
///   exactly what made a few of the original single-frame sprites look off).
pub fn sprite_for(species_name: &str, frame: usize) -> &'static [&'static str] {
    match (species_name, frame % ANIM_FRAMES) {
        ("Slime", 0) => &[
            "  .--.  ",
            " /    \\ ",
            "| o  o |",
            "|  __  |",
            " \\____/ ",
            " ~~~~~~ ",
        ],
        ("Slime", _) => &[
            "        ",
            "  .--.  ",
            "/ o  o \\",
            "|  __  |",
            "\\______/",
            " ~~~~~~ ",
        ],
        ("Goblin", 0) => &[
            " /\\_/\\  ",
            "( o.o ) ",
            " > ^ <  ",
            "/|   |\\ ",
            " |   |  ",
            "_|   |_ ",
        ],
        ("Goblin", _) => &[
            " /\\_/\\  ",
            "( o.o ) ",
            " > ^ <  ",
            "\\|   |/ ",
            " |   |  ",
            "_|   |_ ",
        ],
        ("Bat", 0) => &[
            " /\\   /\\ ",
            "( o   o )",
            " \\  v  / ",
            "  \\___/  ",
            "  /   \\  ",
        ],
        ("Bat", _) => &[
            " \\/   \\/ ",
            "( o   o )",
            " \\  v  / ",
            "  \\___/  ",
            "  /   \\  ",
        ],
        ("Wolf", 0) => &[
            " /\\___/\\ ",
            "( o   o )",
            " \\  ^  / ",
            " /|---|\\ ",
            "* |   | *",
        ],
        ("Wolf", _) => &[
            " /\\___/\\ ",
            "( o   o )",
            " \\  ^  / ",
            " \\|---|/ ",
            " *|   |* ",
        ],
        ("Skeleton", 0) => &[
            "  .-.   ",
            " (o.o)  ",
            "  |=|   ",
            " /|-|\\  ",
            "  | |   ",
            " _| |_  ",
        ],
        ("Skeleton", _) => &[
            "  .-.   ",
            " (-.-)  ",
            "  |=|   ",
            " \\|-|/  ",
            "  | |   ",
            " _| |_  ",
        ],
        ("Orc", 0) => &[
            "  ___    ",
            " /o o\\   ",
            "( >_< )  ",
            "/|   |\\  ",
            "\\|   |/  ",
            " |___|   ",
        ],
        ("Orc", _) => &[
            "  ___    ",
            " /o o\\   ",
            "( >o< )  ",
            "\\|   |/  ",
            "/|   |\\  ",
            " |___|   ",
        ],
        ("Wraith", 0) => &[
            "  .:::.   ",
            " (: o o:) ",
            "  ':::.'  ",
            " /  |  \\  ",
            " (  |  )  ",
        ],
        ("Wraith", _) => &[
            "  .:::.   ",
            " (: o o:) ",
            "  ':::.'  ",
            " \\  |  /  ",
            " )  |  (  ",
        ],
        ("Mimic", 0) => &[
            " ______ ",
            "/|_||_|\\",
            "( ^  ^ )",
            "\\ \\/\\/ /",
            " \\____/ ",
        ],
        ("Mimic", _) => &[
            " ______ ",
            "/|_||_|\\",
            "( o  o )",
            "\\ /\\/\\ /",
            " \\____/ ",
        ],
        ("The Barrow Knight", 0) => &[
            "  /^^^\\   ",
            " |[o_o]|  ",
            " |=====|  ",
            "/|##|##|\\ ",
            " |  |  |  ",
            " '--'--'  ",
        ],
        ("The Barrow Knight", _) => &[
            "  /^^^\\   ",
            " |[o_-]|  ",
            " |=====|  ",
            "\\|##|##|/ ",
            " |  |  |  ",
            " '--'--'  ",
        ],
        ("Wyrmscale Warden", 0) => &[
            "  /\\/\\/\\   ",
            " ( o   o ) ",
            "  \\  ^^ /  ",
            " //|VVVV|\\\\",
            "   |    |  ",
            "  //    \\\\ ",
        ],
        ("Wyrmscale Warden", _) => &[
            "  /\\/\\/\\   ",
            " ( o   o ) ",
            "  \\  vv /  ",
            " \\\\|VVVV|//",
            "   |    |  ",
            "  \\\\    // ",
        ],
        ("The Ashen Sovereign", 0) => &[
            "   /##\\    ",
            "  |[**]|   ",
            "  |=><=|   ",
            " /|#|##|#\\ ",
            "  |  ||  | ",
            " ^^^  ^^^^ ",
        ],
        ("The Ashen Sovereign", _) => &[
            "   \\##/    ",
            "  |[**]|   ",
            "  |=><=|   ",
            " \\|#|##|#/ ",
            "  |  ||  | ",
            " ^^^  ^^^^ ",
        ],
        ("Hollow", 0) => &["  ___   ", " (x_x)  ", " /| |\\  ", "  | |   ", " _/ \\_  "],
        ("Hollow", _) => &["  ___   ", " (x_x)  ", " \\| |/  ", "  | |   ", " _/ \\_  "],
        ("Rat", 0) => &[" (\\__/) ", " (o..o) ", "  |__|~ "],
        ("Rat", _) => &[" (\\__/) ", " (-..-) ", " ~|__|  "],
        ("Carrion Crow", 0) => &[" \\(o)/  ", "  |V|   ", "  / \\   "],
        ("Carrion Crow", _) => &[" /(o)\\  ", "  |V|   ", "  / \\   "],
        ("Bandit", 0) => &["  ,-.   ", " (>_>)  ", " /|x|\\  ", "  | |   ", " _| |_  "],
        ("Bandit", _) => &["  ,-.   ", " (<_<)  ", " \\|x|/  ", "  | |   ", " _| |_  "],
        ("Fell Acolyte", 0) => &[
            "  .^.   ",
            " (u_u)  ",
            " /|+|\\  ",
            "  |||   ",
            " //|\\\\  ",
        ],
        ("Fell Acolyte", _) => &[
            "  .^.   ",
            " (u_u)  ",
            " \\|+|/  ",
            "  |||   ",
            " \\\\|//  ",
        ],
        ("Grave Ghoul", 0) => &[" ,---.  ", " (o_o)  ", " /VVV\\  ", "  } {   ", " _/ \\_  "],
        ("Grave Ghoul", _) => &[" ,---.  ", " (o_O)  ", " \\VVV/  ", "  } {   ", " _\\ /_  "],
        ("Barrow Sentinel", 0) => &[
            "  [====]  ",
            "  |o  o|  ",
            " /|====|\\ ",
            "  |####|  ",
            "  _|__|_  ",
        ],
        ("Barrow Sentinel", _) => &[
            "  [====]  ",
            "  |o  o|  ",
            " \\|====|/ ",
            "  |####|  ",
            "  _|__|_  ",
        ],
        ("Forsaken Knight", 0) => &[
            "   /^\\    ",
            "  |o-o|   ",
            " /|===|\\  ",
            "  |# #|   ",
            "  d| |b   ",
        ],
        ("Forsaken Knight", _) => &[
            "   /^\\    ",
            "  |o-o|   ",
            " \\|===|/  ",
            "  |# #|   ",
            "  d| |b   ",
        ],
        (_, 0) => &["  ???  ", " ????? ", "  ???  "],
        (_, _) => &[" ????? ", "  ???  ", " ????? "],
    }
}

/// Rough per-species color so sprites read differently at a glance.
pub fn color_for(species_name: &str) -> Color {
    match species_name {
        "Slime" => Color::Green,
        "Goblin" => Color::Yellow,
        "Bat" => Color::DarkGray,
        "Wolf" => Color::Gray,
        "Skeleton" => Color::White,
        "Orc" => Color::LightGreen,
        "Wraith" => Color::Magenta,
        "Mimic" => Color::Red,
        "Hollow" => Color::DarkGray,
        "Rat" => Color::Gray,
        "Carrion Crow" => Color::LightMagenta,
        "Bandit" => Color::Red,
        "Fell Acolyte" => Color::Magenta,
        "Grave Ghoul" => Color::Green,
        "Barrow Sentinel" => Color::White,
        "Forsaken Knight" => Color::Blue,
        "The Barrow Knight" => Color::LightRed,
        "Wyrmscale Warden" => Color::LightGreen,
        "The Ashen Sovereign" => Color::LightYellow,
        _ => Color::Red,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SPECIES: [&str; 20] = [
        "Slime",
        "Goblin",
        "Bat",
        "Wolf",
        "Skeleton",
        "Orc",
        "Wraith",
        "Mimic",
        "Hollow",
        "Rat",
        "Carrion Crow",
        "Bandit",
        "Fell Acolyte",
        "Grave Ghoul",
        "Barrow Sentinel",
        "Forsaken Knight",
        "The Barrow Knight",
        "Wyrmscale Warden",
        "The Ashen Sovereign",
        "Something Unknown",
    ];

    #[test]
    fn every_species_has_the_same_height_across_all_frames() {
        for species in SPECIES {
            let heights: Vec<usize> = (0..ANIM_FRAMES)
                .map(|f| sprite_for(species, f).len())
                .collect();
            assert!(
                heights.windows(2).all(|w| w[0] == w[1]),
                "{species}'s frames differ in height: {heights:?}"
            );
        }
    }

    #[test]
    fn every_line_of_every_frame_is_the_same_width() {
        // The combat screen centers each line independently, so a ragged
        // line drifts sideways — the original bug this rewrite fixes.
        for species in SPECIES {
            for frame in 0..ANIM_FRAMES {
                let sprite = sprite_for(species, frame);
                let widths: Vec<usize> = sprite.iter().map(|l| l.chars().count()).collect();
                assert!(
                    widths.windows(2).all(|w| w[0] == w[1]),
                    "{species} frame {frame} has ragged line widths: {widths:?}"
                );
            }
        }
    }

    #[test]
    fn frames_actually_differ_so_the_animation_is_visible() {
        for species in SPECIES {
            assert_ne!(
                sprite_for(species, 0),
                sprite_for(species, 1),
                "{species}'s two frames should not be identical"
            );
        }
    }

    #[test]
    fn frame_indices_wrap_around() {
        assert_eq!(sprite_for("Slime", 0), sprite_for("Slime", ANIM_FRAMES));
        assert_eq!(sprite_for("Slime", 1), sprite_for("Slime", ANIM_FRAMES + 1));
    }
}
