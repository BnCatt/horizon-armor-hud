// uku's Armor HUD for Horizon: a port of uku3lig/armor-hud 0.14.0+mc26.3
// (MIT, Copyright (c) 2021 BerdinskiyBear; fork by uku). Every number below
// is the Java mod's own (ArmorHudMod, ArmorHudRenderer, ArmorHudConfig,
// ArmorHudConfigScreen and ukulib's TabbedConfigScreen).
//
// Only `ukus_armor_hud_peek` reads Horizon internals; everything here goes
// through numbered hook points and `Game::native_*` helpers (API 411).
//
// The MIT License (MIT)
// Copyright (c) 2021 BerdinskiyBear (uku's Armor HUD: design, code, warn.png
// and armor_breaking.ogg); Copyright (c) 2026 Horizon port contributors.
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in
// all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
// THE SOFTWARE.
use super::Game;
use super::ukus_armor_hud_peek as peek;
use horizon_client::hud::HudBatch;
use horizon_client::items::ItemStack;
use horizon_client::mod_api::{NativeMod, NativeScreenView};
use horizon_client::text::Text;
use std::sync::{Mutex, MutexGuard};
use std::time::Instant;

pub(super) const ID: &str = "ukus_armor_hud";
const NAME: &str = "uku's Armor HUD";
const VERSION: &str = "0.14.0";
const DESCRIPTION: &str = "This mod adds a minimalistic vanilla-like widget that shows your armor items and a warning if durability is low enough. Forked by uku. Ported to Horizon.";

// ArmorHudMod
const STEP: i32 = 20;
const SIZE: i32 = 22;
const HOTBAR_OFFSET: i32 = 98;
const OFFHAND_OFFSET: i32 = 29;
const ATTACK_INDICATOR_OFFSET: i32 = 23;
const WARNING_SIZE: i32 = 8;
/// `Font.lineHeight`.
const LINE_HEIGHT: i32 = 9;

const HOTBAR_SPRITE: &str = "gui/sprites/hud/hotbar";
const HOTBAR_OFFHAND_LEFT_SPRITE: &str = "gui/sprites/hud/hotbar_offhand_left";
/// `InventoryMenu.TEXTURE_EMPTY_SLOTS` by `SLOT_IDS` (HEAD, CHEST, LEGS, FEET).
const EMPTY_SLOTS: [&str; 4] = [
    "gui/sprites/container/slot/helmet",
    "gui/sprites/container/slot/chestplate",
    "gui/sprites/container/slot/leggings",
    "gui/sprites/container/slot/boots",
];

/// `assets/ukus-armor-hud/warn.png` (8x8 RGBA), pixel for pixel, as ARGB.
const WARN: [[u32; 8]; 8] = [
    [0, 0, 0xFF9D0D0D, 0xFF9D0D0D, 0xFF9D0D0D, 0xFF9D0D0D, 0, 0],
    [0, 0xFFE4B9B9, 0xFFDD3232, 0xFFEABDBD, 0xFFD22323, 0xFFE4B9B9, 0xFF8C0B0B, 0],
    [0xFF9D0D0D, 0xFFEABDBD, 0xFFD22323, 0xFFE4B9B9, 0xFFB90F0F, 0xFFE4B9B9, 0xFFB90F0F, 0xFF740A0A],
    [0xFF9D0D0D, 0xFFEABDBD, 0xFFB90F0F, 0xFFE4B9B9, 0xFFB90F0F, 0xFFE4B9B9, 0xFFB90F0F, 0xFF740A0A],
    [0xFF9D0D0D, 0xFFD22323, 0xFFB90F0F, 0xFFB90F0F, 0xFFB90F0F, 0xFFB90F0F, 0xFFB90F0F, 0xFF740A0A],
    [0xFF9D0D0D, 0xFFE4B9B9, 0xFFB90F0F, 0xFFE4B9B9, 0xFFB90F0F, 0xFFE4B9B9, 0xFFB90F0F, 0xFF740A0A],
    [0, 0xFF8C0B0B, 0xFFB90F0F, 0xFFB90F0F, 0xFFB90F0F, 0xFFB90F0F, 0xFF740A0A, 0],
    [0, 0, 0xFF740A0A, 0xFF740A0A, 0xFF740A0A, 0xFF740A0A, 0, 0],
];

// ---------------------------------------------------------------- config

trait Choice: Copy + PartialEq + 'static {
    const ALL: &'static [Self];
    /// The name the Java config writes (`StringTranslatable.getName`).
    fn key(self) -> &'static str;
    fn label(self) -> String;
    fn step(self, by: i32) -> Self {
        let all = Self::ALL;
        let i = all.iter().position(|c| *c == self).unwrap_or(0) as i32;
        all[(i + by).rem_euclid(all.len() as i32) as usize]
    }
    fn parse(s: &str) -> Option<Self> {
        Self::ALL.iter().copied().find(|c| c.key() == s)
    }
}

macro_rules! choice {
    ($name:ident { $($v:ident => $key:literal, $label:expr;)* }) => {
        #[derive(Clone, Copy, PartialEq, Eq, Debug)]
        pub(super) enum $name { $($v,)* }
        impl Choice for $name {
            const ALL: &'static [Self] = &[$(Self::$v,)*];
            fn key(self) -> &'static str { match self { $(Self::$v => $key,)* } }
            fn label(self) -> String { match self { $(Self::$v => $label,)* } }
        }
    };
}

fn tr(key: &str) -> String {
    Text::translate(key, &[]).plain()
}

choice!(Anchor {
    TopCenter => "top_center", "Top Center".into();
    Top => "top", "Top".into();
    VertCenter => "vertical_center", "Vertical Center".into();
    Bottom => "bottom", "Bottom".into();
    Hotbar => "hotbar", "Hotbar".into();
});
choice!(Side {
    Left => "left", tr("options.mainHand.left");
    Right => "right", tr("options.mainHand.right");
});
choice!(OffhandSlotBehavior {
    AlwaysIgnore => "always_ignore", "Always Ignore".into();
    Adhere => "adhere", "Adhere".into();
    AlwaysLeaveSpace => "always_leave_space", "Always Leave Space".into();
});
choice!(WidgetShown {
    Always => "always", "Always".into();
    IfAnyPresent => "if_any_present", "If Any Present".into();
    NotEmpty => "not_empty", "Not Empty".into();
    DamagedPieces => "damaged_pieces", "Damaged Pieces".into();
});
choice!(Style {
    Hotbar => "hotbar", "Hotbar".into();
    RoundedCorners => "rounded_corners", "Rounded Corners".into();
    Rounded => "rounded", "Rounded".into();
    None => "none", "None".into();
});
choice!(Orientation {
    Horizontal => "horizontal", "Horizontal".into();
    Vertical => "vertical", "Vertical".into();
});
choice!(DurabilityDisplay {
    Bar => "bar", "Bar".into();
    Numeric => "numeric", "Numeric".into();
    Percentage => "percentage", "Percentage".into();
});

impl Anchor {
    fn is_top(self) -> bool {
        matches!(self, Anchor::Top | Anchor::TopCenter)
    }
}
impl Side {
    fn opposite(self) -> Side {
        if self == Side::Left { Side::Right } else { Side::Left }
    }
    /// `asArm() == LEFT`.
    fn left(self) -> bool {
        self == Side::Left
    }
}
impl WidgetShown {
    fn draws_empty_slots(self) -> bool {
        matches!(self, WidgetShown::Always | WidgetShown::IfAnyPresent)
    }
}

/// `ArmorHudConfig`, with its defaults.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Config {
    enabled: bool,
    anchor: Anchor,
    side: Side,
    offset_x: i32,
    offset_y: i32,
    style: Style,
    orientation: Orientation,
    widget_shown: WidgetShown,
    offhand_slot_behavior: OffhandSlotBehavior,
    durability_display: DurabilityDisplay,
    off_hand_durability: bool,
    main_hand_durability: bool,
    push_bossbars: bool,
    push_status_effect_icons: bool,
    push_subtitles: bool,
    reversed: bool,
    icons_shown: bool,
    warning_shown: bool,
    play_break_sound: bool,
    min_durability_value: i32,
    min_durability_percentage: f64,
    warning_bob_intensity: i32,
}

const DEFAULT: Config = Config {
    enabled: true,
    anchor: Anchor::Hotbar,
    side: Side::Left,
    offset_x: 0,
    offset_y: 0,
    style: Style::Hotbar,
    orientation: Orientation::Horizontal,
    widget_shown: WidgetShown::NotEmpty,
    offhand_slot_behavior: OffhandSlotBehavior::Adhere,
    durability_display: DurabilityDisplay::Bar,
    off_hand_durability: true,
    main_hand_durability: false,
    push_bossbars: true,
    push_status_effect_icons: true,
    push_subtitles: true,
    reversed: false,
    icons_shown: true,
    warning_shown: true,
    play_break_sound: true,
    min_durability_value: 20,
    min_durability_percentage: 0.1,
    warning_bob_intensity: 3,
};

