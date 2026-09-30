//! Image Studio logic (P1). Kept in hcs-ui so both the GUI and the tests use
//! one source of truth, and so the .slint view stays logic-free.

use crate::{clamp_cfg, clamp_steps};
use hcs_image::{EngineConfig, ImageMode, InferenceRequest};

/// UI-facing state of the Image Studio tab.
#[derive(Debug, Clone, PartialEq)]
pub struct ImageStudioState {
    pub prompt: String,
    pub negative_prompt: String,
    pub steps: i32,
    pub cfg: f32,
    pub seed: i64,
    pub resolution: i32,
    pub img2img_path: Option<String>,
    pub output_path: String,
}

impl Default for ImageStudioState {
    /// Defaults are exactly the values the master plan §4.2 specifies:
    /// 512x512, 4-8 LCM steps, CFG 1.5-2.0, random seed.
    fn default() -> Self {
        Self {
            prompt: "a sleek futuristic cybernetic workstation in neon glass".to_string(),
            negative_prompt: "blurry, low-res, watermark".to_string(),
            steps: 6,
            cfg: 1.8,
            seed: -1,
            resolution: 512,
            img2img_path: None,
            output_path: "render.png".to_string(),
        }
    }
}

impl ImageStudioState {
    /// Map a UI resolution slider value to the engine's width/height.
    /// The slider offers 256 / 512 / 768; only the resolutions §4.2 allows
    /// (256x256, 512x512, 512x768) are accepted by the engine.
    pub fn dims(&self) -> (u32, u32) {
        match self.resolution {
            r if r <= 256 => (256, 256),
            r if r <= 512 => (512, 512),
            _ => (512, 768),
        }
    }

    /// Build the engine request, clamping to the documented LCM limits.
    pub fn to_request(&self) -> InferenceRequest {
        let (w, h) = self.dims();
        InferenceRequest {
            prompt: self.prompt.clone(),
            negative_prompt: self.negative_prompt.clone(),
            mode: if self.img2img_path.is_some() {
                ImageMode::Img2Img
            } else {
                ImageMode::Txt2Img
            },
            width: w,
            height: h,
            steps: clamp_steps(self.steps as f32) as u32,
            cfg_scale: clamp_cfg(self.cfg),
            seed: self.seed,
            input_image: self.img2img_path.clone(),
            output: self.output_path.clone(),
        }
    }

    /// Validate the request against the engine, returning a human message.
    pub fn validate(&self) -> Result<(), String> {
        EngineConfig::default()
            .validate(&self.to_request())
            .map_err(|e| e.to_string())
    }
}

/// Build a request directly from raw UI values (used by the window binding and
/// by tests, so both paths cannot drift apart).
#[allow(clippy::too_many_arguments)]
pub fn render_request_from_ui(
    prompt: &str,
    negative_prompt: &str,
    steps: u32,
    cfg: f32,
    seed: i64,
    img2img: Option<String>,
    output: String,
) -> InferenceRequest {
    let mut req = InferenceRequest {
        prompt: prompt.to_string(),
        negative_prompt: negative_prompt.to_string(),
        mode: if img2img.is_some() {
            ImageMode::Img2Img
        } else {
            ImageMode::Txt2Img
        },
        width: 512,
        height: 512,
        steps: clamp_steps(steps as f32) as u32,
        cfg_scale: clamp_cfg(cfg),
        seed,
        input_image: img2img,
        output,
    };
    if req.prompt.trim().is_empty() {
        req.prompt = "a sleek futuristic cybernetic workstation in neon glass".to_string();
    }
    req
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_plan_compliant() {
        let s = ImageStudioState::default();
        assert!(s.steps >= 1 && s.steps <= 8, "LCM needs 1-8 steps");
        assert!((1.5..=2.0).contains(&s.cfg), "plan §4.2 CFG 1.5-2.0");
        assert_eq!(s.dims(), (512, 512));
        assert!(s.validate().is_ok());
    }

    #[test]
    fn resolution_slider_maps_to_allowed_dims() {
        for (slider, expect) in [
            (256, (256u32, 256u32)),
            (512, (512, 512)),
            (768, (512, 768)),
        ] {
            let s = ImageStudioState {
                resolution: slider,
                ..Default::default()
            };
            assert_eq!(s.dims(), expect, "slider {slider}");
        }
    }

    #[test]
    fn out_of_range_ui_values_are_clamped() {
        let s = ImageStudioState {
            steps: 99,
            cfg: 12.0,
            ..Default::default()
        };
        let r = s.to_request();
        assert_eq!(r.steps, 8, "steps clamp to LCM max");
        assert_eq!(r.cfg_scale, 2.5, "cfg clamps to max");
    }

    #[test]
    fn img2img_switches_mode() {
        let s = ImageStudioState {
            img2img_path: Some("/tmp/in.png".into()),
            ..Default::default()
        };
        let r = s.to_request();
        assert_eq!(r.mode, ImageMode::Img2Img);
        assert_eq!(r.input_image.as_deref(), Some("/tmp/in.png"));
    }

    #[test]
    fn empty_prompt_is_rejected() {
        let s = ImageStudioState {
            prompt: "   ".into(),
            ..Default::default()
        };
        assert!(
            s.validate().is_err(),
            "empty prompt must not reach the engine"
        );
    }
}
