//! Zoomify's zoom (isXander/Zoomify, LGPL-3.0-or-later, see THIRD_PARTY_NOTICES.md): the
//! transitions, interpolators and `ZoomHelper` that turn the zoom key and the scroll steps into
//! the divisor the field of view, the hand, the mouse and the view bobbing are scaled by, plus
//! the spyglass rules, the cinematic camera and the sounds of `Zoomify.tick`.
//!
//! The helpers are ticked once per client tick and read once per frame with the partial tick,
//! as the mod does (`Zoomify.tick`, `Camera.calculateFov`).
use std::f64::consts::PI;

/// `TransitionType`: the curve a zoom follows between normal and zoomed.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Transition {
    Instant,
    Linear,
    EaseInSine,
    EaseOutSine,
    EaseInOutSine,
    EaseInQuad,
    EaseOutQuad,
    EaseInOutQuad,
    EaseInCubic,
    EaseOutCubic,
    EaseInOutCubic,
    EaseInExp,
    #[default]
    EaseOutExp,
    EaseInOutExp,
}

impl Transition {
    pub const ALL: [Self; 14] = [
        Self::Instant,
        Self::Linear,
        Self::EaseInSine,
        Self::EaseOutSine,
        Self::EaseInOutSine,
        Self::EaseInQuad,
        Self::EaseOutQuad,
        Self::EaseInOutQuad,
        Self::EaseInCubic,
        Self::EaseOutCubic,
        Self::EaseInOutCubic,
        Self::EaseInExp,
        Self::EaseOutExp,
        Self::EaseInOutExp,
    ];

    /// The name the mod's config writes (`getSerializedName`).
    pub fn name(self) -> &'static str {
        match self {
            Self::Instant => "instant",
            Self::Linear => "linear",
            Self::EaseInSine => "ease_in_sine",
            Self::EaseOutSine => "ease_out_sine",
            Self::EaseInOutSine => "ease_in_out_sine",
            Self::EaseInQuad => "ease_in_quad",
            Self::EaseOutQuad => "ease_out_quad",
            Self::EaseInOutQuad => "ease_in_out_quad",
            Self::EaseInCubic => "ease_in_cubic",
            Self::EaseOutCubic => "ease_out_cubic",
            Self::EaseInOutCubic => "ease_in_out_cubic",
            Self::EaseInExp => "ease_in_exp",
            Self::EaseOutExp => "ease_out_exp",
            Self::EaseInOutExp => "ease_in_out_exp",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Instant => "Instant",
            Self::Linear => "Linear",
            Self::EaseInSine => "Ease in sine",
            Self::EaseOutSine => "Ease out sine",
            Self::EaseInOutSine => "Ease in-out sine",
            Self::EaseInQuad => "Ease in quad",
            Self::EaseOutQuad => "Ease out quad",
            Self::EaseInOutQuad => "Ease in-out quad",
            Self::EaseInCubic => "Ease in cubic",
            Self::EaseOutCubic => "Ease out cubic",
            Self::EaseInOutCubic => "Ease in-out cubic",
            Self::EaseInExp => "Ease in exponential",
            Self::EaseOutExp => "Ease out exponential",
            Self::EaseInOutExp => "Ease in-out exponential",
        }
    }
    pub fn parse(s: &str, default: Self) -> Self {
        Self::ALL
            .into_iter()
            .find(|v| v.name() == s)
            .unwrap_or(default)
    }

    /// `apply`: the eased value of the linear progress `t`, 0 to 1.
    pub fn apply(self, t: f64) -> f64 {
        let c = 1023f64;
        match self {
            Self::Instant | Self::Linear => t,
            Self::EaseInSine => 1.0 - (t * PI / 2.0).cos(),
            Self::EaseOutSine => (t * PI / 2.0).sin(),
            Self::EaseInOutSine => -((PI * t).cos() - 1.0) / 2.0,
            Self::EaseInQuad => t * t,
            Self::EaseOutQuad => 1.0 - (1.0 - t) * (1.0 - t),
            Self::EaseInOutQuad => {
                if t < 0.5 {
                    2.0 * t * t
                } else {
                    1.0 - (-2.0 * t + 2.0).powi(2) / 2.0
                }
            }
            Self::EaseInCubic => t.powi(3),
            Self::EaseOutCubic => 1.0 - (1.0 - t).powi(3),
            Self::EaseInOutCubic => {
                if t < 0.5 {
                    4.0 * t * t * t
                } else {
                    1.0 - (-2.0 * t + 2.0).powi(3) / 2.0
                }
            }
            // The mod's `1/1023` is an integer division in Kotlin: 0.
            Self::EaseInExp => {
                if t == 0.0 {
                    0.0
                } else if t == 1.0 {
                    1.0
                } else {
                    2f64.powf(10.0 * t - c.log2())
                }
            }
            Self::EaseOutExp => {
                if t == 0.0 {
                    0.0
                } else if t == 1.0 {
                    1.0
                } else {
                    1.0 - 2f64.powf(10.0 - c.log2() - 10.0 * t)
                }
            }
            Self::EaseInOutExp => {
                if t == 0.0 {
                    0.0
                } else if t == 1.0 {
                    1.0
                } else if t < 0.5 {
                    2f64.powf(20.0 * t - c.log2())
                } else {
                    1.0 - 2f64.powf(10.0 - c.log2() - 10.0 * t)
                }
            }
        }
    }

    /// `inverse`, for the curves that have one.
    pub fn inverse(self, x: f64) -> Option<f64> {
        let c = 1023f64;
        Some(match self {
            Self::EaseInSine => (-(x - 1.0)).acos() * 2.0 / PI,
            Self::EaseOutSine => x.asin() * 2.0 / PI,
            Self::EaseInQuad => x.sqrt(),
            Self::EaseOutQuad => -((-(x - 1.0)).sqrt() - 1.0),
            Self::EaseInCubic => x.powf(1.0 / 3.0),
            Self::EaseOutCubic => -((-x + 1.0).powf(1.0 / 3.0)) + 1.0,
            Self::EaseInExp => {
                if x == 0.0 {
                    0.0
                } else if x == 1.0 {
                    1.0
                } else {
                    (c * x + 1.0).ln() / (10.0 * 2f64.ln())
                }
            }
            Self::EaseOutExp => {
                if x == 0.0 {
                    0.0
                } else if x == 1.0 {
                    1.0
                } else {
                    let c10 = 10.0 * 2f64.ln();
                    -(((-((c * x - 1024.0) / c)).ln() - c10 + c.ln()) / c10)
                }
            }
            _ => return None,
        })
    }
    fn has_inverse(self) -> bool {
        self.inverse(0.0).is_some()
    }

    /// `opposite`: the curve for going back out.
    pub fn opposite(self) -> Self {
        match self {
            Self::Instant => Self::Instant,
            Self::Linear => Self::Linear,
            Self::EaseInSine => Self::EaseOutSine,
            Self::EaseOutSine => Self::EaseInSine,
            Self::EaseInOutSine => Self::EaseInOutSine,
            Self::EaseInQuad => Self::EaseOutQuad,
            Self::EaseOutQuad => Self::EaseInQuad,
            Self::EaseInOutQuad => Self::EaseInOutQuad,
            Self::EaseInCubic => Self::EaseOutCubic,
            Self::EaseOutCubic => Self::EaseInCubic,
            Self::EaseInOutCubic => Self::EaseInOutCubic,
            Self::EaseInExp => Self::EaseOutExp,
            Self::EaseOutExp => Self::EaseInExp,
            Self::EaseInOutExp => Self::EaseInOutExp,
        }
    }
}

