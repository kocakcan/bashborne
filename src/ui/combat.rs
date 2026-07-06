use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph, Wrap};
use ratatui::Frame;

use crate::game::character::AbilityKind;
use crate::game::combat::{ActorRef, CombatAction, CombatPhase, CombatState};
use crate::game::item::Inventory;
use crate::game::party::Party;
use crate::game::sprites;

const ACTION_LABELS: [&str; 4] = ["Attack", "Ability", "Item", "Flee"];

pub fn draw(
    frame: &mut Frame,
    area: Rect,
    combat: &CombatState,
    party: &Party,
    inventory: &Inventory,
    anim_frame: usize,
) {
    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(40),
            Constraint::Percentage(35),
            Constraint::Percentage(25),
        ])
        .split(area);

    draw_enemies(frame, outer[0], combat, party, anim_frame);

    let mid = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(outer[1]);
    draw_party(frame, mid[0], party, combat);
    draw_menu_or_result(frame, mid[1], combat, party, inventory);

    draw_log(frame, outer[2], &combat.log, combat.log_scroll);
}

/// Whether a pending action targets the party (heals/items) rather than the
/// enemy line-up. Ability actions depend on *which* ability the acting
/// character has, not just the action variant, so we look it up here rather
/// than assuming all abilities are offensive or all are healing.
fn targets_party(action: CombatAction, actor_idx: usize, party: &Party) -> bool {
    match action {
        CombatAction::Item(_) => true, // current items (potion/ether) are always party-targeted
        CombatAction::Ability(idx) => party
            .members
            .get(actor_idx)
            .map(|m| m.ability_is_heal(idx))
            .unwrap_or(false),
        CombatAction::Attack | CombatAction::Flee => false,
    }
}

fn draw_enemies(
    frame: &mut Frame,
    area: Rect,
    combat: &CombatState,
    party: &Party,
    anim_frame: usize,
) {
    let target_idx = match combat.phase {
        CombatPhase::SelectTarget {
            actor: ActorRef::Player(pi),
            action,
            target_idx,
        } if !targets_party(action, pi, party) => Some(target_idx),
        _ => None,
    };

    let outer_block = Block::default().borders(Borders::ALL).title("Enemies");
    let inner = outer_block.inner(area);
    frame.render_widget(outer_block, area);

    if combat.enemies.is_empty() {
        return;
    }

    let constraints: Vec<Constraint> = combat
        .enemies
        .iter()
        .map(|_| Constraint::Ratio(1, combat.enemies.len() as u32))
        .collect();
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(constraints)
        .split(inner);

    for (i, enemy) in combat.enemies.iter().enumerate() {
        let species_color = sprites::color_for(&enemy.name);
        let species_color = if enemy.is_elite {
            sprites::elite_tint(species_color)
        } else {
            species_color
        };
        let is_targeted = target_idx == Some(i) && enemy.is_alive();
        let mut lines: Vec<Line> = Vec::new();

        if enemy.is_alive() {
            for sprite_line in sprites::sprite_for(&enemy.name, anim_frame) {
                lines.push(Line::from(Span::styled(
                    sprite_line.to_string(),
                    Style::default().fg(species_color),
                )));
            }
            lines.push(Line::from(""));
            let marker = if is_targeted { "> " } else { "  " };
            let name_style = if is_targeted {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::White)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().add_modifier(Modifier::BOLD)
            };
            lines.push(Line::from(Span::styled(
                format!("{marker}{}", enemy.display_name()),
                name_style,
            )));
            let bar = crate::ui::hp_bar(enemy.stats.hp, enemy.stats.max_hp, 12);
            let bar_color = crate::ui::hp_color(enemy.hp_ratio());
            lines.push(Line::from(vec![
                Span::styled(bar, Style::default().fg(bar_color)),
                Span::raw(format!(" {}/{}", enemy.stats.hp, enemy.stats.max_hp)),
            ]));
        } else {
            // Keep the column height stable even once an enemy is down.
            // (Both frames of a sprite are the same height, so frame 0 works.)
            for _ in 0..sprites::sprite_for(&enemy.name, 0).len() {
                lines.push(Line::from(""));
            }
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                format!("{} (defeated)", enemy.display_name()),
                Style::default()
                    .fg(Color::DarkGray)
                    .add_modifier(Modifier::CROSSED_OUT),
            )));
        }

        let p = Paragraph::new(lines).alignment(Alignment::Center);
        frame.render_widget(p, cols[i]);
    }
}