impl Config {
    fn load(read: impl Fn(&str) -> Option<String>) -> Config {
        let d = DEFAULT;
        let b = |k: &str, v: bool| read(k).and_then(|s| s.parse().ok()).unwrap_or(v);
        let i = |k: &str, v: i32| read(k).and_then(|s| s.parse().ok()).unwrap_or(v);
        fn c<T: Choice>(read: &impl Fn(&str) -> Option<String>, k: &str, v: T) -> T {
            read(k).and_then(|s| T::parse(&s)).unwrap_or(v)
        }
        Config {
            enabled: b("enabled", d.enabled),
            anchor: c(&read, "anchor", d.anchor),
            side: c(&read, "side", d.side),
            offset_x: i("offsetX", d.offset_x),
            offset_y: i("offsetY", d.offset_y),
            style: c(&read, "style", d.style),
            orientation: c(&read, "orientation", d.orientation),
            widget_shown: c(&read, "widgetShown", d.widget_shown),
            offhand_slot_behavior: c(&read, "offhandSlotBehavior", d.offhand_slot_behavior),
            durability_display: c(&read, "durabilityDisplay", d.durability_display),
            off_hand_durability: b("offHandDurability", d.off_hand_durability),
            main_hand_durability: b("mainHandDurability", d.main_hand_durability),
            push_bossbars: b("pushBossbars", d.push_bossbars),
            push_status_effect_icons: b("pushStatusEffectIcons", d.push_status_effect_icons),
            push_subtitles: b("pushSubtitles", d.push_subtitles),
            reversed: b("reversed", d.reversed),
            icons_shown: b("iconsShown", d.icons_shown),
            warning_shown: b("warningShown", d.warning_shown),
            play_break_sound: b("playBreakSound", d.play_break_sound),
            min_durability_value: i("minDurabilityValue", d.min_durability_value),
            min_durability_percentage: read("minDurabilityPercentage")
                .and_then(|s| s.parse::<f64>().ok())
                .filter(|v| v.is_finite())
                .unwrap_or(d.min_durability_percentage),
            warning_bob_intensity: i("warningBobIntensity", d.warning_bob_intensity),
        }
    }
    fn entries(&self) -> Vec<(&'static str, String)> {
        vec![
            ("enabled", self.enabled.to_string()),
            ("anchor", self.anchor.key().into()),
            ("side", self.side.key().into()),
            ("offsetX", self.offset_x.to_string()),
            ("offsetY", self.offset_y.to_string()),
            ("style", self.style.key().into()),
            ("orientation", self.orientation.key().into()),
            ("widgetShown", self.widget_shown.key().into()),
            ("offhandSlotBehavior", self.offhand_slot_behavior.key().into()),
            ("durabilityDisplay", self.durability_display.key().into()),
            ("offHandDurability", self.off_hand_durability.to_string()),
            ("mainHandDurability", self.main_hand_durability.to_string()),
            ("pushBossbars", self.push_bossbars.to_string()),
            ("pushStatusEffectIcons", self.push_status_effect_icons.to_string()),
            ("pushSubtitles", self.push_subtitles.to_string()),
            ("reversed", self.reversed.to_string()),
            ("iconsShown", self.icons_shown.to_string()),
            ("warningShown", self.warning_shown.to_string()),
            ("playBreakSound", self.play_break_sound.to_string()),
            ("minDurabilityValue", self.min_durability_value.to_string()),
            ("minDurabilityPercentage", self.min_durability_percentage.to_string()),
            ("warningBobIntensity", self.warning_bob_intensity.to_string()),
        ]
    }
}

// ----------------------------------------------------------------- state

struct State {
    cfg: Option<Config>,
    /// The first vertex of the effects, boss bar and subtitle layers.
    layer_start: [Option<usize>; 3],
    /// `Hud.random` stand-in for the warning icon's bob.
    rng: u64,
    /// `ArmorHudMod.lastStacks` (HEAD, CHEST, LEGS, FEET, MAINHAND, OFFHAND).
    last: [Option<ItemStack>; 6],
    ui: Option<Ui>,
    /// Done or Reset was pressed: close the screen on the next frame.
    close: bool,
    /// "Open config screen" was asked for from Options: open it once playing.
    open: bool,
    /// A widget was pressed: play the UI click on the next frame.
    sound: bool,
}

static STATE: Mutex<State> = Mutex::new(State {
    cfg: None,
    layer_start: [None; 3],
    rng: 0x9E37_79B9_7F4A_7C15,
    last: [None, None, None, None, None, None],
    ui: None,
    close: false,
    open: false,
    sound: false,
});

fn st() -> MutexGuard<'static, State> {
    STATE.lock().unwrap_or_else(|e| e.into_inner())
}

fn config(g: &Game<'_>) -> Config {
    let mut s = st();
    if s.cfg.is_none() {
        s.cfg = Some(Config::load(|k| g.native_setting(ID, k)));
    }
    s.cfg.unwrap_or(DEFAULT)
}

fn save(g: &Game<'_>, cfg: Config) {
    st().cfg = Some(cfg);
    for (k, v) in cfg.entries() {
        if g.native_setting(ID, k).as_deref() != Some(v.as_str()) {
            g.native_set_setting(ID, k, &v);
        }
    }
}

/// `RandomSource.nextInt(bound)` stand-in (xorshift64*).
fn next_int(bound: i32) -> i32 {
    let mut s = st();
    let mut x = s.rng;
    x ^= x >> 12;
    x ^= x << 25;
    x ^= x >> 27;
    s.rng = x;
    ((x.wrapping_mul(0x2545_F491_4F6C_DD1D) >> 33) % bound as u64) as i32
}

/// `y += random.nextInt(intensity) - ceil(intensity / 2F)`. Java throws on a
/// negative intensity; here it just doesn't bob.
fn bob(intensity: i32) -> i32 {
    if intensity <= 0 {
        return 0;
    }
    next_int(intensity) - (intensity as f32 / 2.0).ceil() as i32
}

// -------------------------------------------------------------- durability

/// `Mth.hsvToRgb` (Horizon's `hsv_to_argb`), opaque.
fn hsv(hue: f32, saturation: f32, value: f32) -> u32 {
    let h = (hue * 6.0) as i32 % 6;
    let f = hue * 6.0 - h as f32;
    let p = value * (1.0 - saturation);
    let q = value * (1.0 - f * saturation);
    let t = value * (1.0 - (1.0 - f) * saturation);
    let (r, g, b) = match h {
        0 => (value, t, p),
        1 => (q, value, p),
        2 => (p, value, t),
        3 => (p, q, value),
        4 => (t, p, value),
        _ => (value, p, q),
    };
    let c = |v: f32| ((v * 255.0) as i32).clamp(0, 255) as u32;
    (c(r) << 16) | (c(g) << 8) | c(b)
}

/// `ItemStack.getBarColor`.
fn bar_color(damage: i32, max: i32) -> u32 {
    let f = ((max as f32 - damage as f32) / max as f32).max(0.0);
    hsv(f / 3.0, 1.0, 1.0)
}

/// `ArmorHudMod.shouldShowWarning`.
fn should_warn(cfg: &Config, g: &Game<'_>, s: &ItemStack) -> bool {
    let Some((damage, max)) = peek::durability(g, s) else {
        return false;
    };
    let percentage = 1.0 - damage as f64 / max as f64;
    percentage <= cfg.min_durability_percentage || max - damage <= cfg.min_durability_value
}

/// `ArmorHudMod.getDurabilityText`.
fn durability_text(cfg: &Config, damage: i32, max: i32) -> String {
    match cfg.durability_display {
        DurabilityDisplay::Percentage => {
            if damage == 0 {
                return String::new();
            }
            let p = 1.0 - damage as f64 / max as f64;
            format!("{}%", (p * 100.0).floor() as i32)
        }
        _ => (max - damage).to_string(),
    }
}

// ------------------------------------------------------------------ layout

#[derive(Clone, Copy, Debug, PartialEq)]
struct Rect {
    x: i32,
    y: i32,
    w: i32,
    h: i32,
}

/// `ArmorHudMod.getArmorItems` (None = an empty slot).
fn armor_items(cfg: &Config, g: &Game<'_>, v: &peek::View) -> Vec<Option<ItemStack>> {
    let all = v.armor.clone();
    match cfg.widget_shown {
        WidgetShown::Always => all.to_vec(),
        WidgetShown::IfAnyPresent => {
            if all.iter().all(Option::is_none) {
                Vec::new()
            } else {
                all.to_vec()
            }
        }
        WidgetShown::NotEmpty => all.into_iter().filter(Option::is_some).collect(),
        WidgetShown::DamagedPieces => all
            .into_iter()
            .filter(|s| s.as_ref().is_some_and(|s| should_warn(cfg, g, s)))
            .collect(),
    }
}

