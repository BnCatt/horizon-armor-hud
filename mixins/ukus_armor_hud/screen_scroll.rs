// The wheel over a cycling button.
fn ukus_armor_hud_screen_scroll(game: &crate::game::Game<'_>, screen: &horizon_client::mod_api::NativeScreenView<'_>, scroll: [f64; 2], canceled: &mut bool) {
    crate::game::ukus_armor_hud::on_mouse_scrolled(game, screen, scroll, canceled);
}
