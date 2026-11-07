// The config screen.
fn ukus_armor_hud_screen_render(game: &crate::game::Game<'_>, batch: &mut horizon_client::hud::HudBatch, screen: &horizon_client::mod_api::NativeScreenView<'_>) {
    crate::game::ukus_armor_hud::on_screen_render(game, batch, screen);
}
