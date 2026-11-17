use crate::capturable::{Capturable, Recorder};
use captrs::{CaptureError, Capturer};
use std::boxed::Box;
use std::error::Error;
use winapi::shared::windef::RECT;

use super::Geometry;

/// A duplicated output that stops delivering frames (monitor asleep, mode change,
/// fullscreen exclusive application, driver hiccup) never recovers on its own:
/// dxgcap only re-acquires the duplication on DXGI_ERROR_ACCESS_LOST, so the
/// capture loop would keep timing out forever and the stream would stay frozen.
/// Rebuild the capturer after this many consecutive timeouts.
///
/// Captrs acquires frames with a 200ms timeout, so 250 timeouts is roughly 50s.
const MAX_CONSECUTIVE_TIMEOUTS: u32 = 250;

/// Bytes per pixel of a wide colour (HDR) desktop duplication frame.
///
/// A display that is in HDR/wide colour mode is composited in scRGB and DXGI
/// then hands out `DXGI_FORMAT_R16G16B16A16_FLOAT` surfaces instead of
/// `DXGI_FORMAT_B8G8R8A8_UNORM`, i.e. four 16 bit half floats (RGB + alpha) per
/// pixel. This happens right after the duplication is created, and DXGI can also
/// switch back to it later, so it has to be handled at run time rather than
/// assumed away.
const WIDE_COLOUR_BYTES_PER_PIXEL: usize = 8;

#[derive(Clone)]
pub struct CaptrsCapturable {
    id: u8,
    name: String,
    screen: RECT,
    virtual_screen: RECT,
}

impl CaptrsCapturable {
    pub fn new(id: u8, name: String, screen: RECT, virtual_screen: RECT) -> CaptrsCapturable {
        CaptrsCapturable {
            id,
            name,
            screen,
            virtual_screen,
        }
    }
}

impl Capturable for CaptrsCapturable {
    fn name(&self) -> String {
        format!("Desktop {} (captrs)", self.name).into()
    }
    fn before_input(&mut self) -> Result<(), Box<dyn Error>> {
        Ok(())
    }
    fn recorder(&self, _capture_cursor: bool) -> Result<Box<dyn Recorder>, Box<dyn Error>> {
        Ok(Box::new(CaptrsRecorder::new(self.id)?))
    }
    fn geometry(&self) -> Result<Geometry, Box<dyn Error>> {
        Ok(Geometry::VirtualScreen(
            self.screen.left - self.virtual_screen.left,
            self.screen.top - self.virtual_screen.top,
            (self.screen.right - self.screen.left) as u32,
            (self.screen.bottom - self.screen.top) as u32,
            self.screen.left,
            self.screen.top,
        ))
    }
}
#[derive(Debug)]
pub struct CaptrsError(String);

impl std::fmt::Display for CaptrsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self(s) = self;
        write!(f, "{}", s)
    }
}

impl Error for CaptrsError {}

/// Convert one IEEE 754 binary16 (half float) to a float.
///
/// scRGB frames from desktop duplication only ever contain ordinary finite
/// values, but denormals, infinities and NaNs are handled so that a corrupt
/// frame can never turn into a panic.
fn half_to_f32(h: u16) -> f32 {
    let sign = (h >> 15) & 1;
    let exponent = (h >> 10) & 0x1f;
    let mantissa = (h & 0x03ff) as f32;
    let value = match exponent {
        0 => mantissa * 2f32.powi(-24),
        0x1f => {
            if mantissa == 0.0 {
                f32::INFINITY
            } else {
                f32::NAN
            }
        }
        e => (1.0 + mantissa / 1024.0) * 2f32.powi(e as i32 - 15),
    };
    if sign == 1 {
        -value
    } else {
        value
    }
}

/// scRGB is linear, the video pipeline expects sRGB.
fn linear_to_srgb_u8(value: f32) -> u8 {
    if value.is_nan() {
        return 0;
    }
    let value = value.clamp(0.0, 1.0);
    let srgb = if value <= 0.003_130_8 {
        12.92 * value
    } else {
        1.055 * value.powf(1.0 / 2.4) - 0.055
    };
    (srgb * 255.0 + 0.5) as u8
}

