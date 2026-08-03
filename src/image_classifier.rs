use image::{AnimationDecoder};
use lazy_static::lazy_static;
use std::sync::Arc;
use tract_onnx::prelude::*;

const MODEL_PATH: &str = "models/model.onnx";
const INPUT_SIZE: usize = 299;
const NSFW_THRESHOLD: f32 = 0.5;

pub struct NsfwResult {
    pub is_nsfw: bool,
    pub dominant_class: String,
    pub nsfw_score: f32,
}

lazy_static! {
    pub static ref CLASSIFIER: Option<Arc<TypedRunnableModel>> = {
        load_classifier(MODEL_PATH).ok()
    };
}

fn load_classifier(path: &str) -> Result<Arc<TypedRunnableModel>, Box<dyn std::error::Error + Send + Sync>> {
    let model = tract_onnx::onnx()
        .model_for_path(path)?
        .into_optimized()?
        .into_runnable()?;
    Ok(model)
}

fn do_classify(model: &Arc<TypedRunnableModel>, img: &image::RgbImage) -> Result<Vec<(String, f32)>, Box<dyn std::error::Error + Send + Sync>> {
    let (in_w, in_h) = img.dimensions();
    let x_scale = (in_w as f32 - 1.0) / (INPUT_SIZE as f32 - 1.0);
    let y_scale = (in_h as f32 - 1.0) / (INPUT_SIZE as f32 - 1.0);
    let mut data = vec![0f32; INPUT_SIZE * INPUT_SIZE * 3];
    for y in 0..INPUT_SIZE {
        for x in 0..INPUT_SIZE {
            let gx = x as f32 * x_scale;
            let gy = y as f32 * y_scale;
            let x0 = gx.floor() as u32;
            let y0 = gy.floor() as u32;
            let x1 = (x0 + 1).min(in_w.saturating_sub(1));
            let y1 = (y0 + 1).min(in_h.saturating_sub(1));
            let fx = gx - x0 as f32;
            let fy = gy - y0 as f32;
            let p00 = img.get_pixel(x0, y0);
            let p10 = img.get_pixel(x1, y0);
            let p01 = img.get_pixel(x0, y1);
            let p11 = img.get_pixel(x1, y1);
            for c in 0..3 {
                let top = p00[c] as f32 * (1.0 - fx) + p10[c] as f32 * fx;
                let bot = p01[c] as f32 * (1.0 - fx) + p11[c] as f32 * fx;
                let v = (top * (1.0 - fy) + bot * fy) / 255.0;
                data[y * INPUT_SIZE + x + c * INPUT_SIZE * INPUT_SIZE] = v;
            }
        }
    }
    let arr = tract_ndarray::Array4::from_shape_vec((1, INPUT_SIZE, INPUT_SIZE, 3), data)?;
    let tensor: Tensor = arr.into();
    let result = model.run(tvec![tensor.into()])?;
    let out = result[0].to_plain_array_view::<f32>()?;
    let classes = ["drawings", "hentai", "neutral", "porn", "sexy"];
    let mut probs: Vec<(String, f32)> = classes.iter().enumerate()
        .map(|(i, c)| (c.to_string(), out[[0, i]]))
        .collect();
    probs.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
    Ok(probs)
}

pub fn classify_image(bytes: &[u8]) -> Result<NsfwResult, Box<dyn std::error::Error + Send + Sync>> {
    let img = image::load_from_memory(bytes)?;
    let classifier = CLASSIFIER.as_ref().ok_or("nsfw classifier not loaded")?;
    let rgb = img.to_rgb8();
    let probs = do_classify(classifier, &rgb)?;
    let nsfw_score = probs.iter()
        .filter(|(c, _)| c == "hentai" || c == "porn")
        .map(|(_, p)| p)
        .sum::<f32>();
    let dominant = probs.first().map(|(c, _)| c.clone()).unwrap_or_default();
    Ok(NsfwResult {
        is_nsfw: nsfw_score > NSFW_THRESHOLD,
        dominant_class: dominant,
        nsfw_score,
    })
}

pub fn classify_gif(bytes: &[u8]) -> Result<NsfwResult, Box<dyn std::error::Error + Send + Sync>> {
    use image::codecs::gif::GifDecoder;
    use std::io::Cursor;
    let cursor = Cursor::new(bytes);
    let decoder = GifDecoder::new(cursor)?;
    let frames = decoder.into_frames();
    let classifier = CLASSIFIER.as_ref().ok_or("nsfw classifier not loaded")?;

    let mut worst_score = 0f32;
    let mut worst_probs: Vec<(String, f32)> = vec![];
    let mut frames_classified = 0u32;

    for frame in frames {
        let frame = frame?;
        let rgb = image::DynamicImage::ImageRgba8(frame.into_buffer()).to_rgb8();
        let probs = do_classify(classifier, &rgb)?;
        let score = probs.iter()
            .filter(|(c, _)| c == "hentai" || c == "porn")
            .map(|(_, p)| p)
            .sum::<f32>();
        if score > worst_score {
            worst_score = score;
            worst_probs = probs;
        }
        frames_classified += 1;
        if frames_classified >= 10 {
            break;
        }
    }

    let dominant = worst_probs.first().map(|(c, _)| c.clone()).unwrap_or_default();
    Ok(NsfwResult {
        is_nsfw: worst_score > NSFW_THRESHOLD,
        dominant_class: dominant,
        nsfw_score: worst_score,
    })
}