fn draw_party(frame: &mut Frame, area: Rect, party: &Party, combat: &CombatState) {
    let acting_player = match combat.phase {
        CombatPhase::SelectAction {
            actor: ActorRef::Player(i),
        } => Some(i),
        CombatPhase::SelectTarget {
            actor: ActorRef::Player(i),
            ..
        } => Some(i),
        _ => None,
    };
    let party_target = match combat.phase {
        CombatPhase::SelectTarget {
            actor: ActorRef::Player(pi),
            action,
            target_idx,
        } if targets_party(action, pi, party) => Some(target_idx),
        _ => None,
    };

    let mut items: Vec<ListItem> = party
        .members
        .iter()
        .enumerate()
        .map(|(i, m)| {
            let mut marker = "  ".to_string();
            if acting_player == Some(i) {
                marker = "* ".to_string();
            }
            if party_target == Some(i) {
                marker = "> ".to_string();
            }
            let hp_color = if !m.is_alive() {
                Color::DarkGray
            } else {
                crate::ui::hp_color(m.hp_ratio())
            };
            let bar = crate::ui::hp_bar(m.stats.hp, m.stats.max_hp, 10);
            let line = Line::from(vec![
                Span::raw(marker),
                Span::styled(
                    format!("{:<8}", m.name),
                    Style::default().add_modifier(Modifier::BOLD),
                ),
                Span::styled(format!(" {bar} "), Style::default().fg(hp_color)),
                Span::raw(format!("{:>3}/{:<3}", m.stats.hp, m.stats.max_hp)),
                Span::raw(format!("  MP {:>3}/{:<3}", m.stats.mp, m.stats.max_mp)),
            ]);
            ListItem::new(line)
        })
        .collect();

    if !party.effects.is_empty() {
        items.push(ListItem::new(Line::from(Span::styled(
            "──────────",
            Style::default().fg(Color::DarkGray),
        ))));
        for e in &party.effects {
            let color = if e.delta >= 0 {
                Color::Cyan
            } else {
                Color::Magenta
            };
            items.push(ListItem::new(Line::from(Span::styled(
                format!(
                    "{} ({:+} {}, {} left)",
                    e.name, e.delta, e.target, e.encounters_remaining
                ),
                Style::default().fg(color),
            ))));
        }
    }
    let block = Block::default().borders(Borders::ALL).title("Party");
    frame.render_widget(List::new(items).block(block), area);
}

