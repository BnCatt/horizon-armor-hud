// uku's Armor HUD: the ONLY part of the mod that reads Horizon internals.
//
// Horizon's numbered hook API (411) has no client read of the local player's
// armour, off hand, selected slot, main arm, attack-indicator charge or an
// item's durability. This module is a child of `game` (game.rs), so it can
// read `Game`'s private fields. If a later Nightly renames one of the fields
// below, the build breaks HERE and nowhere else:
//   Game.player, Game.armor, Game.offhand, Game.inventory, Game.selected,
//   Game.settings.{main_hand_left, attack_indicator, extras.pvp.combat,
//   master_volume}, Game.hand_strength, Game::main_hand_attack_speed(),
//   Game::spectator(), Game::items(), horizon_client::stations::is_damageable,
//   horizon_client::settings::{source_volume, SOUND_SOURCES}.
// Once Horizon numbers its `equipment` helper (host_bridge fragment), the
// armour reads can move to it.
use super::Game;
use horizon_client::items::ItemStack;

/// What the HUD needs of the local player this frame.
pub(super) struct View {
    /// HEAD, CHEST, LEGS, FEET (`InventoryMenu.SLOT_IDS`); None = empty.
    pub armor: [Option<ItemStack>; 4],
    pub offhand: Option<ItemStack>,
    pub mainhand: Option<ItemStack>,
    /// `Inventory.getSelectedSlot`.
    pub selected: usize,
    /// `Player.getMainArm() == HumanoidArm.LEFT`.
    pub main_arm_left: bool,
    /// `options.attackIndicator == HOTBAR && getAttackStrengthScale(0) < 1`.
    pub hotbar_indicator_charging: bool,
    /// Spectator mode: vanilla draws the spectator bar instead of the hotbar.
    pub spectator: bool,
}

fn present(s: &Option<ItemStack>) -> Option<ItemStack> {
    s.as_ref()
        .filter(|s| s.count > 0 && &*s.item != "minecraft:air")
        .cloned()
}

/// The local player, None when no player is in the world.
pub(super) fn view(g: &Game<'_>) -> Option<View> {
    g.player.as_ref()?;
    // Same as Horizon's own hotbar: Combat feel Legacy hides the indicator
    // unless the player keeps it.
    let combat = g.settings.extras.pvp.combat;
    let indicator = if combat.legacy && !combat.indicator {
        0
    } else {
        g.settings.attack_indicator
    };
    let strength = g.hand_strength.scale(0.0, g.main_hand_attack_speed());
    Some(View {
        armor: [
            present(&g.armor[0]),
            present(&g.armor[1]),
            present(&g.armor[2]),
            present(&g.armor[3]),
        ],
        offhand: present(&g.offhand),
        mainhand: g.inventory.get(g.selected).and_then(present),
        selected: g.selected,
        main_arm_left: g.settings.main_hand_left,
        hotbar_indicator_charging: indicator == 2 && strength < 1.0,
        spectator: g.spectator(),
    })
}

/// `(getDamageValue, getMaxDamage)` when `isDamageableItem`, else None.
pub(super) fn durability(g: &Game<'_>, s: &ItemStack) -> Option<(i32, i32)> {
    let items = g.items();
    if !horizon_client::stations::is_damageable(items, s) {
        return None;
    }
    let max = items.get(&s.item).map_or(0, |i| i.max_damage as i32);
    (max > 0).then_some((s.damage as i32, max))
}

/// The volume a `SoundSource.PLAYERS` sound plays at: master times players.
pub(super) fn players_volume(g: &Game<'_>) -> f32 {
    let i = horizon_client::settings::SOUND_SOURCES
        .iter()
        .position(|s| *s == "player")
        .unwrap_or(0);
    g.settings.master_volume * horizon_client::settings::source_volume(i)
}