/// `ArmorHudMod.getWidgetRect`.
fn widget_rect(cfg: &Config, v: &peek::View, count: usize, gw: i32, gh: i32) -> Option<Rect> {
    if count == 0 {
        return None;
    }
    let (side_mul, side_offset_mul) = if (cfg.anchor == Anchor::Hotbar && cfg.side == Side::Left)
        || (cfg.anchor != Anchor::Hotbar && cfg.side == Side::Right)
    {
        (-1, -1)
    } else {
        (1, 0)
    };
    let main_arm_on_side = v.main_arm_left == cfg.side.left();
    let added = match cfg.offhand_slot_behavior {
        OffhandSlotBehavior::AlwaysIgnore => 0,
        OffhandSlotBehavior::AlwaysLeaveSpace => {
            if main_arm_on_side { ATTACK_INDICATOR_OFFSET } else { OFFHAND_OFFSET }
        }
        OffhandSlotBehavior::Adhere => {
            if main_arm_on_side {
                if v.hotbar_indicator_charging { ATTACK_INDICATOR_OFFSET } else { 0 }
            } else if v.offhand.is_some() {
                OFFHAND_OFFSET
            } else {
                0
            }
        }
    };
    let texture_width = SIZE + (count as i32 - 1) * STEP;
    let vertical = cfg.orientation == Orientation::Vertical;
    let (w, h) = if vertical { (SIZE, texture_width) } else { (texture_width, SIZE) };
    let x = cfg.offset_x * side_mul
        + match cfg.anchor {
            Anchor::TopCenter => (gw - w) / 2,
            Anchor::Top | Anchor::VertCenter | Anchor::Bottom => (w - gw) * side_offset_mul,
            Anchor::Hotbar => gw / 2 + (HOTBAR_OFFSET + added) * side_mul + w * side_offset_mul,
        };
    let y = match cfg.anchor {
        // `modCompat.screenSafeArea()` is 0 without Bedrockify.
        Anchor::Bottom | Anchor::Hotbar => gh - h - cfg.offset_y,
        Anchor::Top | Anchor::TopCenter => cfg.offset_y,
        Anchor::VertCenter => (gh - h) / 2 + cfg.offset_y,
    };
    Some(Rect { x, y, w, h })
}

/// `ArmorHudMod.getEffectiveWidgetRect`.
fn effective_rect(cfg: &Config, r: Rect) -> Rect {
    let mut r = r;
    if cfg.orientation == Orientation::Horizontal {
        let mut extra = 0;
        if cfg.warning_shown {
            extra += WARNING_SIZE + 2 + cfg.warning_bob_intensity / 2;
        }
        if cfg.durability_display != DurabilityDisplay::Bar {
            extra += LINE_HEIGHT;
        }
        r.h += extra;
        if !cfg.anchor.is_top() {
            r.y -= extra;
        }
    }
    r
}

// ----------------------------------------------------------------- drawing

fn text_width(g: &Game<'_>, s: &str) -> i32 {
    g.native_text_width(s) as i32
}

fn text(g: &Game<'_>, b: &mut HudBatch, s: &str, x: i32, y: i32, rgb: u32, shadow: bool) {
    if !s.is_empty() {
        g.native_text(b, s, [x as f32, y as f32], rgb & 0xFF_FFFF, shadow);
    }
}

/// `GuiGraphicsExtractor.centeredText` (with shadow).
fn centered(g: &Game<'_>, b: &mut HudBatch, s: &str, cx: i32, y: i32, rgb: u32) {
    text(g, b, s, cx - text_width(g, s) / 2, y, rgb, true);
}

/// `blitSprite(sprite, textureWidth, textureHeight, u, v, x, y, w, h)`:
/// part `[u, v, w, h]` of a sprite laid out `tex` pixels large, unstretched.
fn blit_region(g: &Game<'_>, b: &mut HudBatch, sprite: &str, tex: [i32; 2], uv: [i32; 2], dst: [i32; 4]) {
    let size = g.native_sprite_size(sprite).unwrap_or([tex[0] as f32, tex[1] as f32]);
    let (sx, sy) = (size[0] / tex[0] as f32, size[1] / tex[1] as f32);
    let [x, y, w, h] = dst;
    g.native_sprite(
        b,
        sprite,
        [uv[0] as f32 * sx, uv[1] as f32 * sy, w as f32 * sx, h as f32 * sy],
        [x as f32, y as f32, w as f32, h as f32],
    );
}

/// `blitSprite(sprite, x, y, w, h)`: the whole sprite into the box.
fn blit_whole(g: &Game<'_>, b: &mut HudBatch, sprite: &str, dst: [f32; 4]) -> bool {
    let Some([w, h]) = g.native_sprite_size(sprite) else {
        return false;
    };
    g.native_sprite(b, sprite, [0.0, 0.0, w, h], dst)
}

fn warn_icon(g: &Game<'_>, b: &mut HudBatch, x: i32, y: i32) {
    for (row, pixels) in WARN.iter().enumerate() {
        let mut col = 0;
        while col < 8 {
            let c = pixels[col];
            let mut end = col + 1;
            while end < 8 && pixels[end] == c {
                end += 1;
            }
            if c != 0 {
                g.native_fill(
                    b,
                    [(x + col as i32) as f32, (y + row as i32) as f32, (end - col) as f32, 1.0],
                    c,
                );
            }
            col = end;
        }
    }
}

/// `Hud.extractSlot`: the item and its decorations (count included).
fn slot_item(g: &Game<'_>, b: &mut HudBatch, s: &ItemStack, x: i32, y: i32) {
    if !g.native_item_stack(b, s, [x as f32, y as f32], 16.0) {
        return;
    }
    if s.count != 1 {
        let count = s.count.to_string();
        text(g, b, &count, x + 19 - 2 - text_width(g, &count), y + 6 + 3, 0xFFFFFF, true);
    }
}

fn enabled(g: &Game<'_>) -> Option<Config> {
    let cfg = config(g);
    (g.native_on(ID) && cfg.enabled).then_some(cfg)
}

/// `ArmorHudRenderer.drawArmorHud`, at the end of the hotbar layer.
fn draw_armor_hud(g: &Game<'_>, b: &mut HudBatch) {
    let Some(cfg) = enabled(g) else { return };
    let Some(v) = peek::view(g) else { return };
    if v.spectator {
        return;
    }
    let (gw, gh) = (b.width as i32, b.height as i32);
    let mut items = armor_items(&cfg, g, &v);
    let Some(rect) = widget_rect(&cfg, &v, items.len(), gw, gh) else { return };
    if cfg.reversed {
        items.reverse();
    }
    let n = items.len() as i32;
    let texture_width = SIZE + (n - 1) * STEP;
    let vertical = cfg.orientation == Orientation::Vertical;

    // The slots, drawn at the origin and then moved (and turned) into place,
    // as the pose stack does.
    let start = b.vertices.len();
    match cfg.style {
        Style::Hotbar => {
            blit_region(g, b, HOTBAR_SPRITE, [182, 22], [0, 0], [0, 0, texture_width - 3, SIZE]);
            blit_region(g, b, HOTBAR_SPRITE, [182, 22], [182 - 3, 0], [texture_width - 3, 0, 3, SIZE]);
        }
        Style::RoundedCorners => {
            if n > 1 {
                blit_region(g, b, HOTBAR_OFFHAND_LEFT_SPRITE, [29, 24], [0, 1], [0, 0, 3, SIZE]);
                blit_region(g, b, HOTBAR_SPRITE, [182, 22], [3, 0], [3, 0, texture_width - 6, SIZE]);
                blit_region(g, b, HOTBAR_OFFHAND_LEFT_SPRITE, [29, 24], [SIZE - 3, 1], [texture_width - 3, 0, 3, SIZE]);
            } else {
                blit_region(g, b, HOTBAR_OFFHAND_LEFT_SPRITE, [29, 24], [0, 1], [0, 0, SIZE, SIZE]);
            }
        }
        Style::Rounded => {
            if n > 1 {
                let border = (SIZE - STEP) / 2;
                blit_region(g, b, HOTBAR_OFFHAND_LEFT_SPRITE, [29, 24], [0, 1], [0, 0, SIZE - border, SIZE]);
                for i in 1..n - 1 {
                    blit_region(g, b, HOTBAR_OFFHAND_LEFT_SPRITE, [29, 24], [border, 1], [border + i * STEP, 0, STEP, SIZE]);
                }
                blit_region(g, b, HOTBAR_OFFHAND_LEFT_SPRITE, [29, 24], [1, 1], [texture_width - STEP - border, 0, SIZE - border, SIZE]);
            } else {
                blit_region(g, b, HOTBAR_OFFHAND_LEFT_SPRITE, [29, 24], [0, 1], [0, 0, SIZE, SIZE]);
            }
        }
        Style::None => {}
    }
    let (rx, ry) = (rect.x as f32, rect.y as f32);
    for vtx in &mut b.vertices[start..] {
        let [lx, ly] = vtx.pos;
        // translate(x, y), then for VERTICAL rotate(HALF_PI).translate(0, -SIZE).
        vtx.pos = if vertical {
            [rx + SIZE as f32 - ly, ry + lx]
        } else {
            [rx + lx, ry + ly]
        };
    }

    let extras_side = if cfg.anchor == Anchor::Hotbar { cfg.side } else { cfg.side.opposite() };
    for (i, stack) in items.iter().enumerate() {
        let (mut x, mut y) = (rect.x, rect.y);
        if vertical {
            y += STEP * i as i32;
        } else {
            x += STEP * i as i32;
        }
        if cfg.icons_shown && cfg.widget_shown.draws_empty_slots() && stack.is_none() {
            let slot = if cfg.reversed { 3 - i } else { i };
            blit_whole(g, b, EMPTY_SLOTS[slot], [(x + 3) as f32, (y + 3) as f32, 16.0, 16.0]);
        }
        if let Some(s) = stack {
            slot_item(g, b, s, x + 3, y + 3);
        }
        if cfg.anchor.is_top() && !vertical {
            y += SIZE;
        } else if extras_side == Side::Right && vertical {
            x += SIZE;
        }
        if cfg.durability_display != DurabilityDisplay::Bar
            && let Some(s) = stack
            && let Some((damage, max)) = peek::durability(g, s)
        {
            let dura = durability_text(&cfg, damage, max);
            let color = bar_color(damage, max);
            if !vertical {
                if !cfg.anchor.is_top() {
                    y -= LINE_HEIGHT;
                }
                centered(g, b, &dura, x + SIZE / 2, y, color);
                if cfg.anchor.is_top() {
                    y += LINE_HEIGHT;
                }
            } else {
                let tw = text_width(g, &dura) + 2;
                let ty = (SIZE - LINE_HEIGHT) / 2;
                if extras_side == Side::Left {
                    x -= tw;
                }
                text(g, b, &dura, x + 1, y + ty, color, true);
                if extras_side == Side::Right {
                    x += tw;
                }
            }
        }
        if cfg.warning_shown && stack.as_ref().is_some_and(|s| should_warn(&cfg, g, s)) {
            if cfg.warning_bob_intensity != 0 {
                y += bob(cfg.warning_bob_intensity);
            }
            if !vertical {
                if !cfg.anchor.is_top() {
                    y -= WARNING_SIZE + 2;
                }
                warn_icon(g, b, x + (SIZE - WARNING_SIZE) / 2, y + 1);
            } else {
                if extras_side == Side::Left {
                    x -= WARNING_SIZE + 2;
                }
                warn_icon(g, b, x + 1, y + (SIZE - WARNING_SIZE) / 2);
            }
        }
    }
}

