// After a vanilla HUD layer: the armour widget (hotbar), the hands' durability (selected item name), the pushes.
fn ukus_armor_hud_layer_post(game: &crate::game::Game<'_>, layer: &str, batch: &mut horizon_client::hud::HudBatch) {
    crate::game::ukus_armor_hud::on_layer_post(game, layer, batch);
}