/// `ZoomKeyBehaviour`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum KeyBehaviour {
    #[default]
    Hold,
    Toggle,
}
impl KeyBehaviour {
    pub const ALL: [Self; 2] = [Self::Hold, Self::Toggle];
    pub fn name(self) -> &'static str {
        match self {
            Self::Hold => "hold",
            Self::Toggle => "toggle",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Hold => "Hold",
            Self::Toggle => "Toggle",
        }
    }
    pub fn parse(s: &str, default: Self) -> Self {
        Self::ALL
            .into_iter()
            .find(|v| v.name() == s)
            .unwrap_or(default)
    }
}

/// `SpyglassBehaviour`: how the spyglass and the zoom key combine.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Spyglass {
    /// The spyglass zooms and the zoom key adds to it.
    #[default]
    Combine,
    /// Scoping with a spyglass zooms by the zoom instead of the spyglass's own.
    Override,
    /// The zoom key works only while holding a spyglass.
    OnlyHolding,
    /// The zoom key works only while carrying a spyglass.
    OnlyCarrying,
}
impl Spyglass {
    pub const ALL: [Self; 4] = [
        Self::Combine,
        Self::Override,
        Self::OnlyHolding,
        Self::OnlyCarrying,
    ];
    pub fn name(self) -> &'static str {
        match self {
            Self::Combine => "combine",
            Self::Override => "override",
            Self::OnlyHolding => "only_zoom_while_holding",
            Self::OnlyCarrying => "only_zoom_while_carrying",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Combine => "Combine",
            Self::Override => "Override",
            Self::OnlyHolding => "Only while holding",
            Self::OnlyCarrying => "Only while carrying",
        }
    }
    pub fn parse(s: &str, default: Self) -> Self {
        Self::ALL
            .into_iter()
            .find(|v| v.name() == s)
            .unwrap_or(default)
    }
}

/// `OverlayVisibility`: when the spyglass's scope is drawn.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Overlay {
    Never,
    /// Scoping, or zooming while holding a spyglass (with a spyglass behaviour other than combine).
    #[default]
    Holding,
    /// Zooming while carrying a spyglass.
    Carrying,
    /// Whenever zooming.
    Always,
}
impl Overlay {
    pub const ALL: [Self; 4] = [Self::Never, Self::Holding, Self::Carrying, Self::Always];
    pub fn name(self) -> &'static str {
        match self {
            Self::Never => "never",
            Self::Holding => "holding",
            Self::Carrying => "carrying",
            Self::Always => "always",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Never => "Never",
            Self::Holding => "Holding",
            Self::Carrying => "Carrying",
            Self::Always => "Always",
        }
    }
    pub fn parse(s: &str, default: Self) -> Self {
        Self::ALL
            .into_iter()
            .find(|v| v.name() == s)
            .unwrap_or(default)
    }
}

/// `SoundBehaviour`: when the spyglass's use and stop sounds play.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SpyglassSound {
    Never,
    Always,
    OnlySpyglass,
    #[default]
    WithOverlay,
}
impl SpyglassSound {
    pub const ALL: [Self; 4] = [
        Self::Never,
        Self::Always,
        Self::OnlySpyglass,
        Self::WithOverlay,
    ];
    pub fn name(self) -> &'static str {
        match self {
            Self::Never => "never",
            Self::Always => "always",
            Self::OnlySpyglass => "only_spyglass",
            Self::WithOverlay => "with_overlay",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Never => "Never",
            Self::Always => "Always",
            Self::OnlySpyglass => "Only with a spyglass",
            Self::WithOverlay => "With the overlay",
        }
    }
    pub fn parse(s: &str, default: Self) -> Self {
        Self::ALL
            .into_iter()
            .find(|v| v.name() == s)
            .unwrap_or(default)
    }
}