/// `ArmorHudRenderer.renderMainAndOffhand`, after the hotbar and its
/// decorations (the last of them, the selected item's name).
fn draw_hands(g: &Game<'_>, b: &mut HudBatch) {
    let Some(cfg) = enabled(g) else { return };
    let Some(v) = peek::view(g) else { return };
    let (gw, gh) = (b.width as i32, b.height as i32);
    if let Some(off) = &v.offhand
        && cfg.off_hand_durability
        && let Some((damage, max)) = peek::durability(g, off)
    {
        // The off hand slot is 7 pixels from the hotbar.
        let x = if !v.main_arm_left { gw / 2 - 91 - 29 } else { gw / 2 + 91 + 7 };
        let mut y = gh - SIZE;
        if cfg.durability_display != DurabilityDisplay::Bar {
            let dura = durability_text(&cfg, damage, max);
            y -= LINE_HEIGHT;
            centered(g, b, &dura, x + SIZE / 2, y, bar_color(damage, max));
        }
        if cfg.warning_shown && should_warn(&cfg, g, off) {
            if cfg.warning_bob_intensity != 0 {
                y += bob(cfg.warning_bob_intensity);
            }
            y -= WARNING_SIZE + 2;
            warn_icon(g, b, x + (SIZE - WARNING_SIZE) / 2, y + 1);
        }
    }
    if let Some(main) = &v.mainhand
        && cfg.main_hand_durability
        && cfg.durability_display != DurabilityDisplay::Bar
        && let Some((damage, max)) = peek::durability(g, main)
    {
        let x = gw / 2 - 91 + v.selected as i32 * 20 + SIZE / 2;
        let y = gh - SIZE - 1;
        let dura = durability_text(&cfg, damage, max);
        let tx = x - text_width(g, &dura) / 2;
        // An outline for visibility.
        text(g, b, &dura, tx + 1, y, 0, false);
        text(g, b, &dura, tx - 1, y, 0, false);
        text(g, b, &dura, tx, y + 1, 0, false);
        text(g, b, &dura, tx, y - 1, 0, false);
        text(g, b, &dura, tx, y, bar_color(damage, max), false);
    }
}

const PUSHED: [&str; 3] = ["minecraft:effects", "minecraft:boss_overlay", "minecraft:subtitle_overlay"];

/// MixinHud (effect icons), MixinBossHealthOverlay, MixinSubtitleOverlay:
/// the whole layer moves by the widget's effective box.
fn push(g: &Game<'_>, which: usize, b: &mut HudBatch) {
    let Some(start) = st().layer_start[which].take() else { return };
    let Some(cfg) = enabled(g) else { return };
    let wanted = match which {
        0 => cfg.push_status_effect_icons && cfg.anchor == Anchor::Top && cfg.side == Side::Right,
        1 => cfg.push_bossbars && cfg.anchor == Anchor::TopCenter,
        _ => cfg.push_subtitles && cfg.anchor == Anchor::Bottom && cfg.side == Side::Right,
    };
    if !wanted || start > b.vertices.len() {
        return;
    }
    let Some(v) = peek::view(g) else { return };
    let items = armor_items(&cfg, g, &v);
    let Some(r) = widget_rect(&cfg, &v, items.len(), b.width as i32, b.height as i32) else {
        return;
    };
    let r = effective_rect(&cfg, r);
    let dy = match which {
        0 | 1 => r.y + r.h,
        // "The subtitles widget is approx 25 pixels above the bottom of the screen."
        _ => -(r.h - 25).max(0),
    };
    if dy != 0 {
        for vtx in &mut b.vertices[start..] {
            vtx.pos[1] += dy as f32;
        }
    }
}

pub(super) fn on_layer_pre(g: &Game<'_>, layer: &str, b: &mut HudBatch) {
    if !g.native_window() {
        return;
    }
    if let Some(i) = PUSHED.iter().position(|l| *l == layer) {
        st().layer_start[i] = Some(b.vertices.len());
    }
}

pub(super) fn on_layer_post(g: &Game<'_>, layer: &str, b: &mut HudBatch) {
    if !g.native_window() {
        return;
    }
    match layer {
        "minecraft:hotbar" => draw_armor_hud(g, b),
        "minecraft:selected_item_name" => draw_hands(g, b),
        _ => {
            if let Some(i) = PUSHED.iter().position(|l| *l == layer) {
                push(g, i, b);
            }
        }
    }
}

// ------------------------------------------------------------ break sound

fn same_item(a: &Option<ItemStack>, b: &Option<ItemStack>) -> bool {
    let id = |s: &Option<ItemStack>| s.as_ref().map_or("minecraft:air".to_owned(), |s| s.item.to_string());
    id(a) == id(b)
}

/// `ArmorHudMod.shouldPlayBreakSound`, early return (and the stale
/// `lastStacks` it leaves) included.
fn should_play_break_sound(cfg: &Config, g: &Game<'_>, current: [Option<ItemStack>; 6]) -> bool {
    let damage = |s: &Option<ItemStack>| {
        s.as_ref()
            .and_then(|s| peek::durability(g, s).map(|(d, m)| d.clamp(0, m)))
            .unwrap_or(0)
    };
    for (i, cur) in current.into_iter().enumerate() {
        let last = std::mem::replace(&mut st().last[i], cur.clone());
        let should = damage(&last) != damage(&cur)
            && cur.as_ref().is_some_and(|s| should_warn(cfg, g, s));
        let main_changed = i == 4 && !same_item(&last, &cur);
        let armor = i < 4;
        let main = i == 4 && cfg.main_hand_durability;
        let off = i == 5 && cfg.off_hand_durability;
        if should && !main_changed && (armor || main || off) {
            return true;
        }
    }
    false
}

/// MixinMinecraft: at the end of each client tick.
pub(super) fn on_tick(g: &Game<'_>) {
    if !g.native_window() || !g.native_on(ID) {
        return;
    }
    let cfg = config(g);
    if !cfg.play_break_sound {
        return;
    }
    let Some(v) = peek::view(g) else { return };
    let [h, c, l, f] = v.armor;
    if should_play_break_sound(&cfg, g, [h, c, l, f, v.mainhand, v.offhand]) {
        ukus_armor_hud_sound::play(peek::players_volume(g));
    }
}

// ------------------------------------------------------------------- keys

/// `Ukutils.registerToggleBind`: "Armor Hud toggled ON/OFF" on the action bar.
pub(super) fn on_key(g: &mut Game<'_>, key: &str, pressed: bool) {
    if !pressed || !g.native_window() || !g.native_on(ID) || !g.native_playing() {
        return;
    }
    if key == g.native_mod_key(ID, "toggle", "key.keyboard.unknown") {
        let mut cfg = config(g);
        cfg.enabled = !cfg.enabled;
        save(g, cfg);
        let state = if cfg.enabled {
            Text::translate("options.on", &[]).with_color(0x55FF55).bold()
        } else {
            Text::translate("options.off", &[]).with_color(0xFF5555).bold()
        };
        g.native_client_action_bar(Text::literal("Armor Hud toggled ").append(state), false);
    } else if key == g.native_mod_key(ID, "config", "key.keyboard.unknown") {
        open_screen(g);
    }
}

fn open_screen(g: &mut Game<'_>) {
    let cfg = config(g);
    if g.native_mod_screen_open(ID) {
        st().ui = Some(Ui::new(&cfg));
    }
}

