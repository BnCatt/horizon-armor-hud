// Before a vanilla HUD layer: where the pushed layers (effects, boss bars, subtitles) start.
fn ukus_armor_hud_layer_pre(game: &crate::game::Game<'_>, layer: &str, batch: &mut horizon_client::hud::HudBatch) {
    crate::game::ukus_armor_hud::on_layer_pre(game, layer, batch);
}