/// Table of every half float mapped to its sRGB gamma encoded 8 bit value.
///
/// Converting per pixel would need a `powf` per channel and per frame, which is
/// far too slow at 1440p, so the (fixed, 64 KiB) table is built once instead.
fn wide_colour_to_srgb_lut() -> Vec<u8> {
    (0..=u16::MAX)
        .map(|h| linear_to_srgb_u8(half_to_f32(h)))
        .collect()
}

/// Convert one `R16G16B16A16_FLOAT` frame to a tightly packed BGRA8 frame.
fn convert_wide_colour_frame(
    source: &[u8],
    width: usize,
    height: usize,
    stride: usize,
    srgb_lut: &[u8],
    converted: &mut Vec<u8>,
) {
    converted.clear();
    converted.reserve(width * height * 4);
    for row in source.chunks_exact(stride).take(height) {
        for pixel in row[..width * WIDE_COLOUR_BYTES_PER_PIXEL]
            .chunks_exact(WIDE_COLOUR_BYTES_PER_PIXEL)
        {
            // R16G16B16A16_FLOAT is stored as R, G, B, A, the video pipeline
            // wants B, G, R, A.
            let r = u16::from_le_bytes([pixel[0], pixel[1]]) as usize;
            let g = u16::from_le_bytes([pixel[2], pixel[3]]) as usize;
            let b = u16::from_le_bytes([pixel[4], pixel[5]]) as usize;
            converted.push(srgb_lut[b]);
            converted.push(srgb_lut[g]);
            converted.push(srgb_lut[r]);
            converted.push(255);
        }
    }
}

pub struct CaptrsRecorder {
    id: u8,
    capturer: Capturer,
    consecutive_timeouts: u32,
    /// Scratch buffer for frames that need converting to BGRA8.
    converted: Vec<u8>,
    /// Half float -> sRGB lookup table, see [`wide_colour_to_srgb_lut`].
    srgb_lut: Vec<u8>,
}

impl CaptrsRecorder {
    pub fn new(id: u8) -> Result<CaptrsRecorder, Box<dyn Error>> {
        Ok(CaptrsRecorder {
            id,
            capturer: Capturer::new(id.into())?,
            consecutive_timeouts: 0,
            converted: Vec::new(),
            srgb_lut: wide_colour_to_srgb_lut(),
        })
    }
}