/// The settings the helpers read (`ZoomifySettings`), at the mod's defaults.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Settings {
    pub initial_zoom: i32,
    pub zoom_in_time: f64,
    pub zoom_out_time: f64,
    pub zoom_in_transition: Transition,
    pub zoom_out_transition: Transition,
    pub affect_hand_fov: bool,
    pub retain_zoom_steps: bool,
    pub scroll_zoom: bool,
    pub scroll_step_count: i32,
    /// Percent: 150 is 1.5 times the zoom each step.
    pub zoom_per_step: i32,
    pub scroll_zoom_smoothness: i32,
    pub key_behaviour: KeyBehaviour,
    pub relative_sensitivity: i32,
    pub relative_view_bobbing: bool,
    pub secondary_zoom_amount: i32,
    pub secondary_zoom_in_time: f64,
    pub secondary_zoom_out_time: f64,
    pub spyglass: Spyglass,
    pub overlay: Overlay,
    pub spyglass_sound: SpyglassSound,
    /// The cinematic camera while zooming, 0 (off) to 100 percent: the mouse is smoothed more.
    pub cinematic: i32,
    /// The zoom in and out keys step the zoom instead of the wheel.
    pub keybind_scrolling: bool,
    /// The HUD hides while the secondary zoom is on.
    pub hide_hud_on_secondary: bool,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            initial_zoom: 4,
            zoom_in_time: 1.0,
            zoom_out_time: 0.5,
            zoom_in_transition: Transition::EaseOutExp,
            zoom_out_transition: Transition::EaseOutExp,
            affect_hand_fov: true,
            retain_zoom_steps: false,
            scroll_zoom: true,
            scroll_step_count: 10,
            zoom_per_step: 150,
            scroll_zoom_smoothness: 70,
            key_behaviour: KeyBehaviour::Hold,
            relative_sensitivity: 100,
            relative_view_bobbing: true,
            secondary_zoom_amount: 4,
            secondary_zoom_in_time: 10.0,
            secondary_zoom_out_time: 1.0,
            spyglass: Spyglass::Combine,
            overlay: Overlay::Holding,
            spyglass_sound: SpyglassSound::WithOverlay,
            cinematic: 0,
            keybind_scrolling: false,
            hide_hud_on_secondary: true,
        }
    }
}

fn lerp(delta: f64, start: f64, end: f64) -> f64 {
    start + delta * (end - start)
}

/// Which `Interpolator` a helper uses for one of its two values.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Kind {
    /// `InstantInterpolator`.
    Instant,
    /// `TransitionInterpolator` over the regular zoom's times and curves.
    Transition,
    /// `TimedInterpolator` over the secondary zoom's times.
    Timed,
    /// `SmoothInterpolator`, the scroll zoom's.
    Smooth,
}
/// Where a helper reads its times from.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Source {
    Regular,
    Secondary,
}

#[derive(Clone, Debug)]
struct Interpolator {
    kind: Kind,
    source: Source,
    going_in: bool,
    active: Transition,
    inactive: Transition,
    prev_target: f64,
    just_swapped: bool,
}
impl Interpolator {
    fn new(kind: Kind, source: Source, s: &Settings) -> Self {
        Self {
            kind,
            source,
            going_in: true,
            active: s.zoom_in_transition,
            inactive: s.zoom_out_transition.opposite(),
            prev_target: 0.0,
            just_swapped: false,
        }
    }
    fn time_in(&self, s: &Settings) -> f64 {
        match self.source {
            Source::Regular => s.zoom_in_time,
            Source::Secondary => s.secondary_zoom_in_time,
        }
    }
    fn time_out(&self, s: &Settings) -> f64 {
        match self.source {
            Source::Regular => s.zoom_out_time,
            Source::Secondary => s.secondary_zoom_out_time,
        }
    }
    fn smoothness(&self, s: &Settings) -> f64 {
        lerp(f64::from(s.scroll_zoom_smoothness) / 100.0, 1.0, 0.1)
    }
    fn transition_in(&self, s: &Settings) -> Transition {
        s.zoom_in_transition
    }
    /// `transitionOut`: the configured one's opposite.
    fn transition_out(&self, s: &Settings) -> Transition {
        s.zoom_out_transition.opposite()
    }

    fn increment(
        &self,
        zooming_out: bool,
        delta: f64,
        target: f64,
        current: f64,
        s: &Settings,
    ) -> f64 {
        match self.kind {
            Kind::Smooth => {
                let diff = if zooming_out {
                    current - target
                } else {
                    target - current
                };
                diff * self.smoothness(s) / 0.05 * delta
            }
            _ => {
                delta
                    / if zooming_out {
                        self.time_out(s)
                    } else {
                        self.time_in(s)
                    }
            }
        }
    }
    /// `LinearInterpolator.tickInterpolation`.
    fn linear(&mut self, target: f64, current: f64, delta: f64, s: &Settings) -> f64 {
        if target > current {
            self.going_in = true;
            (current + self.increment(false, delta, target, current, s)).min(target)
        } else if target < current {
            self.going_in = false;
            (current - self.increment(true, delta, target, current, s)).max(target)
        } else {
            self.going_in = true;
            target
        }
    }
    fn tick(&mut self, target: f64, current: f64, delta: f64, s: &Settings) -> f64 {
        match self.kind {
            Kind::Instant => target,
            Kind::Timed => self.linear(target, current, delta, s),
            Kind::Smooth => {
                if !self.is_smooth(s) {
                    return target;
                }
                self.linear(target, current, delta, s)
            }
            Kind::Transition => {
                let mut current_mod = current;
                if target > current {
                    self.active = self.transition_in(s);
                    self.inactive = self.transition_out(s);
                    if self.prev_target < target && self.active.has_inverse() {
                        self.just_swapped = true;
                        current_mod = self
                            .active
                            .inverse(self.inactive.apply(current_mod))
                            .unwrap_or(current_mod);
                    }
                } else if target < current {
                    self.active = self.transition_out(s);
                    self.inactive = self.transition_in(s);
                    if self.prev_target > target && self.active.has_inverse() {
                        self.just_swapped = true;
                        current_mod = self
                            .active
                            .inverse(self.inactive.apply(current_mod))
                            .unwrap_or(current_mod);
                    }
                }
                self.prev_target = target;
                if self.active == Transition::Instant {
                    return target;
                }
                self.linear(target, current_mod, delta, s)
            }
        }
    }
    fn modify(&self, interpolation: f64) -> f64 {
        match self.kind {
            Kind::Transition => self.active.apply(interpolation),
            _ => interpolation,
        }
    }
    fn modify_prev(&mut self, interpolation: f64) -> f64 {
        if self.kind == Kind::Transition && self.just_swapped {
            self.just_swapped = false;
            return self
                .active
                .inverse(self.inactive.apply(interpolation))
                .unwrap_or(interpolation);
        }
        interpolation
    }
    fn is_smooth(&self, s: &Settings) -> bool {
        match self.kind {
            Kind::Instant => false,
            Kind::Smooth => self.smoothness(s) != 1.0,
            Kind::Timed => self.timed_smooth(s),
            Kind::Transition => {
                !self.just_swapped && self.timed_smooth(s) && self.active != Transition::Instant
            }
        }
    }
    fn timed_smooth(&self, s: &Settings) -> bool {
        (self.going_in && self.time_in(s) > 0.0) || (!self.going_in && self.time_out(s) > 0.0)
    }
}

