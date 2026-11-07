// Typing in a text box.
fn ukus_armor_hud_screen_char(game: &crate::game::Game<'_>, screen: &horizon_client::mod_api::NativeScreenView<'_>, c: char, canceled: &mut bool) {
    crate::game::ukus_armor_hud::on_char_typed(game, screen, c, canceled);
}
