use image::DynamicImage;
use rusty_tesseract::{Args, Image};
use std::collections::HashMap;

pub fn extract_text_from_image(img: &DynamicImage) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let tess_image = Image::from_dynamic_image(img)?;
    
    let mut config_variables = HashMap::new();
    config_variables.insert("--psm".to_string(), "6".to_string());
    config_variables.insert("--oem".to_string(), "3".to_string());
    
    let args = Args {
        lang: "eng".to_string(),
        config_variables,
        dpi: Some(300),
        psm: Some(6),
        oem: Some(3),
    };
    
    let output = rusty_tesseract::image_to_string(&tess_image, &args)?;
    Ok(output.trim().to_string())
}

pub fn extract_text_from_gif(bytes: &[u8]) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    use image::codecs::gif::GifDecoder;
    use image::AnimationDecoder;
    use std::io::Cursor;
    
    let cursor = Cursor::new(bytes);
    let decoder = GifDecoder::new(cursor)?;
    let frames = decoder.into_frames();
    
    let mut all_text = String::new();
    let mut frames_processed = 0u32;
    
    for frame in frames {
        let frame = frame?;
        let img = DynamicImage::ImageRgba8(frame.into_buffer());
        
        if let Ok(text) = extract_text_from_image(&img) {
            if !text.is_empty() {
                all_text.push_str(&text);
                all_text.push(' ');
            }
        }
        
        frames_processed += 1;
        if frames_processed >= 10 {
            break;
        }
    }
    
    Ok(all_text.trim().to_string())
}
