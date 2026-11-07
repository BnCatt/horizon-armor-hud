// A click on the config screen.
fn ukus_armor_hud_screen_press(game: &crate::game::Game<'_>, screen: &horizon_client::mod_api::NativeScreenView<'_>, button: i32, modifiers: u32, canceled: &mut bool) {
    crate::game::ukus_armor_hud::on_mouse_pressed(game, screen, button, modifiers, canceled);
}