/// `ZoomHelper`: the zoom key's interpolation (`initial`) and the scroll steps' (`scroll`).
#[derive(Clone, Debug)]
pub struct ZoomHelper {
    initial: Interpolator,
    scroll: Interpolator,
    secondary: bool,
    prev_initial: f64,
    initial_t: f64,
    zooming_last_tick: bool,
    prev_scroll: f64,
    scroll_t: f64,
    last_scroll_tier: i32,
    resetting: bool,
    reset_multiplier: f64,
}
impl ZoomHelper {
    /// `RegularZoomHelper`.
    pub fn regular(s: &Settings) -> Self {
        Self::build(
            Interpolator::new(Kind::Transition, Source::Regular, s),
            Interpolator::new(Kind::Smooth, Source::Regular, s),
            false,
        )
    }
    /// `SecondaryZoomHelper`.
    pub fn secondary(s: &Settings) -> Self {
        Self::build(
            Interpolator::new(Kind::Timed, Source::Secondary, s),
            Interpolator::new(Kind::Instant, Source::Secondary, s),
            true,
        )
    }
    fn build(initial: Interpolator, scroll: Interpolator, secondary: bool) -> Self {
        Self {
            initial,
            scroll,
            secondary,
            prev_initial: 0.0,
            initial_t: 0.0,
            zooming_last_tick: false,
            prev_scroll: 0.0,
            scroll_t: 0.0,
            last_scroll_tier: 0,
            resetting: false,
            reset_multiplier: 0.0,
        }
    }
    fn initial_zoom(&self, s: &Settings) -> i32 {
        if self.secondary {
            s.secondary_zoom_amount
        } else {
            s.initial_zoom
        }
    }
    fn zoom_per_step(&self, s: &Settings) -> i32 {
        if self.secondary { 100 } else { s.zoom_per_step }
    }
    fn max_scroll_tiers(&self, s: &Settings) -> i32 {
        if self.secondary {
            0
        } else {
            s.scroll_step_count
        }
    }
    /// Scrolling out stops at normal field of view.
    pub fn min_scroll_tiers(&self, s: &Settings) -> i32 {
        let step = f64::from(self.zoom_per_step(s)) / 100.0;
        if step <= 1.0 {
            return 0;
        }
        let steps = (f64::from(self.initial_zoom(s)).ln() / step.ln()).ceil() as i32;
        -steps.clamp(0, self.max_scroll_tiers(s))
    }

    /// `tick`: one client tick (`last_frame_duration` is the mod's default 0.05 s).
    pub fn tick(&mut self, zooming: bool, scroll_tiers: i32, s: &Settings) {
        let delta = 0.05;
        // The initial zoom.
        if zooming && !self.zooming_last_tick {
            self.resetting = false;
        }
        let target = if zooming { 1.0 } else { 0.0 };
        self.prev_initial = self.initial_t;
        self.initial_t = self.initial.tick(target, self.initial_t, delta, s);
        self.prev_initial = self.initial.modify_prev(self.prev_initial);
        if !self.initial.is_smooth(s) {
            self.prev_initial = self.initial_t;
        }
        self.zooming_last_tick = zooming;
        // The scroll steps.
        if scroll_tiers.abs() > self.last_scroll_tier.abs() {
            self.resetting = false;
        }
        let max = self.max_scroll_tiers(s);
        let target = if max > 0 {
            f64::from(scroll_tiers) / f64::from(max)
        } else {
            0.0
        };
        self.prev_scroll = self.scroll_t;
        self.scroll_t = self.scroll.tick(target, self.scroll_t, delta, s);
        self.prev_scroll = self.scroll.modify_prev(self.prev_scroll);
        if !self.initial.is_smooth(s) {
            self.prev_initial = self.initial_t;
        }
        self.last_scroll_tier = scroll_tiers;
    }