/// Once a frame: Done/Reset close the screen; a screen asked for from
/// Options opens once the player is back in the world.
pub(super) fn on_frame(g: &mut Game<'_>) {
    if !g.native_window() {
        return;
    }
    let (close, open, sound) = {
        let mut s = st();
        (std::mem::take(&mut s.close), s.open, std::mem::take(&mut s.sound))
    };
    if sound
        && let Some(p) = g.native_player()
    {
        // SimpleSoundInstance.forUI(UI_BUTTON_CLICK, 1.0): volume 0.25.
        g.native_play_local_sound("minecraft:ui.button.click", [p.x, p.y, p.z], 0.25, 1.0, 0);
    }
    if close {
        g.native_mod_screen_close(ID);
    }
    if open && g.native_playing() {
        st().open = false;
        open_screen(g);
    }
}

// ------------------------------------------------------- Options > Mods

pub(super) fn on_mods(mods: &mut Vec<NativeMod>) {
    mods.push(NativeMod {
        id: ID.into(),
        title: format!("{NAME} {VERSION}"),
        description: DESCRIPTION.into(),
        keys: vec![
            ["toggle".into(), "Toggle Armor Hud".into(), "key.keyboard.unknown".into()],
            ["config".into(), "Open Config Screen".into(), "key.keyboard.unknown".into()],
        ],
    });
}

/// The settings page: the config screen (in a world), then every
/// cycling option, so they can be changed from the title screen too.
const PAGE: [W; 17] = [
    W::Enabled,
    W::Style,
    W::WidgetShown,
    W::OffhandSlot,
    W::DuraDisplay,
    W::Reversed,
    W::ShowIcons,
    W::OffhandDura,
    W::MainhandDura,
    W::Anchor,
    W::Side,
    W::Orientation,
    W::PushBoss,
    W::PushIcons,
    W::PushSubs,
    W::PlaySound,
    W::ShowWarning,
];

pub(super) fn on_settings(g: &Game<'_>, mod_id: &str, buttons: &mut Vec<String>) {
    if mod_id != ID {
        return;
    }
    let cfg = config(g);
    let open = if st().open { "Open Config Screen... (opens in game)" } else { "Open Config Screen..." };
    buttons.push(open.into());
    for w in PAGE {
        buttons.push(w.label(&cfg));
    }
}

pub(super) fn on_settings_click(g: &Game<'_>, mod_id: &str, button: usize) {
    if mod_id != ID {
        return;
    }
    if button == 0 {
        st().open = true;
        return;
    }
    if let Some(w) = PAGE.get(button - 1) {
        let mut cfg = config(g);
        w.cycle(&mut cfg, 1);
        save(g, cfg);
    }
}

// ------------------------------------------------------- the config screen

/// Every widget of the three tabs.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum W {
    Enabled,
    Style,
    WidgetShown,
    OffhandSlot,
    DuraDisplay,
    Reversed,
    ShowIcons,
    OffhandDura,
    MainhandDura,
    Anchor,
    Side,
    Orientation,
    PushBoss,
    PushIcons,
    PushSubs,
    OffsetX,
    OffsetY,
    PlaySound,
    ShowWarning,
    Bob,
    MinValue,
    MinPercent,
}

/// `ArmorHudConfigScreen`: StyleTab, PositionTab, WarningTab.
const TABS: [(&str, &[W]); 3] = [
    (
        "Style",
        &[W::Enabled, W::Style, W::WidgetShown, W::OffhandSlot, W::DuraDisplay, W::Reversed, W::ShowIcons, W::OffhandDura, W::MainhandDura],
    ),
    (
        "Position",
        &[W::Anchor, W::Side, W::Orientation, W::PushBoss, W::PushIcons, W::PushSubs, W::OffsetX, W::OffsetY],
    ),
    ("Warning", &[W::PlaySound, W::ShowWarning, W::Bob, W::MinValue, W::MinPercent]),
];

fn on_off(v: bool) -> String {
    tr(if v { "options.on" } else { "options.off" })
}

impl W {
    fn name(self) -> &'static str {
        match self {
            W::Enabled => "Enabled",
            W::Style => "Style",
            W::WidgetShown => "Show Widget",
            W::OffhandSlot => "Offhand Slot",
            W::DuraDisplay => "Durability Display",
            W::Reversed => "Reversed",
            W::ShowIcons => "Show Icons",
            W::OffhandDura => "Show Offhand Durability",
            W::MainhandDura => "Show Main Hand Durability",
            W::Anchor => "Anchor",
            W::Side => "Side",
            W::Orientation => "Orientation",
            W::PushBoss => "Push Bossbars",
            W::PushIcons => "Push Effect Icons",
            W::PushSubs => "Push Subtitles",
            W::OffsetX => "X Offset",
            W::OffsetY => "Y Offset",
            W::PlaySound => "Play Breaking Sound",
            W::ShowWarning => "Show Warning",
            W::Bob => "Icon Bob Intensity",
            W::MinValue | W::MinPercent => "Min Durability",
        }
    }
    /// `CycleButton`'s message: `Options.genericValueLabel(name, value)`.
    fn label(self, c: &Config) -> String {
        let value = match self {
            W::Enabled => on_off(c.enabled),
            W::Style => c.style.label(),
            W::WidgetShown => c.widget_shown.label(),
            W::OffhandSlot => c.offhand_slot_behavior.label(),
            W::DuraDisplay => c.durability_display.label(),
            W::Reversed => on_off(c.reversed),
            W::ShowIcons => on_off(c.icons_shown),
            W::OffhandDura => on_off(c.off_hand_durability),
            W::MainhandDura => on_off(c.main_hand_durability),
            W::Anchor => c.anchor.label(),
            W::Side => c.side.label(),
            W::Orientation => c.orientation.label(),
            W::PushBoss => on_off(c.push_bossbars),
            W::PushIcons => on_off(c.push_status_effect_icons),
            W::PushSubs => on_off(c.push_subtitles),
            W::PlaySound => on_off(c.play_break_sound),
            W::ShowWarning => on_off(c.warning_shown),
            W::OffsetX => c.offset_x.to_string(),
            W::OffsetY => c.offset_y.to_string(),
            W::Bob => c.warning_bob_intensity.to_string(),
            W::MinValue => c.min_durability_value.to_string(),
            W::MinPercent => format!("{:.0}%", c.min_durability_percentage * 100.0),
        };
        Text::translate(
            "options.generic_value",
            &[Text::literal(self.name()), Text::literal(value)],
        )
        .plain()
    }
    /// `CycleButton.cycleValue(by)` on a boolean (`[TRUE, FALSE]`) or enum option.
    fn cycle(self, c: &mut Config, by: i32) {
        let flip = |v: &mut bool| *v = !*v;
        match self {
            W::Enabled => flip(&mut c.enabled),
            W::Style => c.style = c.style.step(by),
            W::WidgetShown => c.widget_shown = c.widget_shown.step(by),
            W::OffhandSlot => c.offhand_slot_behavior = c.offhand_slot_behavior.step(by),
            W::DuraDisplay => c.durability_display = c.durability_display.step(by),
            W::Reversed => flip(&mut c.reversed),
            W::ShowIcons => flip(&mut c.icons_shown),
            W::OffhandDura => flip(&mut c.off_hand_durability),
            W::MainhandDura => flip(&mut c.main_hand_durability),
            W::Anchor => c.anchor = c.anchor.step(by),
            W::Side => c.side = c.side.step(by),
            W::Orientation => c.orientation = c.orientation.step(by),
            W::PushBoss => flip(&mut c.push_bossbars),
            W::PushIcons => flip(&mut c.push_status_effect_icons),
            W::PushSubs => flip(&mut c.push_subtitles),
            W::PlaySound => flip(&mut c.play_break_sound),
            W::ShowWarning => flip(&mut c.warning_shown),
            _ => {}
        }
    }
    /// A `TypedInputOption`: its index among the screen's text boxes.
    fn input(self) -> Option<usize> {
        match self {
            W::OffsetX => Some(0),
            W::OffsetY => Some(1),
            W::Bob => Some(2),
            W::MinValue => Some(3),
            _ => None,
        }
    }
    /// What an input's valid value sets.
    fn set_int(self, c: &mut Config, v: i32) {
        match self {
            W::OffsetX => c.offset_x = v,
            W::OffsetY => c.offset_y = v,
            W::Bob => c.warning_bob_intensity = v,
            W::MinValue => c.min_durability_value = v,
            _ => {}
        }
    }
}

const INPUTS: [W; 4] = [W::OffsetX, W::OffsetY, W::Bob, W::MinValue];

/// `ArmorHudConfigScreen.getInt`: `Integer.parseInt`.
fn parse_int(s: &str) -> Option<i32> {
    let digits = s.strip_prefix(['+', '-']).unwrap_or(s);
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    s.parse().ok()
}

/// An `EditBox` (ukulib's `TextInputWidget`).
struct Edit {
    value: String,
    /// Character positions.
    cursor: usize,
    anchor: usize,
    display: usize,
    focused_at: Instant,
}

