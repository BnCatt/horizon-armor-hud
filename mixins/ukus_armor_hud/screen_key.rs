// Editing keys in a text box.
fn ukus_armor_hud_screen_key(game: &crate::game::Game<'_>, screen: &horizon_client::mod_api::NativeScreenView<'_>, key: &str, modifiers: u32, canceled: &mut bool) {
    crate::game::ukus_armor_hud::on_key_pressed(game, screen, key, modifiers, canceled);
}