    /// `getZoomDivisor(tickDelta)`, 1 when not zooming.
    pub fn divisor(&mut self, partial: f64, s: &Settings) -> f64 {
        let multiplier = self.initial_multiplier(partial, s);
        let base = 1.0 / multiplier;
        let scroll_t = if self.resetting {
            0.0
        } else if self.scroll.is_smooth(s) {
            self.scroll
                .modify(lerp(partial, self.prev_scroll, self.scroll_t))
        } else {
            self.scroll_t
        };
        let step = f64::from(self.zoom_per_step(s)) / 100.0;
        let current_step = scroll_t * f64::from(self.max_scroll_tiers(s));
        // Each step multiplies the divisor by the step size; negative steps zoom out below
        // the initial zoom. Limited to 1x to 500x as the mod does.
        let raw = base * step.powf(current_step);
        let divisor = raw.clamp(1.0, 500.0);
        if self.initial_t == 0.0 && self.scroll_t == 0.0 {
            self.resetting = false;
        }
        if !self.resetting {
            self.reset_multiplier = 1.0 / divisor;
        }
        divisor
    }
    fn initial_multiplier(&self, partial: f64, s: &Settings) -> f64 {
        let t = if self.initial.is_smooth(s) {
            self.initial
                .modify(lerp(partial, self.prev_initial, self.initial_t))
        } else {
            self.initial_t
        };
        lerp(
            t,
            1.0,
            if self.resetting {
                self.reset_multiplier
            } else {
                1.0 / f64::from(self.initial_zoom(s))
            },
        )
    }

    /// `reset`: the key is up, so the scroll steps fall away with the zoom.
    pub fn reset(&mut self) {
        if !self.resetting && self.scroll_t != 0.0 {
            self.resetting = true;
            self.scroll_t = 0.0;
            self.prev_scroll = 0.0;
        }
    }
    /// `setToZero`.
    pub fn set_to_zero(&mut self) {
        self.initial_t = 0.0;
        self.prev_initial = 0.0;
        self.zooming_last_tick = false;
        self.scroll_t = 0.0;
        self.prev_scroll = 0.0;
        self.last_scroll_tier = 0;
        self.resetting = false;
    }
}

/// What the player holds and does this tick, for the spyglass rules.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Hands {
    /// `isScoping`: using a spyglass.
    pub scoping: bool,
    /// The first-person camera.
    pub first_person: bool,
    /// A spyglass in the main or the off hand (`isHolding`).
    pub holding: bool,
    /// A spyglass anywhere in the inventory.
    pub carrying: bool,
}

/// The spyglass sounds a tick starts or stops, which the game plays at the player.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SpyglassSounds {
    pub start: bool,
    pub stop: bool,
}

