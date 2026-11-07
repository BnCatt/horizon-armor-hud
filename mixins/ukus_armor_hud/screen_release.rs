// The end of a slider drag.
fn ukus_armor_hud_screen_release(game: &crate::game::Game<'_>, screen: &horizon_client::mod_api::NativeScreenView<'_>, canceled: &mut bool) {
    crate::game::ukus_armor_hud::on_mouse_released(game, screen, canceled);
}
