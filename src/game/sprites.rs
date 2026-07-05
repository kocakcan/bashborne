use ratatui::style::Color;

/// Small ASCII sprites for the combat screen. Keep entries narrow (~12 chars)
/// and short (~6 lines) so two or three fit comfortably side by side.
/// Keyed on the enemy's display name, which currently doubles as its species tag.
pub fn sprite_for(species_name: &str) -> &'static [&'static str] {
    match species_name {
        "Slime" => &[
            "  .--.  ",
            " /    \\ ",
            "| o  o |",
            "|  __  |",
            " \\____/ ",
            " ~~~~~~ ",
        ],
        "Goblin" => &[
            " /\\_/\\  ",
            "( o.o ) ",
            " > ^ <  ",
            "/|   |\\ ",
            " |   |  ",
            "_|   |_ ",
        ],
        "Bat" => &[
            "/\\   /\\ ",
            "(o\\ /o) ",
            " \\  V / ",
            "  \\___/ ",
            "  /   \\ ",
        ],
        "Wolf" => &[
            " /\\___/\\ ",
            "( o   o )",
            " \\  ^  / ",
            " /|---|\\ ",
            "* |   | *",
        ],
        "Skeleton" => &[
            "  .-.   ",
            " (o.o)  ",
            "  |=|   ",
            " /|-|\\  ",
            "  | |   ",
            " _| |_  ",
        ],
        "Orc" => &[
            "  ___    ",
            " /o o\\   ",
            "( >_< )  ",
            "/|   |\\  ",
            "\\|   |/  ",
            " |___|   ",
        ],
        "Wraith" => &[
            "  .:::.   ",
            " (: o o:) ",
            "  ':::.'  ",
            " /  |  \\  ",
            "((  |  ))",
        ],
        "Mimic" => &[
            " ______ ",
            "/|_||_|\\",
            "( ^  ^ )",
            "\\ \\/\\/ /",
            " \\____/ ",
        ],
        "The Barrow Knight" => &[
            "  /^^^\\   ",
            " |[o_o]|  ",
            " |=====|  ",
            "/|##|##|\\ ",
            " |  |  |  ",
            " '--'--'  ",
        ],
        "Wyrmscale Warden" => &[
            "  /\\/\\/\\  ",
            " ( o   o )",
            "  \\  ^^ / ",
            " //|VVVV|\\\\",
            "   |    |  ",
            "  //    \\\\ ",
        ],
        "The Ashen Sovereign" => &[
            "   /##\\    ",
            "  |[**]|   ",
            "  |=><=|   ",
            " /|#|##|#\\ ",
            "  |  ||  | ",
            " ^^^  ^^^^ ",
        ],
        _ => &["  ???  ", " ????? ", "  ???  "],
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
        "The Barrow Knight" => Color::LightRed,
        "Wyrmscale Warden" => Color::LightGreen,
        "The Ashen Sovereign" => Color::LightYellow,
        _ => Color::Red,
    }
}
