// The row in Options > Mods.
fn ukus_armor_hud_mods(mods: &mut Vec<horizon_client::mod_api::NativeMod>) {
    crate::game::ukus_armor_hud::on_mods(mods);
}