impl Recorder for CaptrsRecorder {
    fn capture(&mut self) -> Result<crate::video::PixelProvider, Box<dyn Error>> {
        match self.capturer.capture_store_frame() {
            Ok(()) => self.consecutive_timeouts = 0,
            Err(e) => {
                if matches!(e, CaptureError::Timeout) {
                    self.consecutive_timeouts += 1;
                    if self.consecutive_timeouts >= MAX_CONSECUTIVE_TIMEOUTS {
                        self.consecutive_timeouts = 0;
                        self.capturer = Capturer::new(self.id.into())?;
                    }
                }
                return Err(Box::new(CaptrsError(format!(
                    "Captrs failed to capture frame: {e:?}"
                ))));
            }
        }
        let (width, height) = self.capturer.geometry();
        let frame = self
            .capturer
            .get_stored_frame()
            .ok_or_else(|| Box::new(CaptrsError("Captrs has no stored frame".into())))?;
        // `frame` is a slice of BGRA pixels, so its length is a pixel count, not
        // a byte count: transmuting the slice would keep the pixel count as the
        // length of the byte slice and make every frame look four times smaller
        // than it is.
        let data: &[u8] = unsafe {
            std::slice::from_raw_parts(
                frame.as_ptr() as *const u8,
                frame.len() * std::mem::size_of::<captrs::Bgr8>(),
            )
        };

        // Captrs stores the frame as (row_pitch / 4) * scan_lines pixels, which is
        // only tightly packed when the mapped surface has no row padding. It also
        // hands out the buffer of the frame it actually got, which is not
        // guaranteed to be the size reported by `geometry()` (the first frame after
        // a duplication is set up can be larger). Use the buffer to work out the
        // real stride instead of assuming `width * 4`, otherwise the rows are
        // shifted against each other and the image is sheared/scaled.
        let (width, height) = (width as usize, height as usize);
        if height == 0 || data.len() % height != 0 {
            return Err(Box::new(CaptrsError(format!(
                "Captrs frame of {} bytes does not fit into {} rows",
                data.len(),
                height
            ))));
        }
        let stride = data.len() / height;

        // Wide colour (HDR) frames are scRGB half floats, not BGRA. Reading them
        // as BGRA uses only the low byte of each 16 bit channel and packs two
        // pixels into one row slot, which shows up on the client as a horizontally
        // zoomed image with a blue cast. Convert them instead.
        if stride == width * WIDE_COLOUR_BYTES_PER_PIXEL {
            let mut converted = std::mem::take(&mut self.converted);
            convert_wide_colour_frame(
                data,
                width,
                height,
                stride,
                &self.srgb_lut,
                &mut converted,
            );
            self.converted = converted;
            tracing::info!(
                "Captured wide colour (HDR) frame, converted {}x{} scRGB to sRGB",
                width,
                height
            );
            return Ok(crate::video::PixelProvider::BGR0(
                width,
                height,
                &self.converted,
            ));
        }
        if stride < width * 4 {
            return Err(Box::new(CaptrsError(format!(
                "Captrs frame stride {} is smaller than {} px * 4",
                stride, width
            ))));
        }
        if stride == width * 4 {
            Ok(crate::video::PixelProvider::BGR0(width, height, data))
        } else {
            Ok(crate::video::PixelProvider::BGR0S(
                width, height, stride, data,
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn half_floats_are_decoded() {
        assert_eq!(half_to_f32(0x0000), 0.0);
        assert_eq!(half_to_f32(0x3c00), 1.0);
        assert_eq!(half_to_f32(0xbc00), -1.0);
        assert_eq!(half_to_f32(0x3800), 0.5);
        assert_eq!(half_to_f32(0x3400), 0.25);
        // smallest subnormal half is 2^-24
        assert!((half_to_f32(0x0001) - 5.960_464_5e-8).abs() < 1e-12);
        assert!(half_to_f32(0x7c00).is_infinite());
        assert!(half_to_f32(0x7e00).is_nan());
    }

    #[test]
    fn linear_values_become_srgb() {
        assert_eq!(linear_to_srgb_u8(0.0), 0);
        assert_eq!(linear_to_srgb_u8(1.0), 255);
        // HDR highlights and negative values are clipped
        assert_eq!(linear_to_srgb_u8(2.0), 255);
        assert_eq!(linear_to_srgb_u8(-1.0), 0);
        // 18% linear grey is sRGB middle grey
        assert_eq!(linear_to_srgb_u8(0.18), 118);
        assert_eq!(linear_to_srgb_u8(0.25), 137);
        assert_eq!(linear_to_srgb_u8(0.5), 188);
    }

    fn half(value: f32) -> u16 {
        // only the exact values used by the tests below
        match value {
            0.0 => 0x0000,
            0.25 => 0x3400,
            0.5 => 0x3800,
            1.0 => 0x3c00,
            _ => panic!("no encoding for {value}"),
        }
    }

    #[test]
    fn wide_colour_frame_becomes_bgra() {
        let srgb_lut = wide_colour_to_srgb_lut();
        // (R, G, B, A) = (0.5, 0.25, 1.0, 1.0) and (0, 0, 0, 1)
        let mut source = Vec::new();
        for (r, g, b) in [(0.5f32, 0.25f32, 1.0f32), (0.0, 0.0, 0.0)] {
            source.extend_from_slice(&half(r).to_le_bytes());
            source.extend_from_slice(&half(g).to_le_bytes());
            source.extend_from_slice(&half(b).to_le_bytes());
            source.extend_from_slice(&half(1.0).to_le_bytes());
        }
        let mut converted = Vec::new();
        convert_wide_colour_frame(&source, 2, 1, 16, &srgb_lut, &mut converted);
        assert_eq!(converted, vec![255, 137, 188, 255, 0, 0, 0, 255]);
    }

    #[test]
    fn wide_colour_frame_respects_the_row_pitch() {
        // two pixels with four bytes of padding at the end of the row
        let srgb_lut = wide_colour_to_srgb_lut();
        let mut source = vec![0u8; 20];
        source[0..2].copy_from_slice(&half(1.0).to_le_bytes()); // R of pixel 0
        source[12..14].copy_from_slice(&half(1.0).to_le_bytes()); // B of pixel 1
        let mut converted = Vec::new();
        convert_wide_colour_frame(&source, 2, 1, 20, &srgb_lut, &mut converted);
        assert_eq!(converted, vec![0, 0, 255, 255, 255, 0, 0, 255]);
    }
}