impl Edit {
    fn new(value: String) -> Edit {
        let n = value.chars().count();
        Edit { value, cursor: n, anchor: n, display: 0, focused_at: Instant::now() }
    }
    fn len(&self) -> usize {
        self.value.chars().count()
    }
    fn byte(&self, i: usize) -> usize {
        self.value.char_indices().nth(i).map_or(self.value.len(), |(b, _)| b)
    }
    fn selection(&self) -> (usize, usize) {
        (self.cursor.min(self.anchor), self.cursor.max(self.anchor))
    }
    /// `EditBox.insertText`.
    fn insert(&mut self, s: &str) {
        let (a, b) = self.selection();
        let (ba, bb) = (self.byte(a), self.byte(b));
        let room = 1000usize.saturating_sub(self.len() - (b - a));
        let s: String = s.chars().take(room).collect();
        self.value.replace_range(ba..bb, &s);
        self.cursor = a + s.chars().count();
        self.anchor = self.cursor;
    }
    /// `EditBox.deleteChars(dir)`.
    fn delete(&mut self, dir: i32) {
        if self.cursor != self.anchor {
            self.insert("");
            return;
        }
        let (a, b) = if dir < 0 {
            (self.cursor.saturating_sub(1), self.cursor)
        } else {
            (self.cursor, (self.cursor + 1).min(self.len()))
        };
        if a == b {
            return;
        }
        let (ba, bb) = (self.byte(a), self.byte(b));
        self.value.replace_range(ba..bb, "");
        self.cursor = a;
        self.anchor = a;
    }
    fn move_to(&mut self, pos: usize, select: bool) {
        self.cursor = pos.min(self.len());
        if !select {
            self.anchor = self.cursor;
        }
    }
    /// `EditBox.scrollTo`: keep the cursor within the shown part.
    fn scroll(&mut self, g: &Game<'_>, inner: i32) {
        let chars: Vec<char> = self.value.chars().collect();
        self.display = self.display.min(chars.len());
        if self.cursor < self.display {
            self.display = self.cursor;
        }
        loop {
            let shown: String = chars[self.display..self.cursor.max(self.display)].iter().collect();
            if text_width(g, &shown) <= inner || self.display >= self.cursor {
                break;
            }
            self.display += 1;
        }
    }
    /// `EditBox.findClickedPositionInText`.
    fn clicked(&self, g: &Game<'_>, mouse_x: i32, text_x: i32, inner: i32) -> usize {
        let at = (mouse_x - text_x).min(inner);
        let mut n = 0;
        let mut acc = String::new();
        for ch in self.value.chars().skip(self.display) {
            acc.push(ch);
            if text_width(g, &acc) > at {
                break;
            }
            n += 1;
        }
        self.display + n
    }
}

/// What has the screen's focus (vanilla keeps a clicked widget focused, and
/// a focused button or tab draws highlighted).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Focus {
    Tab(usize),
    Widget(W),
    Reset,
    Done,
}

pub(super) struct Ui {
    tab: usize,
    focus: Option<Focus>,
    edits: [Edit; 4],
    /// The slider's own 0..1 value (`AbstractSliderButton.value`).
    slider: f64,
    dragging: bool,
}

impl Ui {
    fn new(c: &Config) -> Ui {
        Ui {
            tab: 0,
            focus: None,
            edits: [
                Edit::new(c.offset_x.to_string()),
                Edit::new(c.offset_y.to_string()),
                Edit::new(c.warning_bob_intensity.to_string()),
                Edit::new(c.min_durability_value.to_string()),
            ],
            slider: c.min_durability_percentage.clamp(0.0, 1.0),
            dragging: false,
        }
    }
    /// `BaseConfigScreen.getInvalidOptions`: every tab's text boxes.
    fn invalid(&self) -> Vec<W> {
        INPUTS
            .iter()
            .zip(&self.edits)
            .filter(|(_, e)| parse_int(&e.value).is_none())
            .map(|(w, _)| *w)
            .collect()
    }
    fn focused(&self, f: Focus) -> bool {
        self.focus == Some(f)
    }
    /// The text box that has the focus.
    fn edit(&self) -> Option<(W, usize)> {
        match self.focus {
            Some(Focus::Widget(w)) => w.input().map(|i| (w, i)),
            _ => None,
        }
    }
}

/// `DoubleSlider.fixedValue` for min 0, max 1, step 0.01.
fn slider_value(v: f64) -> f64 {
    ((v / 0.01).ceil() * 0.01).clamp(0.0, 1.0)
}

// Layout: vanilla's numbers.
const TAB_H: i32 = 24;
const FOOTER_H: i32 = 33;
const WIDGET_W: i32 = 210;
const WIDGET_H: i32 = 20;
const ROW_SPACING: i32 = 4;
const BUTTON_W: i32 = 150;

/// `Mth.roundToward`: `positiveCeilDiv(input, multiple) * multiple`.
fn round_toward(input: i32, multiple: i32) -> i32 {
    -((-input).div_euclid(multiple)) * multiple
}

struct Layout {
    tab_x: i32,
    tab_w: i32,
    grid_x: i32,
    grid_y: i32,
    reset_x: i32,
    done_x: i32,
    footer_y: i32,
    separator_y: i32,
}

/// `MenuTabBar.arrangeElements`, `TabManager.setTabArea` with
/// `GridLayoutTab.doLayout` (`alignInRectangle(0.5, 1/6)`), and the
/// `HeaderAndFooterLayout` footer (33 high, buttons 8 apart).
fn layout(w: i32, h: i32, tab: usize) -> Layout {
    let tabs_width = w.min(400) - 28;
    let tab_w = round_toward(tabs_width / TABS.len() as i32, 2);
    let tab_x = round_toward((w - tabs_width) / 2, 2);
    let n = TABS[tab].1.len() as i32;
    let grid_h = n * WIDGET_H + (n - 1) * ROW_SPACING;
    let area_h = h - 36 - TAB_H;
    // `(int) Mth.lerp(align, 0, room)`.
    let lerp = |align: f32, room: i32| (align * room as f32) as i32;
    let reset_x = lerp(0.5, w - (BUTTON_W * 2 + 8));
    Layout {
        tab_x,
        tab_w,
        grid_x: lerp(0.5, w - WIDGET_W),
        grid_y: TAB_H + lerp(1.0 / 6.0, area_h - grid_h),
        reset_x,
        done_x: reset_x + BUTTON_W + 8,
        // A FrameLayout child: `Math.round(lerp(0.5, 0, 33 - 20))` = 7.
        footer_y: h - FOOTER_H + (0.5 * (FOOTER_H - WIDGET_H) as f32).round() as i32,
        separator_y: round_toward(h - 36 - 2, 2),
    }
}

fn widget_y(l: &Layout, row: usize) -> i32 {
    l.grid_y + row as i32 * (WIDGET_H + ROW_SPACING)
}

fn inside(m: [f32; 2], x: i32, y: i32, w: i32, h: i32) -> bool {
    m[0] >= x as f32 && m[1] >= y as f32 && m[0] < (x + w) as f32 && m[1] < (y + h) as f32
}

fn ours(g: &Game<'_>, screen: &NativeScreenView<'_>) -> bool {
    screen.id == "mod" && g.native_mod_screen() == Some(ID)
}

/// `Screen.extractBlurredBackground`: the config screen blurs the world and
/// HUD behind it by `menuBackgroundBlurriness`, as any vanilla screen does.
pub(super) fn on_menu_background(g: &Game<'_>, radius: &mut Option<f32>) {
    if radius.is_none()
        && g.native_mod_screen() == Some(ID)
        && let Some(state) = g.native_client_state()
        && state.menu_blur >= 1.0
    {
        *radius = Some(state.menu_blur);
    }
}

fn gui_size(g: &Game<'_>) -> (i32, i32) {
    g.native_client_state()
        .map_or((0, 0), |s| (s.gui_size[0] as i32, s.gui_size[1] as i32))
}

/// `blitSprite` of a nine-slice widget sprite; a flat stand-in without one.
fn widget(g: &Game<'_>, b: &mut HudBatch, sprite: &str, x: i32, y: i32, w: i32, h: i32) {
    if g.native_sprite_size(sprite).is_some() {
        g.native_gui_nine_slice(b, sprite, [x as f32, y as f32, w as f32, h as f32], 0xFFFF_FFFF);
    } else {
        g.native_fill(b, [x as f32, y as f32, w as f32, h as f32], 0xFF55_5555);
    }
}

/// `extractScrollingStringOverContents(message, 2)` for a label that fits:
/// centred, `(top + bottom - 9) / 2 + 1`.
fn label(g: &Game<'_>, b: &mut HudBatch, s: &str, x: i32, y: i32, w: i32, rgb: u32) {
    centered(g, b, s, x + w / 2, (y + y + WIDGET_H - LINE_HEIGHT) / 2 + 1, rgb);
}

/// `AbstractButton`: `SPRITES.get(active, isHoveredOrFocused)` and its label
/// (gray when inactive, `WithInactiveMessage`).
fn button(g: &Game<'_>, b: &mut HudBatch, m: [f32; 2], s: &str, x: i32, y: i32, w: i32, active: bool, focused: bool) {
    let lit = inside(m, x, y, w, WIDGET_H) || focused;
    let sprite = match (active, lit) {
        (false, _) => "gui/sprites/widget/button_disabled",
        (true, true) => "gui/sprites/widget/button_highlighted",
        (true, false) => "gui/sprites/widget/button",
    };
    widget(g, b, sprite, x, y, w, WIDGET_H);
    label(g, b, s, x, y, w, if active { 0xFFFFFF } else { 0xA0A0A0 });
}

