//! Zoomify's zoom (isXander/Zoomify, LGPL-3.0-or-later, see THIRD_PARTY_NOTICES.md): the
//! transitions, interpolators and `ZoomHelper` that turn the zoom key and the scroll steps into
//! the divisor the field of view, the hand, the mouse and the view bobbing are scaled by.
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
        Self::ALL.into_iter().find(|v| v.name() == s).unwrap_or(default)
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
        Self::ALL.into_iter().find(|v| v.name() == s).unwrap_or(default)
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

    fn increment(&self, zooming_out: bool, delta: f64, target: f64, current: f64, s: &Settings) -> f64 {
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
        if self.secondary { 0 } else { s.scroll_step_count }
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

/// The zoom as the game keeps it: the key's state, the scroll steps and both helpers.
#[derive(Clone, Debug)]
pub struct State {
    regular: ZoomHelper,
    secondary: ZoomHelper,
    held: bool,
    toggled: bool,
    pub secondary_on: bool,
    scroll_steps: i32,
    /// The last divisor the regular zoom gave (`previousZoomDivisor`): the mouse and the view
    /// bobbing scale by it.
    previous: f64,
}
impl Default for State {
    fn default() -> Self {
        let s = Settings::default();
        Self {
            regular: ZoomHelper::regular(&s),
            secondary: ZoomHelper::secondary(&s),
            held: false,
            toggled: false,
            secondary_on: false,
            scroll_steps: 0,
            previous: 1.0,
        }
    }
}
impl State {
    pub fn zooming(&self, s: &Settings) -> bool {
        match s.key_behaviour {
            KeyBehaviour::Hold => self.held,
            KeyBehaviour::Toggle => self.toggled,
        }
    }
    /// The zoom key went down or up. A press while `playing` zooms (or toggles); anywhere else
    /// the key belongs to the screen.
    pub fn key(&mut self, pressed: bool, repeat: bool, playing: bool) {
        if !pressed || !playing {
            self.held = false;
        } else if !repeat {
            self.held = true;
            self.toggled = !self.toggled;
        }
    }
    /// The secondary zoom key went down.
    pub fn secondary_key(&mut self, playing: bool) {
        if playing {
            self.secondary_on = !self.secondary_on;
        }
    }
    /// Lets go of what the screen took: the held key (a toggled zoom stays on).
    pub fn release(&mut self) {
        self.held = false;
    }
    /// The extra went off: nothing zooms.
    pub fn clear(&mut self) {
        self.held = false;
        self.toggled = false;
        self.secondary_on = false;
        self.scroll_steps = 0;
        self.regular.set_to_zero();
        self.secondary.set_to_zero();
        self.previous = 1.0;
    }
    /// A wheel scroll (`Zoomify.mouseZoom`): true when it moved a step, taking it from the
    /// hotbar. Every scroll is one step, whatever its size.
    pub fn scroll(&mut self, delta: f32, s: &Settings) -> bool {
        if !s.scroll_zoom || !self.zooming(s) || delta == 0.0 {
            return false;
        }
        self.scroll_steps += if delta > 0.0 { 1 } else { -1 };
        self.scroll_steps = self
            .scroll_steps
            .clamp(self.regular.min_scroll_tiers(s), s.scroll_step_count);
        true
    }
    /// One client tick.
    pub fn tick(&mut self, s: &Settings) {
        let zooming = self.zooming(s);
        self.regular.tick(zooming, self.scroll_steps, s);
        self.secondary.tick(self.secondary_on, self.scroll_steps, s);
    }
    /// The divisor for the frame (`Zoomify.getZoomDivisor`): 1 unless zooming.
    pub fn divisor(&mut self, partial: f32, s: &Settings) -> f64 {
        if !self.zooming(s) {
            if !s.retain_zoom_steps {
                self.scroll_steps = 0;
            }
            self.regular.reset();
        }
        let regular = self.regular.divisor(f64::from(partial), s);
        self.previous = regular;
        regular * self.secondary.divisor(f64::from(partial), s)
    }
    /// The regular zoom's last divisor.
    pub fn previous(&self) -> f64 {
        self.previous
    }
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
    fn releasing_the_key_drops_the_scroll_with_it() {
        let s = Settings::default();
        let mut z = State::default();
        z.key(true, false, true);
        for _ in 0..30 {
            z.tick(&s);
        }
        assert!(z.scroll(1.0, &s) && z.scroll(1.0, &s));
        assert_eq!(z.steps(), 2);
        for _ in 0..40 {
            z.tick(&s);
        }
        assert!(z.divisor(1.0, &s) > 8.0);
        z.key(false, false, true);
        assert_eq!(z.divisor(1.0, &s), z.divisor(1.0, &s));
        assert_eq!(z.steps(), 0, "the steps are forgotten");
        for _ in 0..40 {
            z.tick(&s);
        }
        assert!((z.divisor(1.0, &s) - 1.0).abs() < 1e-9);
        // The wheel is the hotbar's when not zooming.
        assert!(!z.scroll(1.0, &s));
    }

    #[test]
    fn retained_steps_survive_the_key() {
        let s = Settings {
            retain_zoom_steps: true,
            ..Settings::default()
        };
        let mut z = State::default();
        z.key(true, false, true);
        z.scroll(1.0, &s);
        z.key(false, false, true);
        z.divisor(1.0, &s);
        assert_eq!(z.steps(), 1);
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
        assert!(z.zooming(&s));
        z.release();
        assert!(z.zooming(&s), "a screen opening does not end a toggled zoom");
        z.key(true, false, true);
        assert!(!z.zooming(&s));
        // Key repeat doesn't toggle.
        z.key(true, true, true);
        assert!(!z.zooming(&s));
    }

    #[test]
    fn the_secondary_zoom_is_slow_and_scroll_free() {
        let s = Settings::default();
        let mut z = State::default();
        z.secondary_key(true);
        for _ in 0..100 {
            z.tick(&s);
        }
        let d = z.divisor(1.0, &s);
        assert!(d > 1.0 && d < 4.0, "10 s in, 5 s so far: {d}");
        for _ in 0..200 {
            z.tick(&s);
        }
        assert!((z.divisor(1.0, &s) - 4.0).abs() < 1e-9);
        assert_eq!(z.previous(), 1.0, "the regular zoom is untouched");
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
}
