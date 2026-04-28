pub const GEMINI_3_PRO_ASPECT_RATIOS: [&str; 10] = [
    "1:1", "2:3", "3:2", "3:4", "4:3", "4:5", "5:4", "9:16", "16:9", "21:9",
];

pub const GEMINI_3_FLASH_ASPECT_RATIOS: [&str; 14] = [
    "1:1", "1:4", "1:8", "2:3", "3:2", "3:4", "4:1", "4:3", "4:5", "5:4", "8:1", "9:16", "16:9",
    "21:9",
];

pub const GEMINI_2_5_FLASH_ASPECT_RATIOS: [&str; 10] = [
    "1:1", "2:3", "3:2", "3:4", "4:3", "4:5", "5:4", "9:16", "16:9", "21:9",
];

pub const GEMINI_3_PRO_IMAGE_SIZES: [&str; 3] = ["1K", "2K", "4K"];
pub const GEMINI_3_FLASH_IMAGE_SIZES: [&str; 4] = ["512", "1K", "2K", "4K"];
pub const GEMINI_3_FLASH_THINKING_LEVELS: [&str; 2] = ["minimal", "high"];

pub fn max_reference_images(model: &str) -> usize {
    match model {
        "gemini-2.5-flash-image" => 3,
        "gemini-3.1-flash-image-preview" | "gemini-3-pro-image-preview" => 14,
        _ => 0,
    }
}