/// The zoom as the game keeps it: the zoom key's state, the scroll steps and both helpers, with
/// the spyglass rules and the sounds of `Zoomify`'s tick.
#[derive(Clone, Debug)]
pub struct State {
    regular: ZoomHelper,
    secondary: ZoomHelper,
    /// The zoom key is down (hold).
    held: bool,
    /// The zoom key was pressed since the last tick (toggle).
    pressed: bool,
    /// Zoomify's `zooming`: the regular zoom is on this tick, after the spyglass rules.
    zooming: bool,
    secondary_on: bool,
    scroll_steps: i32,
    /// The regular zoom's last divisor (`previousZoomDivisor`): the mouse and the bobbing scale by it.
    previous: f64,
    /// The spyglass sounds were on last tick (`shouldPlaySound`).
    sound_on: bool,
}
impl Default for State {
    fn default() -> Self {
        let s = Settings::default();
        Self {
            regular: ZoomHelper::regular(&s),
            secondary: ZoomHelper::secondary(&s),
            held: false,
            pressed: false,
            zooming: false,
            secondary_on: false,
            scroll_steps: 0,
            previous: 1.0,
            sound_on: false,
        }
    }
}
impl State {
    /// The regular zoom is on this tick.
    pub fn zooming(&self) -> bool {
        self.zooming
    }
    /// The secondary zoom is on.
    pub fn secondary_on(&self) -> bool {
        self.secondary_on
    }
    /// The zoom key went down or up. A press while `playing` zooms (or flips a toggled zoom);
    /// the release, or a press behind a screen, lets a held zoom go.
    pub fn key(&mut self, pressed: bool, repeat: bool, playing: bool) {
        if !pressed || !playing {
            self.held = false;
        } else if !repeat {
            self.held = true;
            self.pressed = true;
        }
    }
    /// The secondary zoom key was pressed while playing.
    pub fn secondary_key(&mut self, playing: bool) {
        if playing {
            self.secondary_on = !self.secondary_on;
        }
    }
    /// A screen took the zoom key: a held zoom lets go, a toggled one stays on.
    pub fn release(&mut self) {
        self.held = false;
    }
    /// The extra is off: nothing zooms and the steps are forgotten.
    pub fn clear(&mut self) {
        self.held = false;
        self.pressed = false;
        self.zooming = false;
        self.secondary_on = false;
        self.scroll_steps = 0;
        self.sound_on = false;
        self.previous = 1.0;
        self.regular.set_to_zero();
        self.secondary.set_to_zero();
    }
    /// A wheel scroll while zooming takes a step (`Zoomify.mouseZoom`): true when it did, so
    /// the hotbar doesn't scroll. With the keys stepping the zoom, the wheel is the hotbar's.
    pub fn scroll(&mut self, delta: f32, s: &Settings) -> bool {
        if !s.scroll_zoom || s.keybind_scrolling || !self.zooming || delta == 0.0 {
            return false;
        }
        self.step(delta > 0.0, s);
        true
    }
    /// One step in or out, by the wheel or by the zoom in and out keys (with keybind scrolling).
    pub fn step(&mut self, up: bool, s: &Settings) {
        let next = self.scroll_steps + if up { 1 } else { -1 };
        self.scroll_steps = next.clamp(self.regular.min_scroll_tiers(s), s.scroll_step_count);
    }
    /// One client tick (`Zoomify.tick`): the zoom key, the spyglass rules and the sounds they
    /// make, then both helpers.
    pub fn tick(&mut self, s: &Settings, hands: Hands) -> SpyglassSounds {
        let before = self.zooming;
        match s.key_behaviour {
            KeyBehaviour::Hold => self.zooming = self.held,
            KeyBehaviour::Toggle => {
                if self.pressed {
                    self.zooming = !self.zooming;
                }
            }
        }
        self.pressed = false;
        let spyglass = s.spyglass != Spyglass::Combine;
        match s.spyglass {
            Spyglass::Combine => {}
            Spyglass::Override => {
                if hands.scoping {
                    self.zooming = self.zooming && hands.first_person;
                }
            }
            Spyglass::OnlyHolding => {
                if !hands.holding {
                    self.zooming = false;
                }
            }
            Spyglass::OnlyCarrying => {
                if !hands.carrying {
                    self.zooming = false;
                }
            }
        }
        if spyglass && hands.scoping {
            self.zooming = true;
        }
        let mut sounds = SpyglassSounds::default();
        if self.sound_on && before && !self.zooming {
            sounds.stop = true;
        }
        self.sound_on = match s.spyglass_sound {
            SpyglassSound::Never => false,
            SpyglassSound::Always => true,
            SpyglassSound::OnlySpyglass => {
                hands.scoping || (spyglass && self.zooming && hands.holding)
            }
            SpyglassSound::WithOverlay => {
                spyglass && self.overlay(s, hands, hands.first_person && hands.scoping)
            }
        };
        if self.sound_on && !before && self.zooming {
            sounds.start = true;
        }
        let steps = self.scroll_steps;
        self.regular.tick(self.zooming, steps, s);
        self.secondary.tick(self.secondary_on, steps, s);
        sounds
    }
    /// Whether the spyglass's scope is drawn (`Zoomify.shouldRenderOverlay`). `using` is the
    /// vanilla `isScoping` the HUD asks about (or scoping in first person for the sounds).
    pub fn overlay(&self, s: &Settings, hands: Hands, using: bool) -> bool {
        match s.overlay {
            Overlay::Never => false,
            Overlay::Always => self.zooming,
            Overlay::Holding => {
                using || (self.zooming && hands.holding && s.spyglass != Spyglass::Combine)
            }
            Overlay::Carrying => self.zooming && hands.carrying,
        }
    }
    /// Whether vanilla's spyglass use and stop sounds play (`SpyglassItemMixin`).
    pub fn vanilla_sound(&self, s: &Settings) -> bool {
        match s.spyglass_sound {
            SpyglassSound::Never => false,
            SpyglassSound::WithOverlay => s.overlay != Overlay::Never,
            _ => true,
        }
    }
    /// The cinematic camera's fraction (`modifyCinematicSmoothness`): the mouse's smoothing is
    /// divided by it while zooming, and 1 when it is off.
    pub fn cinematic_fraction(&self, s: &Settings) -> f64 {
        if self.zooming && s.cinematic > 0 {
            f64::from(s.cinematic) / 100.0
        } else {
            1.0
        }
    }
    /// The mouse is smoothed whatever the player's setting: the cinematic camera while zooming,
    /// and the secondary zoom (`smoothCameraIfZoom`).
    pub fn smooths_camera(&self, s: &Settings) -> bool {
        self.secondary_on || (self.zooming && s.cinematic > 0)
    }
    /// The HUD hides while the secondary zoom is on, if the setting says so.
    pub fn hides_hud(&self, s: &Settings) -> bool {
        self.secondary_on && s.hide_hud_on_secondary
    }
    /// The divisor the field of view is zoomed by this frame (`Zoomify.getZoomDivisor`): 1
    /// unless zooming.
    pub fn divisor(&mut self, partial: f32, s: &Settings) -> f64 {
        if !self.zooming {
            if !s.retain_zoom_steps {
                self.scroll_steps = 0;
            }
            self.regular.reset();
        }
        let regular = self.regular.divisor(f64::from(partial), s);
        self.previous = regular;
        regular * self.secondary.divisor(f64::from(partial), s)
    }
    /// The regular zoom's last divisor (`previousZoomDivisor`).
    pub fn previous(&self) -> f64 {
        self.previous
    }
    /// The scroll steps taken so far.
    pub fn steps(&self) -> i32 {
        self.scroll_steps
    }
}

