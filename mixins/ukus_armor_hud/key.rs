// The "Toggle Armor Hud" and "Open Config Screen" keys.
fn ukus_armor_hud_key(game: &mut crate::game::Game<'_>, key: &str, pressed: bool) {
    crate::game::ukus_armor_hud::on_key(game, key, pressed);
}
