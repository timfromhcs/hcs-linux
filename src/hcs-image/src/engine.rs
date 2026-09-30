//! Wrapper spawning /usr/lib/hcs/sd-cpp backend with AVX2 threads.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImageMode {
    Txt2Img,
    Img2Img,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceRequest {
    pub prompt: String,
    pub negative_prompt: String,
    pub mode: ImageMode,
    pub width: u32,
    pub height: u32,
    pub steps: u32,
    pub cfg_scale: f32,
    pub seed: i64,
    pub input_image: Option<String>,
    pub output: String,
}

impl Default for InferenceRequest {
    fn default() -> Self {
        Self {
            prompt: String::new(),
            negative_prompt: "blurry, low-res, watermark".to_string(),
            mode: ImageMode::Txt2Img,
            width: 512,
            height: 512,
            steps: 6,
            cfg_scale: 1.8,
            seed: -1,
            input_image: None,
            output: "render.png".to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct EngineConfig {
    pub backend: String,
    pub model_gguf: String,
    pub threads: usize,
}

impl Default for EngineConfig {
    fn default() -> Self {
        let threads = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4);
        Self {
            backend: "/usr/lib/hcs/sd-cpp".to_string(),
            model_gguf: "/usr/share/hcs/models/sd15-lcm-q4_0.gguf".to_string(),
            threads,
        }
    }
}

impl EngineConfig {
    /// Build the backend argv: diffusion sampling with multi-threaded AVX2 (-t $(nproc)).
    /// LCM needs only 4-8 steps (vs 30-50 DDIM/Euler); CFG 1.5-2.0.
    pub fn build_argv(&self, req: &InferenceRequest) -> Vec<String> {
        let mut args = vec![
            "-M".to_string(),
            self.model_gguf.clone(),
            "-p".to_string(),
            req.prompt.clone(),
            "--negative-prompt".to_string(),
            req.negative_prompt.clone(),
            "-W".to_string(),
            req.width.to_string(),
            "-H".to_string(),
            req.height.to_string(),
            "--steps".to_string(),
            req.steps.clamp(1, 8).to_string(),
            "--cfg-scale".to_string(),
            req.cfg_scale.clamp(1.0, 2.5).to_string(),
            "-t".to_string(),
            self.threads.to_string(),
            "-o".to_string(),
            req.output.clone(),
        ];
        if req.seed >= 0 {
            args.push("-s".to_string());
            args.push(req.seed.to_string());
        }
        if req.mode == ImageMode::Img2Img {
            if let Some(input) = &req.input_image {
                args.push("-i".to_string());
                args.push(input.clone());
            }
        }
        args
    }

    pub fn validate(&self, req: &InferenceRequest) -> anyhow::Result<()> {
        if req.prompt.trim().is_empty() {
            anyhow::bail!("prompt must not be empty");
        }
        if !matches!(
            (req.width, req.height),
            (256, 256) | (512, 512) | (512, 768) | (768, 512)
        ) {
            anyhow::bail!(
                "unsupported resolution {}x{} (use 256x256, 512x512, 512x768)",
                req.width,
                req.height
            );
        }
        if !(1..=8).contains(&req.steps) {
            anyhow::bail!("LCM steps must be 1-8 (got {})", req.steps);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_argv_txt2img() {
        let cfg = EngineConfig {
            backend: "/usr/lib/hcs/sd-cpp".into(),
            model_gguf: "m.gguf".into(),
            threads: 4,
        };
        let req = InferenceRequest {
            prompt: "a sleek futuristic workstation".into(),
            ..Default::default()
        };
        let argv = cfg.build_argv(&req);
        assert!(argv.contains(&"-t".to_string()));
        assert!(argv.contains(&"4".to_string()));
        assert!(argv.contains(&"--steps".to_string()));
    }

    #[test]
    fn test_rejects_bad_resolution() {
        let cfg = EngineConfig::default();
        let req = InferenceRequest {
            prompt: "x".into(),
            width: 1024,
            height: 1024,
            ..Default::default()
        };
        assert!(cfg.validate(&req).is_err());
    }
}