fn draw_menu_or_result(
    frame: &mut Frame,
    area: Rect,
    combat: &CombatState,
    party: &Party,
    inventory: &Inventory,
) {
    match combat.phase {
        CombatPhase::SelectAction {
            actor: ActorRef::Player(_),
        } => {
            let items: Vec<ListItem> = ACTION_LABELS
                .iter()
                .enumerate()
                .map(|(i, label)| {
                    let style = if i == combat.menu_cursor {
                        Style::default()
                            .fg(Color::Black)
                            .bg(Color::White)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default()
                    };
                    ListItem::new(Line::from(Span::styled(*label, style)))
                })
                .collect();
            let block = Block::default()
                .borders(Borders::ALL)
                .title("Choose action (↑↓ Enter)");
            frame.render_widget(List::new(items).block(block), area);
        }
        CombatPhase::SelectAbility {
            actor: ActorRef::Player(pi),
            cursor,
        } => {
            let items: Vec<ListItem> = party.members[pi]
                .abilities
                .iter()
                .enumerate()
                .map(|(i, ability)| {
                    let affordable = party.members[pi].stats.mp >= ability.mp_cost;
                    let selected = i == cursor;
                    let label = format!("{} (MP {})", ability.name, ability.mp_cost);
                    let header_style = if selected {
                        Style::default()
                            .fg(Color::Black)
                            .bg(Color::White)
                            .add_modifier(Modifier::BOLD)
                    } else if !affordable {
                        Style::default().fg(Color::DarkGray)
                    } else {
                        Style::default()
                    };
                    let header = Line::from(Span::styled(label, header_style));
                    let power = ability.effective_power(&party.members[pi]);
                    let description = match ability.kind {
                        AbilityKind::PhysicalDamage | AbilityKind::MagicDamage => {
                            if ability.targets_all_enemies {
                                format!("     Deals ~{power} damage to all enemies")
                            } else {
                                format!("     Deals ~{power} damage to one enemy")
                            }
                        }
                        AbilityKind::Heal => {
                            format!("     Heals {power} HP to one ally")
                        }
                    };
                    let detail_style = if selected {
                        Style::default().fg(Color::White)
                    } else {
                        Style::default().fg(Color::DarkGray)
                    };
                    let detail = Line::from(Span::styled(description, detail_style));
                    ListItem::new(Text::from(vec![header, detail]))
                })
                .collect();
            let block = Block::default()
                .borders(Borders::ALL)
                .title("Choose ability (↑↓ Enter, Esc back)");
            frame.render_widget(List::new(items).block(block), area);
        }
        CombatPhase::SelectItem {
            actor: ActorRef::Player(_),
            cursor,
        } => {
            let items: Vec<ListItem> = inventory
                .items
                .iter()
                .enumerate()
                .map(|(i, (item, qty))| {
                    let selected = i == cursor;
                    let label = format!("{} x{qty}", item.name);
                    let header_style = if selected {
                        Style::default()
                            .fg(Color::Black)
                            .bg(Color::White)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default()
                    };
                    let header = Line::from(Span::styled(label, header_style));
                    let detail_style = if selected {
                        Style::default().fg(Color::White)
                    } else {
                        Style::default().fg(Color::DarkGray)
                    };
                    let detail = Line::from(Span::styled(
                        format!("     {}", item.description),
                        detail_style,
                    ));
                    ListItem::new(Text::from(vec![header, detail]))
                })
                .collect();
            let block = Block::default()
                .borders(Borders::ALL)
                .title("Choose item (↑↓ Enter, Esc back)");
            if items.is_empty() {
                frame.render_widget(Paragraph::new("No items.").block(block), area);
            } else {
                frame.render_widget(List::new(items).block(block), area);
            }
        }
        CombatPhase::SelectTarget { action, .. } => {
            let hint = match action {
                CombatAction::Item(idx) => {
                    let item_name = inventory
                        .items
                        .get(idx)
                        .map(|(i, qty)| format!("{} x{qty}", i.name))
                        .unwrap_or_else(|| "no items".to_string());
                    format!("Using: {item_name}\n←→ target, Enter confirm, Esc back")
                }
                _ => "←→ to change target, Enter to confirm, Esc to go back".to_string(),
            };
            let block = Block::default().borders(Borders::ALL).title("Target");
            frame.render_widget(Paragraph::new(hint).block(block), area);
        }
        CombatPhase::Victory => {
            let block = Block::default().borders(Borders::ALL).title("Result");
            let mut lines = vec![Line::from(Span::styled(
                "Victory!",
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ))];
            if let Some(loot) = &combat.loot {
                if loot.gold > 0 {
                    lines.push(Line::from(format!("Found {} gold.", loot.gold)));
                }
                if loot.overkill_bonus > 0 {
                    lines.push(Line::from(Span::styled(
                        format!("Overkill bonus: +{} gold!", loot.overkill_bonus),
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                    )));
                }
                for item in &loot.items {
                    lines.push(Line::from(format!("Found: {}", item.name)));
                }
                for weapon in &loot.weapons {
                    let color = crate::ui::rarity_color(weapon.rarity);
                    lines.push(Line::from(Span::styled(
                        format!("Found weapon: {} [{}]", weapon.name, weapon.rarity),
                        Style::default().fg(color).add_modifier(Modifier::BOLD),
                    )));
                }
                for armor in &loot.armors {
                    let color = crate::ui::rarity_color(armor.rarity);
                    lines.push(Line::from(Span::styled(
                        format!("Found armor: {} [{}]", armor.name, armor.rarity),
                        Style::default().fg(color).add_modifier(Modifier::BOLD),
                    )));
                }
                for ring in &loot.rings {
                    let color = crate::ui::rarity_color(ring.rarity);
                    lines.push(Line::from(Span::styled(
                        format!("Found ring: {} [{}]", ring.name, ring.rarity),
                        Style::default().fg(color).add_modifier(Modifier::BOLD),
                    )));
                }
                if loot.gold == 0
                    && loot.items.is_empty()
                    && loot.weapons.is_empty()
                    && loot.armors.is_empty()
                    && loot.rings.is_empty()
                {
                    lines.push(Line::from("No loot this time."));
                }
            }
            lines.push(Line::from(""));
            lines.push(Line::from("Press Enter to continue."));
            frame.render_widget(Paragraph::new(lines).block(block), area);
        }
        CombatPhase::Defeat => {
            let block = Block::default().borders(Borders::ALL).title("Result");
            frame.render_widget(
                Paragraph::new("Your party has fallen. Press Enter.").block(block),
                area,
            );
        }
        CombatPhase::Fled => {
            let block = Block::default().borders(Borders::ALL).title("Result");
            frame.render_widget(
                Paragraph::new("You got away safely. Press Enter.").block(block),
                area,
            );
        }
        CombatPhase::Resolving
        | CombatPhase::SelectAction {
            actor: ActorRef::Enemy(_),
        }
        | CombatPhase::SelectAbility {
            actor: ActorRef::Enemy(_),
            ..
        }
        | CombatPhase::SelectItem {
            actor: ActorRef::Enemy(_),
            ..
        } => {
            let block = Block::default().borders(Borders::ALL).title("...");
            frame.render_widget(Paragraph::new("The enemy is acting...").block(block), area);
        }
    }
}

fn draw_log(frame: &mut Frame, area: Rect, log: &[String], scroll: usize) {
    let visible_rows = area.height.saturating_sub(2) as usize;
    let scroll = scroll.min(log.len().saturating_sub(visible_rows.max(1)));
    let end = log.len().saturating_sub(scroll);
    let start = end.saturating_sub(visible_rows.max(1));
    let lines: Vec<Line> = log[start..end]
        .iter()
        .map(|s| Line::from(s.as_str()))
        .collect();
    let title = if scroll > 0 {
        format!("Battle Log (PageDown for latest, {scroll} back)")
    } else {
        "Battle Log".to_string()
    };
    let block = Block::default().borders(Borders::ALL).title(title);
    let p = Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: false });
    frame.render_widget(p, area);
}
