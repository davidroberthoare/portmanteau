//! Pictures placed in text, shared by every writer.
//!
//! The editor stores a picture as an `ImageObject` whose `asset_id` is a path
//! below the project's `assets/` folder. A writer asks here for the file's
//! bytes and its size on the page. Only PNG and JPEG are drawn; anything else
//! (or a file that is missing, too large, outside `assets/`, or not what its
//! name says) stays the `[Image]` placeholder.

use std::sync::Arc;

use super::*;

const MAX_PICTURE_BYTES: u64 = 30 * 1024 * 1024;

#[derive(Debug)]
pub struct LoadedPicture {
    pub bytes: Vec<u8>,
    /// `png` or `jpg`.
    pub ext: &'static str,
    pub media: &'static str,
    /// Pixel size.
    pub px: (u32, u32),
}

/// Read a picture from the project's `assets/` folder.
pub(crate) fn load_picture(asset_root: Option<&Path>, asset_id: &str) -> Option<Arc<LoadedPicture>> {
    let root = asset_root?;
    let relative = Path::new(asset_id);
    if relative.is_absolute()
        || relative.components().any(|c| !matches!(c, std::path::Component::Normal(_)))
    {
        return None;
    }
    let assets = fs::canonicalize(root.join("assets")).ok()?;
    let real = fs::canonicalize(assets.join(relative)).ok()?;
    if !real.starts_with(&assets) || fs::metadata(&real).ok()?.len() > MAX_PICTURE_BYTES {
        return None;
    }
    let bytes = fs::read(&real).ok()?;
    let (ext, media, px) = if bytes.starts_with(&[0x89, b'P', b'N', b'G']) {
        ("png", "image/png", png_size(&bytes)?)
    } else if bytes.starts_with(&[0xFF, 0xD8]) {
        ("jpg", "image/jpeg", jpeg_size(&bytes)?)
    } else {
        return None;
    };
    if px.0 == 0 || px.1 == 0 {
        return None;
    }
    Some(Arc::new(LoadedPicture { bytes, ext, media, px }))
}

fn png_size(bytes: &[u8]) -> Option<(u32, u32)> {
    let w = u32::from_be_bytes(bytes.get(16..20)?.try_into().ok()?);
    let h = u32::from_be_bytes(bytes.get(20..24)?.try_into().ok()?);
    Some((w, h))
}

fn jpeg_size(bytes: &[u8]) -> Option<(u32, u32)> {
    let mut i = 2;
    while i + 9 < bytes.len() {
        if bytes[i] != 0xFF {
            i += 1;
            continue;
        }
        let marker = bytes[i + 1];
        if marker == 0xFF {
            i += 1;
            continue;
        }
        if (0xC0..=0xCF).contains(&marker) && !matches!(marker, 0xC4 | 0xC8 | 0xCC) {
            let h = u16::from_be_bytes([bytes[i + 5], bytes[i + 6]]) as u32;
            let w = u16::from_be_bytes([bytes[i + 7], bytes[i + 8]]) as u32;
            return Some((w, h));
        }
        let length = u16::from_be_bytes([bytes[i + 2], bytes[i + 3]]) as usize;
        i += 2 + length.max(2);
    }
    None
}

/// The size, in points, a picture takes on the page: the size the writer asked
/// for (either side alone keeps the proportions), else its natural size at
/// 96 pixels per inch; always scaled down to fit `max_width` x `max_height`.
pub(crate) fn picture_size_pt(
    image: &ImageObject,
    picture: &LoadedPicture,
    max_width: f32,
    max_height: f32,
) -> (f32, f32) {
    let aspect = picture.px.1 as f32 / picture.px.0 as f32;
    let (mut w, mut h) = match (image.width, image.height) {
        (Some(w), Some(h)) if w > 0.0 && h > 0.0 => (w, h),
        (Some(w), _) if w > 0.0 => (w, w * aspect),
        (_, Some(h)) if h > 0.0 => (h / aspect, h),
        _ => (picture.px.0 as f32 * 0.75, picture.px.1 as f32 * 0.75),
    };
    let fit = (max_width / w).min(max_height / h).min(1.0);
    if fit < 1.0 {
        w *= fit;
        h *= fit;
    }
    (w.max(1.0), h.max(1.0))
}

impl ProseParagraph {
    /// The picture, when this paragraph holds nothing else.
    pub(crate) fn standalone_image(&self) -> Option<&ImageObject> {
        let mut found = None;
        for item in &self.content {
            match item {
                InlineContent::Image(image) if found.is_none() => found = Some(image),
                InlineContent::Image(_) => return None,
                InlineContent::Text(run) if run.text.trim().is_empty() => {}
                InlineContent::HardBreak => {}
                InlineContent::Text(_) => return None,
            }
        }
        found
    }
}
