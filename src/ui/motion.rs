//! Short native motions for deliberate workspace changes (ui-design §6).
use nsh_core::element::{LayoutAnimation, LayoutCurve, LayoutTransition};
use nsh_core::prelude::*;

use super::theme::MOTION_FAST_MS;

pub const BASE_MS: f32 = 240.0;
const SLOW_MS: f32 = 320.0;
const DIALOG_RISE: f32 = 10.0;
const DIALOG_ENTER_SCALE: f32 = 0.97;
const TOOLBAR_ENTER_DISTANCE: f32 = -4.0;

pub fn reveal() -> Motion {
    motion().entering(
        MotionSpec::new(KeyframeTiming::new(MOTION_FAST_MS).easing(Easing::EaseOut))
            .values(MotionChannel::Opacity, [0.72, 1.0]),
    )
}

pub fn toolbar_enter() -> Motion {
    motion().entering(
        MotionSpec::new(KeyframeTiming::new(BASE_MS).easing(Easing::EaseOut))
            .values(MotionChannel::Opacity, [0.55, 1.0])
            .values(MotionChannel::TranslateY, [TOOLBAR_ENTER_DISTANCE, 0.0]),
    )
}

pub fn dialog_enter() -> Motion {
    motion().entering(
        MotionSpec::new(KeyframeTiming::new(SLOW_MS).easing(Easing::EaseOut))
            .values(MotionChannel::Opacity, [0.0, 1.0])
            .values(MotionChannel::TranslateY, [DIALOG_RISE, 0.0])
            .values(MotionChannel::Scale, [DIALOG_ENTER_SCALE, 1.0]),
    )
}

pub fn tab_change() {
    if !reduce_motion() {
        animate_layout(LayoutAnimation::all(LayoutTransition::new(
            LayoutCurve::EaseInEaseOut,
            MOTION_FAST_MS,
        )));
    }
}

pub fn panel_change() {
    if !reduce_motion() {
        animate_layout(LayoutAnimation {
            update: Some(LayoutTransition::new(LayoutCurve::EaseInEaseOut, BASE_MS)),
            ..Default::default()
        });
    }
}