/// `blit(texture, x, y, 0, 0, width, height, 32, 32)`: a texture repeated
/// from (x, y), as its UVs wrap.
fn tiled(g: &Game<'_>, b: &mut HudBatch, sprite: &str, size: [i32; 2], x0: i32, y0: i32, w: i32, h: i32) -> bool {
    if g.native_sprite_size(sprite).is_none() {
        return false;
    }
    let mut y = y0;
    while y < y0 + h {
        let th = (y0 + h - y).min(size[1]);
        let mut x = x0;
        while x < x0 + w {
            let tw = (x0 + w - x).min(size[0]);
            blit_region(g, b, sprite, size, [0, 0], [x, y, tw, th]);
            x += tw;
        }
        y += th;
    }
    true
}

/// Splits tooltip text to 170 px (`Tooltip.splitTooltip`).
fn wrap(g: &Game<'_>, s: &str) -> Vec<Text> {
    let mut out = Vec::new();
    for para in s.split('\n') {
        let mut line = String::new();
        for word in para.split(' ') {
            let next = if line.is_empty() { word.to_owned() } else { format!("{line} {word}") };
            if text_width(g, &next) > 170 && !line.is_empty() {
                out.push(Text::literal(std::mem::take(&mut line)));
                line = word.to_owned();
            } else {
                line = next;
            }
        }
        out.push(Text::literal(line));
    }
    out
}

/// `EditBox.extractWidgetRenderState` (ukulib's `TextInputWidget`).
fn draw_edit(g: &Game<'_>, b: &mut HudBatch, e: &mut Edit, wd: W, x: i32, y: i32, focused: bool) {
    let sprite = if focused {
        "gui/sprites/widget/text_field_highlighted"
    } else {
        "gui/sprites/widget/text_field"
    };
    widget(g, b, sprite, x, y, WIDGET_W, WIDGET_H);
    let inner = WIDGET_W - 8;
    e.scroll(g, inner);
    let valid = parse_int(&e.value).is_some();
    let color = if valid { 0xE0E0E0 } else { 0xDF5050 };
    let (text_x, text_y) = (x + 4, y + (WIDGET_H - 8) / 2);
    let rest: Vec<char> = e.value.chars().skip(e.display).collect();
    // `plainSubstrByWidth(value.substring(displayPos), innerWidth)`.
    let mut displayed = String::new();
    for ch in &rest {
        displayed.push(*ch);
        if text_width(g, &displayed) > inner {
            displayed.pop();
            break;
        }
    }
    let shown: Vec<char> = displayed.chars().collect();
    let rel = e.cursor as isize - e.display as isize;
    let on_screen = rel >= 0 && rel as usize <= shown.len();
    let blink = (e.focused_at.elapsed().as_millis() / 300) % 2 == 0;
    let show_cursor = focused && blink && on_screen;
    let mut draw_x = text_x;
    let rel_highlight = (e.anchor as isize - e.display as isize).clamp(0, shown.len() as isize) as usize;
    if !shown.is_empty() {
        let half: String = if on_screen { shown[..rel as usize].iter().collect() } else { displayed.clone() };
        text(g, b, &half, draw_x, text_y, color, true);
        draw_x += text_width(g, &half) + 1;
    }
    let insert = e.cursor < e.len() || e.len() >= 1000;
    let mut cursor_x = draw_x;
    if !on_screen {
        cursor_x = if rel > 0 { text_x + WIDGET_W } else { text_x };
    } else if insert {
        cursor_x -= 1;
        draw_x -= 1;
    }
    if !shown.is_empty() && on_screen && (rel as usize) < shown.len() {
        let tail: String = shown[rel as usize..].iter().collect();
        text(g, b, &tail, draw_x, text_y, color, true);
    }
    if rel_highlight as isize != rel {
        let pre: String = shown[..rel_highlight].iter().collect();
        let highlight_x = text_x + text_width(g, &pre);
        let (x0, x1) = (cursor_x.min(x + WIDGET_W), (highlight_x - 1).min(x + WIDGET_W));
        let (x0, x1) = (x0.min(x1), x0.max(x1));
        // `textHighlight`: an inverting fill, then the blue highlight.
        let rect = [x0 as f32, (text_y - 1) as f32, (x1 - x0) as f32, 11.0];
        let start = b.vertices.len();
        g.native_fill(b, rect, 0xFFFF_FFFF);
        b.invert_since(start);
        let start = b.vertices.len();
        g.native_fill(b, rect, 0xFF00_00FF);
        b.blend_since(start, horizon_client::hud::Blend::Additive);
    }
    if show_cursor {
        if insert {
            // TextCursorUtils.extractInsertCursor: the text's colour, 10 high.
            g.native_fill(b, [cursor_x as f32, (text_y - 1) as f32, 1.0, 11.0], 0xFF00_0000 | color);
        } else {
            text(g, b, "_", cursor_x, text_y, color, true);
        }
    }
    // TextInputWidget: the suggestion, gray, at the right edge, when it fits.
    let suggestion = wd.name();
    if text_width(g, &format!("{}{}", e.value, suggestion)) < inner {
        let sx = x + WIDGET_W - 4 - text_width(g, suggestion);
        text(g, b, suggestion, sx, text_y, 0x808080, true);
    }
}

pub(super) fn on_screen_render(g: &Game<'_>, b: &mut HudBatch, screen: &NativeScreenView<'_>) {
    if !ours(g, screen) {
        return;
    }
    let cfg = config(g);
    let mut s = st();
    let Some(ui) = s.ui.as_mut() else { return };
    let (w, h) = (b.width as i32, b.height as i32);
    let m = screen.mouse;
    let l = layout(w, h, ui.tab);

    // Screen.extractBackground in a world: the in-world menu background.
    if !tiled(g, b, "gui/inworld_menu_background", [32, 32], 0, 0, w, h) {
        g.native_fill(b, [0.0, 0.0, w as f32, h as f32], 0xA000_0000);
    }

    // MenuTabBar: the header separator either side of the tabs, then the tabs.
    let tabs_end = l.tab_x + l.tab_w * TABS.len() as i32;
    tiled(g, b, "gui/header_separator", [32, 2], 0, TAB_H - 2, l.tab_x, 2);
    tiled(g, b, "gui/header_separator", [32, 2], tabs_end, TAB_H - 2, w - tabs_end, 2);
    for (i, (name, _)) in TABS.iter().enumerate() {
        let x = l.tab_x + l.tab_w * i as i32;
        let selected = ui.tab == i;
        let lit = inside(m, x, 0, l.tab_w, TAB_H) || ui.focused(Focus::Tab(i));
        let sprite = match (selected, lit) {
            (true, true) => "gui/sprites/widget/tab_selected_highlighted",
            (true, false) => "gui/sprites/widget/tab_selected",
            (false, true) => "gui/sprites/widget/tab_highlighted",
            (false, false) => "gui/sprites/widget/tab",
        };
        widget(g, b, sprite, x, 0, l.tab_w, TAB_H);
        if selected {
            // MenuTabButton.renderMenuBackground, then the focus underline.
            tiled(g, b, "gui/menu_background", [32, 32], x + 2, 2, l.tab_w - 4, TAB_H - 2);
            let uw = text_width(g, name).min(l.tab_w - 4);
            let ux = x + (l.tab_w - uw) / 2;
            g.native_fill(b, [ux as f32, (TAB_H - 2) as f32, uw as f32, 1.0], 0xFFFF_FFFF);
        }
        let top = if selected { 0 } else { 3 };
        centered(g, b, name, x + l.tab_w / 2, (top + TAB_H - LINE_HEIGHT) / 2 + 1, 0xFFFFFF);
    }

    // The tab's grid.
    let mut tooltip: Option<Vec<Text>> = None;
    for (row, wd) in TABS[ui.tab].1.iter().enumerate() {
        let (x, y) = (l.grid_x, widget_y(&l, row));
        let focused = ui.focused(Focus::Widget(*wd));
        if let Some(i) = wd.input() {
            draw_edit(g, b, &mut ui.edits[i], *wd, x, y, focused);
            if inside(m, x, y, WIDGET_W, WIDGET_H) {
                tooltip = Some(wrap(g, wd.name()));
            }
        } else if *wd == W::MinPercent {
            // AbstractSliderButton: the track lights while focused, the
            // handle while hovered.
            let track = if focused {
                "gui/sprites/widget/slider_highlighted"
            } else {
                "gui/sprites/widget/slider"
            };
            widget(g, b, track, x, y, WIDGET_W, WIDGET_H);
            let handle = if inside(m, x, y, WIDGET_W, WIDGET_H) {
                "gui/sprites/widget/slider_handle_highlighted"
            } else {
                "gui/sprites/widget/slider_handle"
            };
            let hx = x + (ui.slider * (WIDGET_W - 8) as f64) as i32;
            widget(g, b, handle, hx, y, 8, WIDGET_H);
            let mut shown = cfg;
            shown.min_durability_percentage = slider_value(ui.slider);
            label(g, b, &wd.label(&shown), x, y, WIDGET_W, 0xFFFFFF);
        } else {
            button(g, b, m, &wd.label(&cfg), x, y, WIDGET_W, true, focused);
        }
    }

    // TabbedConfigScreen: the footer separator, Reset and Done.
    let invalid = ui.invalid();
    button(g, b, m, "Reset To Default", l.reset_x, l.footer_y, BUTTON_W, true, ui.focused(Focus::Reset));
    button(g, b, m, &tr("gui.done"), l.done_x, l.footer_y, BUTTON_W, invalid.is_empty(), ui.focused(Focus::Done));
    if !invalid.is_empty() && inside(m, l.done_x, l.footer_y, BUTTON_W, WIDGET_H) {
        let names: Vec<&str> = invalid.iter().map(|w| w.name()).collect();
        tooltip = Some(wrap(
            g,
            &format!(
                "Some of the values entered in the config are invalid: {}.\nYou can still exit by pressing ESC, but the config won't be saved.",
                names.join(", ")
            ),
        ));
    }
    drop(s);
    tiled(g, b, "gui/footer_separator", [32, 2], 0, l.separator_y, w, 2);
    if let Some(lines) = tooltip {
        g.native_gui_tooltip(b, &lines, m);
    }
}

