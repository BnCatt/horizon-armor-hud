// The settings page's buttons.
fn ukus_armor_hud_settings(game: &crate::game::Game<'_>, mod_id: &str, buttons: &mut Vec<String>) {
    crate::game::ukus_armor_hud::on_settings(game, mod_id, buttons);
}
