/// The text of the first QR code found in a greyscale image.
fn decode(width: usize, height: usize, luma: impl FnMut(usize, usize) -> u8) -> Result<String, String> {
    let mut prepared = rqrr::PreparedImage::prepare_from_greyscale(width, height, luma);
    prepared
        .detect_grids()
        .iter()
        .find_map(|grid| grid.decode().ok().map(|(_, text)| text))
        .ok_or_else(|| "No QR code found".to_string())
}

pub fn decode_file(bytes: &[u8]) -> Result<String, String> {
    let image = image::load_from_memory(bytes).map_err(|e| e.to_string())?.to_luma8();
    decode(image.width() as usize, image.height() as usize, |x, y| image.get_pixel(x as u32, y as u32).0[0])
}

pub fn decode_clipboard() -> Result<String, String> {
    let mut clipboard = arboard::Clipboard::new().map_err(|e| e.to_string())?;
    let image = clipboard.get_image().map_err(|_| "No image on the clipboard".to_string())?;
    let rgba = image.bytes;
    decode(image.width, image.height, |x, y| {
        let i = (y * image.width + x) * 4;
        let (r, g, b) = (rgba[i] as u32, rgba[i + 1] as u32, rgba[i + 2] as u32);
        ((r * 299 + g * 587 + b * 114) / 1000) as u8
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_an_otpauth_qr_code() {
        let uri = "otpauth://totp/ACME:alice?secret=MZXW6&issuer=ACME";
        let code = qrcode::QrCode::new(uri).unwrap();
        let (modules, scale, quiet) = (code.width(), 8, 4);
        let colors = code.to_colors();
        let size = (modules + quiet * 2) * scale;
        let dark = |x: usize, y: usize| {
            let (mx, my) = ((x / scale).checked_sub(quiet), (y / scale).checked_sub(quiet));
            matches!((mx, my), (Some(mx), Some(my)) if mx < modules && my < modules && colors[my * modules + mx] == qrcode::Color::Dark)
        };
        assert_eq!(decode(size, size, |x, y| if dark(x, y) { 0 } else { 255 }).unwrap(), uri);
        assert!(decode(64, 64, |_, _| 255).is_err());
    }
}