/// `playDownSound`: queued, played on the next frame (the screen hooks only
/// read the game).
fn click_sound() {
    st().sound = true;
}

/// A text box's responder: a value that parses goes into the config.
fn apply_edit(g: &Game<'_>, wd: W, value: &str) {
    if let Some(v) = parse_int(value) {
        let mut cfg = config(g);
        wd.set_int(&mut cfg, v);
        save(g, cfg);
    }
}

pub(super) fn on_mouse_pressed(g: &Game<'_>, screen: &NativeScreenView<'_>, button: i32, modifiers: u32, canceled: &mut bool) {
    if !ours(g, screen) {
        return;
    }
    *canceled = true;
    if button != 0 {
        return;
    }
    let (w, h) = gui_size(g);
    let m = screen.mouse;
    let shift = modifiers & 1 != 0;
    let mut s = st();
    let Some(ui) = s.ui.as_mut() else { return };
    let l = layout(w, h, ui.tab);
    // Tabs.
    for i in 0..TABS.len() {
        if inside(m, l.tab_x + l.tab_w * i as i32, 0, l.tab_w, TAB_H) {
            ui.tab = i;
            ui.focus = Some(Focus::Tab(i));
            drop(s);
            click_sound();
            return;
        }
    }
    // Reset To Default: back to the parent, the default config saved.
    if inside(m, l.reset_x, l.footer_y, BUTTON_W, WIDGET_H) {
        s.ui = None;
        s.close = true;
        s.sound = true;
        drop(s);
        save(g, DEFAULT);
        return;
    }
    if inside(m, l.done_x, l.footer_y, BUTTON_W, WIDGET_H) {
        // An inactive button takes neither the click nor the focus.
        if ui.invalid().is_empty() {
            ui.focus = Some(Focus::Done);
            s.close = true;
            s.sound = true;
        }
        return;
    }
    // The tab's widgets; a click on nothing keeps the focus where it was.
    let Some(row) = (0..TABS[ui.tab].1.len()).find(|&r| inside(m, l.grid_x, widget_y(&l, r), WIDGET_W, WIDGET_H)) else {
        return;
    };
    let wd = TABS[ui.tab].1[row];
    ui.focus = Some(Focus::Widget(wd));
    if let Some(i) = wd.input() {
        // EditBox.onClick: the cursor where the text was clicked.
        let e = &mut ui.edits[i];
        e.focused_at = Instant::now();
        let pos = e.clicked(g, m[0].floor() as i32, l.grid_x + 4, WIDGET_W - 8);
        e.move_to(pos, shift);
    } else if wd == W::MinPercent {
        ui.dragging = true;
        ui.slider = (((m[0] - (l.grid_x + 4) as f32) / (WIDGET_W - 8) as f32) as f64).clamp(0.0, 1.0);
        let v = slider_value(ui.slider);
        if let Some(c) = s.cfg.as_mut() {
            c.min_durability_percentage = v;
        }
    } else {
        // CycleButton.onPress: shift goes backwards.
        let mut cfg = s.cfg.unwrap_or(DEFAULT);
        wd.cycle(&mut cfg, if shift { -1 } else { 1 });
        s.sound = true;
        drop(s);
        save(g, cfg);
    }
}

pub(super) fn on_mouse_released(g: &Game<'_>, screen: &NativeScreenView<'_>, canceled: &mut bool) {
    if !ours(g, screen) {
        return;
    }
    *canceled = true;
    let value = {
        let mut s = st();
        let Some(ui) = s.ui.as_mut() else { return };
        if !ui.dragging {
            return;
        }
        ui.dragging = false;
        // AbstractSliderButton.onRelease plays the click.
        s.sound = true;
        slider_value(s.ui.as_ref().map_or(0.0, |u| u.slider))
    };
    let mut cfg = config(g);
    cfg.min_durability_percentage = value;
    save(g, cfg);
}

pub(super) fn on_mouse_dragged(g: &Game<'_>, screen: &NativeScreenView<'_>, canceled: &mut bool) {
    if !ours(g, screen) {
        return;
    }
    *canceled = true;
    let (w, h) = gui_size(g);
    let mut s = st();
    let Some(ui) = s.ui.as_mut() else { return };
    if ui.dragging {
        let l = layout(w, h, ui.tab);
        ui.slider = (((screen.mouse[0] - (l.grid_x + 4) as f32) / (WIDGET_W - 8) as f32) as f64).clamp(0.0, 1.0);
        let v = slider_value(ui.slider);
        if let Some(c) = s.cfg.as_mut() {
            c.min_durability_percentage = v;
        }
    }
}

pub(super) fn on_mouse_scrolled(g: &Game<'_>, screen: &NativeScreenView<'_>, scroll: [f64; 2], canceled: &mut bool) {
    if !ours(g, screen) {
        return;
    }
    *canceled = true;
    let (w, h) = gui_size(g);
    let mut s = st();
    let Some(ui) = s.ui.as_ref() else { return };
    let l = layout(w, h, ui.tab);
    let Some(row) = (0..TABS[ui.tab].1.len()).find(|&r| inside(screen.mouse, l.grid_x, widget_y(&l, r), WIDGET_W, WIDGET_H)) else {
        return;
    };
    let wd = TABS[ui.tab].1[row];
    if wd.input().is_some() || wd == W::MinPercent {
        return;
    }
    // CycleButton.mouseScrolled.
    let by = if scroll[1] > 0.0 {
        -1
    } else if scroll[1] < 0.0 {
        1
    } else {
        return;
    };
    let mut cfg = s.cfg.unwrap_or(DEFAULT);
    wd.cycle(&mut cfg, by);
    s.cfg = Some(cfg);
    drop(s);
    save(g, cfg);
}

pub(super) fn on_key_pressed(g: &Game<'_>, screen: &NativeScreenView<'_>, key: &str, modifiers: u32, canceled: &mut bool) {
    // Escape closes the screen (Horizon's own handling).
    if !ours(g, screen) || key == "key.keyboard.escape" {
        return;
    }
    *canceled = true;
    let shift = modifiers & 1 != 0;
    let ctrl = modifiers & 2 != 0;
    let (wd, value) = {
        let mut s = st();
        let Some(ui) = s.ui.as_mut() else { return };
        let Some((wd, i)) = ui.edit() else { return };
        let e = &mut ui.edits[i];
        e.focused_at = Instant::now();
        match key {
            "key.keyboard.backspace" => e.delete(-1),
            "key.keyboard.delete" => e.delete(1),
            "key.keyboard.left" => {
                let to = if e.cursor != e.anchor && !shift { e.selection().0 } else { e.cursor.saturating_sub(1) };
                e.move_to(to, shift);
            }
            "key.keyboard.right" => {
                let to = if e.cursor != e.anchor && !shift { e.selection().1 } else { e.cursor + 1 };
                e.move_to(to, shift);
            }
            "key.keyboard.home" => e.move_to(0, shift),
            "key.keyboard.end" => {
                let n = e.len();
                e.move_to(n, shift);
            }
            "key.keyboard.a" if ctrl => {
                e.anchor = 0;
                e.cursor = e.len();
            }
            _ => return,
        }
        (wd, e.value.clone())
    };
    apply_edit(g, wd, &value);
}

pub(super) fn on_char_typed(g: &Game<'_>, screen: &NativeScreenView<'_>, c: char, canceled: &mut bool) {
    if !ours(g, screen) {
        return;
    }
    *canceled = true;
    // StringUtil.isAllowedChatCharacter.
    if c == '\u{a7}' || c < ' ' || c == '\u{7f}' {
        return;
    }
    let (wd, value) = {
        let mut s = st();
        let Some(ui) = s.ui.as_mut() else { return };
        let Some((wd, i)) = ui.edit() else { return };
        let e = &mut ui.edits[i];
        e.focused_at = Instant::now();
        e.insert(&c.to_string());
        (wd, e.value.clone())
    };
    apply_edit(g, wd, &value);
}