/// The mouse sensitivity divisor: `Mth.lerp(relativeSensitivity / 100, 1, previousZoomDivisor)`.
pub fn sensitivity_divisor(previous: f64, s: &Settings) -> f64 {
    lerp(f64::from(s.relative_sensitivity) / 100.0, 1.0, previous)
}
/// The view bobbing divisor: `Mth.lerp(0.2, 1, previousZoomDivisor)`, when it is relative.
pub fn bobbing_divisor(previous: f64, s: &Settings) -> f64 {
    if s.relative_view_bobbing {
        lerp(0.2, 1.0, previous)
    } else {
        1.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(h: &mut ZoomHelper, s: &Settings, zooming: bool, ticks: usize) -> f64 {
        for _ in 0..ticks {
            h.tick(zooming, 0, s);
        }
        h.divisor(1.0, s)
    }

    /// A player holding the zoom key, not scoping, with nothing else in hand.
    fn playing() -> Hands {
        Hands {
            first_person: true,
            ..Hands::default()
        }
    }

    #[test]
    fn not_zooming_is_exactly_normal() {
        let s = Settings::default();
        let mut h = ZoomHelper::regular(&s);
        assert_eq!(h.divisor(0.5, &s), 1.0);
        assert_eq!(run(&mut h, &s, false, 40), 1.0);
    }

    #[test]
    fn the_key_eases_in_to_the_initial_zoom() {
        let s = Settings::default();
        let mut h = ZoomHelper::regular(&s);
        // One second in (20 ticks) of 1 s: arrives at 4x.
        let d = run(&mut h, &s, true, 21);
        assert!((d - 4.0).abs() < 1e-9, "{d}");
        // Half way through, an ease out exponential is nearly there.
        let mut h = ZoomHelper::regular(&s);
        for _ in 0..10 {
            h.tick(true, 0, &s);
        }
        let mid = h.divisor(1.0, &s);
        assert!(mid > 3.0 && mid < 4.0, "{mid}");
        // Out again: half a second.
        for _ in 0..11 {
            h.tick(false, 0, &s);
        }
        assert!((h.divisor(1.0, &s) - 1.0).abs() < 1e-9);
    }

    #[test]
    fn instant_transitions_jump() {
        let s = Settings {
            zoom_in_transition: Transition::Instant,
            zoom_out_transition: Transition::Instant,
            ..Settings::default()
        };
        let mut h = ZoomHelper::regular(&s);
        h.tick(true, 0, &s);
        assert!((h.divisor(1.0, &s) - 4.0).abs() < 1e-9);
        h.tick(false, 0, &s);
        assert_eq!(h.divisor(1.0, &s), 1.0);
    }

    #[test]
    fn scroll_steps_multiply_by_the_step_size() {
        let s = Settings::default();
        let mut h = ZoomHelper::regular(&s);
        for _ in 0..40 {
            h.tick(true, 2, &s);
        }
        // Two steps of 1.5 on 4x, once the smoothing has settled.
        let d = h.divisor(1.0, &s);
        assert!((d - 4.0 * 1.5 * 1.5).abs() < 1e-3, "{d}");
        // Scrolling out all the way stops at normal.
        let min = h.min_scroll_tiers(&s);
        assert_eq!(min, -4, "ceil(ln 4 / ln 1.5) steps reach normal fov");
        for _ in 0..80 {
            h.tick(true, min, &s);
        }
        assert!((h.divisor(1.0, &s) - 1.0).abs() < 1e-2);
    }

    #[test]
    fn the_wheel_steps_only_while_zooming() {
        let s = Settings::default();
        let mut z = State::default();
        assert!(
            !z.scroll(1.0, &s),
            "the wheel is the hotbar's when not zooming"
        );
        z.key(true, false, true);
        z.tick(&s, playing());
        assert!(z.scroll(1.0, &s) && z.scroll(1.0, &s));
        assert_eq!(z.steps(), 2);
        assert!(z.scroll(-1.0, &s));
        assert_eq!(z.steps(), 1);
        // With the keys stepping the zoom instead, the wheel is the hotbar's.
        let keys = Settings {
            keybind_scrolling: true,
            ..s
        };
        assert!(!z.scroll(1.0, &keys));
        z.step(true, &keys);
        assert_eq!(z.steps(), 2);
    }

    #[test]
    fn releasing_the_key_drops_the_scroll_with_it() {
        let s = Settings::default();
        let mut z = State::default();
        z.key(true, false, true);
        for _ in 0..30 {
            z.tick(&s, playing());
        }
        assert!(z.scroll(1.0, &s) && z.scroll(1.0, &s));
        assert_eq!(z.steps(), 2);
        for _ in 0..40 {
            z.tick(&s, playing());
        }
        assert!(z.divisor(1.0, &s) > 8.0);
        z.key(false, false, true);
        z.tick(&s, playing());
        z.divisor(1.0, &s);
        assert_eq!(z.steps(), 0, "the steps are forgotten");
        for _ in 0..40 {
            z.tick(&s, playing());
        }
        assert!((z.divisor(1.0, &s) - 1.0).abs() < 1e-9);
    }

    #[test]
    fn retained_steps_survive_the_key() {
        let s = Settings {
            retain_zoom_steps: true,
            ..Settings::default()
        };
        let mut z = State::default();
        z.key(true, false, true);
        z.tick(&s, playing());
        z.scroll(1.0, &s);
        z.key(false, false, true);
        z.tick(&s, playing());
        z.divisor(1.0, &s);
        assert_eq!(z.steps(), 1);
    }

    #[test]
    fn a_press_behind_a_screen_does_not_zoom() {
        let s = Settings::default();
        let mut z = State::default();
        z.key(true, false, false);
        z.tick(&s, playing());
        assert!(!z.zooming());
    }

    #[test]
    fn toggle_keeps_the_zoom_until_pressed_again() {
        let s = Settings {
            key_behaviour: KeyBehaviour::Toggle,
            ..Settings::default()
        };
        let mut z = State::default();
        z.key(true, false, true);
        z.key(false, false, true);
        z.tick(&s, playing());
        assert!(z.zooming());
        z.release();
        z.tick(&s, playing());
        assert!(z.zooming(), "a screen opening does not end a toggled zoom");
        z.key(true, false, true);
        z.tick(&s, playing());
        assert!(!z.zooming());
        // Key repeat doesn't toggle.
        z.key(true, true, true);
        z.tick(&s, playing());
        assert!(!z.zooming());
    }

    #[test]
    fn the_secondary_zoom_is_slow_and_scroll_free() {
        let s = Settings::default();
        let mut z = State::default();
        z.secondary_key(true);
        for _ in 0..100 {
            z.tick(&s, playing());
        }
        let d = z.divisor(1.0, &s);
        assert!(d > 1.0 && d < 4.0, "10 s in, 5 s so far: {d}");
        for _ in 0..200 {
            z.tick(&s, playing());
        }
        assert!((z.divisor(1.0, &s) - 4.0).abs() < 1e-9);
        assert_eq!(z.previous(), 1.0, "the regular zoom is untouched");
        assert!(z.smooths_camera(&s) && z.hides_hud(&s));
        z.secondary_key(true);
        assert!(!z.hides_hud(&s));
    }

    #[test]
    fn a_spyglass_zooms_on_its_own_unless_the_zoom_key_is_needed() {
        let scoping = Hands {
            scoping: true,
            first_person: true,
            holding: true,
            carrying: true,
        };
        let idle = Hands {
            first_person: true,
            ..Hands::default()
        };
        // Combine: the spyglass alone does not zoom with the regular zoom.
        let s = Settings::default();
        let mut z = State::default();
        z.tick(&s, scoping);
        assert!(!z.zooming());
        // Only while holding: the spyglass zooms, the key without one does not.
        let holding = Settings {
            spyglass: Spyglass::OnlyHolding,
            ..Settings::default()
        };
        let mut z = State::default();
        z.key(true, false, true);
        z.tick(&holding, idle);
        assert!(!z.zooming());
        z.tick(&holding, scoping);
        assert!(z.zooming(), "scoping forces the zoom");
        // Override: the first-person check clears the key's zoom, and scoping forces the spyglass
        // zoom back on (`requiresSpyglass`), in third person too, as the mod's order leaves it.
        let over = Settings {
            spyglass: Spyglass::Override,
            ..Settings::default()
        };
        let mut z = State::default();
        z.key(true, false, true);
        z.tick(
            &over,
            Hands {
                first_person: false,
                ..scoping
            },
        );
        assert!(z.zooming());
        z.tick(
            &over,
            Hands {
                first_person: false,
                scoping: false,
                ..scoping
            },
        );
        assert!(z.zooming(), "the key still holds the zoom without scoping");
    }

    #[test]
    fn the_spyglass_sounds_follow_their_behaviour() {
        let scoping = Hands {
            scoping: true,
            first_person: true,
            holding: true,
            carrying: true,
        };
        let s = Settings {
            spyglass: Spyglass::OnlyHolding,
            spyglass_sound: SpyglassSound::Always,
            ..Settings::default()
        };
        let mut z = State::default();
        z.tick(&s, Hands::default());
        z.key(true, false, true);
        assert!(z.tick(&s, scoping).start, "the zoom starts with a sound");
        z.key(false, false, true);
        assert!(z.tick(&s, Hands::default()).stop, "and stops with one");
        // Never: no sound, and the vanilla ones are off too.
        let never = Settings {
            spyglass_sound: SpyglassSound::Never,
            ..s
        };
        assert!(!State::default().vanilla_sound(&never));
        assert!(State::default().vanilla_sound(&Settings::default()));
    }

    #[test]
    fn the_overlay_follows_its_setting() {
        let s = Settings::default();
        let z = State::default();
        let hands = Hands {
            first_person: true,
            holding: true,
            carrying: true,
            ..Hands::default()
        };
        assert!(z.overlay(&s, hands, true), "vanilla's scope while scoping");
        assert!(!z.overlay(&s, hands, false));
        let never = Settings {
            overlay: Overlay::Never,
            ..s
        };
        assert!(!z.overlay(&never, hands, true));
        let mut zooming = State::default();
        zooming.key(true, false, true);
        zooming.tick(
            &Settings {
                overlay: Overlay::Always,
                ..s
            },
            hands,
        );
        assert!(zooming.overlay(
            &Settings {
                overlay: Overlay::Always,
                ..s
            },
            hands,
            false
        ));
    }

    #[test]
    fn the_cinematic_camera_smooths_while_zooming() {
        let s = Settings {
            cinematic: 50,
            ..Settings::default()
        };
        let mut z = State::default();
        assert_eq!(z.cinematic_fraction(&s), 1.0);
        assert!(!z.smooths_camera(&s));
        z.key(true, false, true);
        z.tick(&s, playing());
        assert_eq!(z.cinematic_fraction(&s), 0.5);
        assert!(z.smooths_camera(&s));
        let off = Settings { cinematic: 0, ..s };
        assert!(!z.smooths_camera(&off));
    }

    #[test]
    fn transitions_invert_where_the_mod_defines_it() {
        for t in Transition::ALL {
            assert_eq!(t.apply(0.0), 0.0, "{t:?}");
            assert!((t.apply(1.0) - 1.0).abs() < 1e-9, "{t:?}");
            // The exponential curves' inverses assume a `- 1/1023` the mod's integer division
            // drops from `apply`, so they are only the mod's own, not true inverses.
            let exp = matches!(t, Transition::EaseInExp | Transition::EaseOutExp);
            if t.inverse(0.0).is_some() && !exp {
                for x in [0.1, 0.35, 0.6, 0.9] {
                    let back = t.inverse(t.apply(x)).unwrap();
                    assert!((back - x).abs() < 1e-6, "{t:?} at {x}: {back}");
                }
            }
            assert_eq!(Transition::parse(t.name(), Transition::Linear), t);
            assert_eq!(t.opposite().opposite(), t);
        }
        assert_eq!(Transition::Instant.inverse(0.5), None);
        assert_eq!(Transition::EaseInOutQuad.inverse(0.5), None);
    }

    #[test]
    fn relative_sensitivity_and_bobbing() {
        let s = Settings::default();
        assert_eq!(sensitivity_divisor(4.0, &s), 4.0);
        let none = Settings {
            relative_sensitivity: 0,
            ..s
        };
        assert_eq!(sensitivity_divisor(4.0, &none), 1.0);
        assert!((bobbing_divisor(4.0, &s) - 1.6).abs() < 1e-12);
        let off = Settings {
            relative_view_bobbing: false,
            ..s
        };
        assert_eq!(bobbing_divisor(4.0, &off), 1.0);
    }

    #[test]
    fn names_round_trip() {
        for v in Spyglass::ALL {
            assert_eq!(Spyglass::parse(v.name(), Spyglass::Combine), v);
        }
        for v in Overlay::ALL {
            assert_eq!(Overlay::parse(v.name(), Overlay::Holding), v);
        }
        for v in SpyglassSound::ALL {
            assert_eq!(
                SpyglassSound::parse(v.name(), SpyglassSound::WithOverlay),
                v
            );
        }
        assert_eq!(
            KeyBehaviour::parse("toggle", KeyBehaviour::Hold),
            KeyBehaviour::Toggle
        );
    }
}
