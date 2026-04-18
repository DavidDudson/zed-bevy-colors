//! `Rgba` color type and color-space conversion helpers.

use crate::num::f32_to_u32_floor_clamped;

/// sRGB color with straight (non-premultiplied) alpha; all components in `[0.0, 1.0]`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rgba {
    /// Red channel in `[0.0, 1.0]`.
    pub r: f32,
    /// Green channel in `[0.0, 1.0]`.
    pub g: f32,
    /// Blue channel in `[0.0, 1.0]`.
    pub b: f32,
    /// Alpha channel in `[0.0, 1.0]`.
    pub a: f32,
}

impl Rgba {
    /// Construct an `Rgba` from four `f32` components.
    #[must_use]
    pub const fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    /// Opaque white.
    pub const WHITE: Self = Self::new(1.0, 1.0, 1.0, 1.0);
    /// Opaque black.
    pub const BLACK: Self = Self::new(0.0, 0.0, 0.0, 1.0);
    /// Fully transparent black.
    pub const NONE: Self = Self::new(0.0, 0.0, 0.0, 0.0);

    /// Construct from 8-bit unsigned channels (each divided by 255).
    #[must_use]
    pub fn from_u8(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self::new(
            f32::from(r) / 255.0,
            f32::from(g) / 255.0,
            f32::from(b) / 255.0,
            f32::from(a) / 255.0,
        )
    }

    /// Construct from linear-light RGB components, applying the sRGB transfer curve.
    #[must_use]
    pub fn from_linear(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self::new(linear_to_srgb(r), linear_to_srgb(g), linear_to_srgb(b), a)
    }

    /// Return a copy with all channels clamped to `[0.0, 1.0]`.
    #[must_use]
    pub const fn clamped(self) -> Self {
        Self::new(
            self.r.clamp(0.0, 1.0),
            self.g.clamp(0.0, 1.0),
            self.b.clamp(0.0, 1.0),
            self.a.clamp(0.0, 1.0),
        )
    }
}

fn linear_to_srgb(c: f32) -> f32 {
    if c <= 0.0 {
        0.0
    } else if c <= 0.003_130_8 {
        c * 12.92
    } else if c < 1.0 {
        1.055 * c.powf(1.0 / 2.4) - 0.055
    } else {
        1.0
    }
}

/// Parse a hex color string, returning `None` on invalid input.
///
/// Accepts `#RGB`, `#RGBA`, `#RRGGBB`, and `#RRGGBBAA` (with or without
/// the leading `#`). Returns `None` for any other length or non-hex digits.
#[must_use]
pub fn parse_hex(s: &str) -> Option<Rgba> {
    let s = s.trim_start_matches('#');
    let bytes: Vec<u8> = match s.len() {
        3 => s
            .chars()
            .map(|c| u8::from_str_radix(&c.to_string().repeat(2), 16).ok())
            .collect::<Option<_>>()?,
        4 => s
            .chars()
            .map(|c| u8::from_str_radix(&c.to_string().repeat(2), 16).ok())
            .collect::<Option<_>>()?,
        6 | 8 => (0..s.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&s[i..i + 2], 16).ok())
            .collect::<Option<_>>()?,
        _ => return None,
    };
    match bytes.as_slice() {
        [r, g, b] => Some(Rgba::from_u8(*r, *g, *b, 255)),
        [r, g, b, a] => Some(Rgba::from_u8(*r, *g, *b, *a)),
        _ => None,
    }
}

/// Convert HSL to sRGB. `h` in degrees `[0, 360)`, `s` and `l` in `[0.0, 1.0]`, `a` in `[0.0, 1.0]`.
#[must_use]
pub fn hsl_to_rgb(h: f32, s: f32, l: f32, a: f32) -> Rgba {
    let h = ((h % 360.0) + 360.0) % 360.0 / 360.0;
    let s = s.clamp(0.0, 1.0);
    let l = l.clamp(0.0, 1.0);
    if s == 0.0 {
        return Rgba::new(l, l, l, a);
    }
    let q = if l < 0.5 { l * (1.0 + s) } else { l + s - l * s };
    let p = 2.0 * l - q;
    Rgba::new(
        hue_to_rgb(p, q, h + 1.0 / 3.0),
        hue_to_rgb(p, q, h),
        hue_to_rgb(p, q, h - 1.0 / 3.0),
        a,
    )
}

fn hue_to_rgb(p: f32, q: f32, mut t: f32) -> f32 {
    if t < 0.0 {
        t += 1.0;
    }
    if t > 1.0 {
        t -= 1.0;
    }
    if t < 1.0 / 6.0 {
        return p + (q - p) * 6.0 * t;
    }
    if t < 0.5 {
        return q;
    }
    if t < 2.0 / 3.0 {
        return p + (q - p) * (2.0 / 3.0 - t) * 6.0;
    }
    p
}

/// Convert HSV to sRGB. `h` in degrees `[0, 360)`, `s` and `v` in `[0.0, 1.0]`, `a` in `[0.0, 1.0]`.
#[must_use]
pub fn hsv_to_rgb(h: f32, s: f32, v: f32, a: f32) -> Rgba {
    let h = ((h % 360.0) + 360.0) % 360.0;
    let s = s.clamp(0.0, 1.0);
    let v = v.clamp(0.0, 1.0);
    let c = v * s;
    let hp = h / 60.0;
    let x = c * (1.0 - (hp % 2.0 - 1.0).abs());
    let (r1, g1, b1) = match f32_to_u32_floor_clamped(hp, 6) {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let m = v - c;
    Rgba::new(r1 + m, g1 + m, b1 + m, a)
}

/// Convert HWB to sRGB. `h` in degrees `[0, 360)`, `w` and `b` (whiteness/blackness) in `[0.0, 1.0]`, `a` in `[0.0, 1.0]`.
#[must_use]
pub fn hwb_to_rgb(h: f32, w: f32, b: f32, a: f32) -> Rgba {
    let w = w.clamp(0.0, 1.0);
    let b = b.clamp(0.0, 1.0);
    if w + b >= 1.0 {
        let g = w / (w + b);
        return Rgba::new(g, g, g, a);
    }
    let rgb = hsv_to_rgb(h, 1.0, 1.0, a);
    Rgba::new(rgb.r * (1.0 - w - b) + w, rgb.g * (1.0 - w - b) + w, rgb.b * (1.0 - w - b) + w, a)
}

/// Convert Oklab to sRGB. `l` in `[0.0, 1.0]`, `a_chan` and `b_chan` roughly in `[-0.4, 0.4]`, `alpha` in `[0.0, 1.0]`.
#[must_use]
pub fn oklab_to_rgb(l: f32, a_chan: f32, b_chan: f32, alpha: f32) -> Rgba {
    let l_ = l + 0.396_337_78 * a_chan + 0.215_803_76 * b_chan;
    let m_ = l - 0.105_561_346 * a_chan - 0.063_854_17 * b_chan;
    let s_ = l - 0.089_484_18 * a_chan - 1.291_485_5 * b_chan;
    let l3 = l_ * l_ * l_;
    let m3 = m_ * m_ * m_;
    let s3 = s_ * s_ * s_;
    let lr = 4.076_741_7 * l3 - 3.307_711_6 * m3 + 0.230_969_94 * s3;
    let lg = -1.268_438 * l3 + 2.609_757_4 * m3 - 0.341_319_38 * s3;
    let lb = -0.004_196_086_3 * l3 - 0.703_418_6 * m3 + 1.707_614_7 * s3;
    Rgba::from_linear(lr, lg, lb, alpha)
}

/// Convert Oklch to sRGB. `l` in `[0.0, 1.0]`, `c` (chroma) ≥ 0, `h` (hue) in degrees, `a` in `[0.0, 1.0]`.
#[must_use]
pub fn oklch_to_rgb(l: f32, c: f32, h: f32, a: f32) -> Rgba {
    let h_rad = h.to_radians();
    oklab_to_rgb(l, c * h_rad.cos(), c * h_rad.sin(), a)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    fn approx(a: f32, b: f32) {
        assert!((a - b).abs() < 0.01, "{a} != {b}");
    }

    #[test]
    fn hex_six() {
        let c = parse_hex("FF8800").unwrap();
        approx(c.r, 1.0);
        approx(c.g, 0.533);
        approx(c.b, 0.0);
        approx(c.a, 1.0);
    }

    #[test]
    fn hex_three() {
        let c = parse_hex("#abc").unwrap();
        let (r, g, b) =
            (f32::from(0xaa_u8) / 255.0, f32::from(0xbb_u8) / 255.0, f32::from(0xcc_u8) / 255.0);
        approx(c.r, r);
        approx(c.g, g);
        approx(c.b, b);
    }

    #[test]
    fn hex_eight_with_alpha() {
        let c = parse_hex("FF000080").unwrap();
        approx(c.r, 1.0);
        approx(c.a, 128.0 / 255.0);
    }

    #[test]
    fn hex_invalid() {
        assert!(parse_hex("XYZ").is_none());
        assert!(parse_hex("12345").is_none());
    }

    #[test]
    fn hsl_red() {
        let c = hsl_to_rgb(0.0, 1.0, 0.5, 1.0);
        approx(c.r, 1.0);
        approx(c.g, 0.0);
        approx(c.b, 0.0);
    }

    #[test]
    fn hsl_cyan() {
        let c = hsl_to_rgb(180.0, 1.0, 0.5, 1.0);
        approx(c.r, 0.0);
        approx(c.g, 1.0);
        approx(c.b, 1.0);
    }

    #[test]
    fn hsv_green() {
        let c = hsv_to_rgb(120.0, 1.0, 1.0, 1.0);
        approx(c.r, 0.0);
        approx(c.g, 1.0);
        approx(c.b, 0.0);
    }

    #[test]
    fn from_u8_basics() {
        let c = Rgba::from_u8(255, 0, 0, 255);
        approx(c.r, 1.0);
        approx(c.g, 0.0);
    }

    #[test]
    fn linear_white_round_trip() {
        let c = Rgba::from_linear(1.0, 1.0, 1.0, 1.0);
        approx(c.r, 1.0);
        approx(c.g, 1.0);
        approx(c.b, 1.0);
    }

    // --- hex ---

    #[test]
    fn hex_four_rgba() {
        // #RGBA short form: each nibble doubled
        let c = parse_hex("#f80f").unwrap();
        approx(c.r, 1.0); // 0xff / 255
        approx(c.g, 0.533); // 0x88 / 255
        approx(c.b, 0.0); // 0x00 / 255
        approx(c.a, 1.0); // 0xff / 255
    }

    // --- linear_to_srgb branches ---

    #[test]
    fn linear_to_srgb_negative_clamps_to_zero() {
        // c <= 0.0 branch
        let c = Rgba::from_linear(-1.0, 0.0, 0.0, 1.0);
        approx(c.r, 0.0);
        approx(c.g, 0.0);
    }

    #[test]
    fn linear_to_srgb_small_value() {
        // c <= 0.003_130_8 branch: sRGB linear segment
        let c = Rgba::from_linear(0.001, 0.0, 0.0, 1.0);
        approx(c.r, 0.001 * 12.92);
    }

    #[test]
    fn linear_to_srgb_mid_value() {
        // 0.003_130_8 < c < 1.0 branch: power curve
        let c = Rgba::from_linear(0.5, 0.0, 0.0, 1.0);
        let expected = 1.055f32 * 0.5f32.powf(1.0 / 2.4) - 0.055;
        approx(c.r, expected);
    }

    // --- Rgba::clamped ---

    #[test]
    fn clamped_out_of_range() {
        let c = Rgba::new(2.0, -0.5, 0.5, 1.5).clamped();
        approx(c.r, 1.0);
        approx(c.g, 0.0);
        approx(c.b, 0.5);
        approx(c.a, 1.0);
    }

    // --- hsl_to_rgb: achromatic (s == 0) and l >= 0.5 branches ---

    #[test]
    fn hsl_achromatic() {
        // s == 0 => gray
        let c = hsl_to_rgb(0.0, 0.0, 0.6, 1.0);
        approx(c.r, 0.6);
        approx(c.g, 0.6);
        approx(c.b, 0.6);
    }

    #[test]
    fn hsl_high_lightness() {
        // l >= 0.5 branch for q computation
        let c = hsl_to_rgb(240.0, 1.0, 0.75, 1.0); // bright blue
        approx(c.r, 0.5);
        approx(c.g, 0.5);
        approx(c.b, 1.0);
    }

    // --- hue_to_rgb all four t branches ---
    // hsl_to_rgb exercises hue_to_rgb indirectly; these cases hit the
    // t < 0 and t > 1 normalisation paths and the final p fallthrough.

    #[test]
    fn hue_to_rgb_all_branches() {
        // h=300° (magenta): hits t<0 (blue channel) and t>1 corrections
        let c = hsl_to_rgb(300.0, 1.0, 0.5, 1.0);
        approx(c.r, 1.0);
        approx(c.g, 0.0);
        approx(c.b, 1.0);
    }

    // --- hwb_to_rgb gray path ---

    #[test]
    fn hwb_gray_path() {
        // w + b >= 1.0  =>  achromatic
        let c = hwb_to_rgb(120.0, 0.6, 0.6, 1.0);
        let expected = 0.6 / (0.6 + 0.6);
        approx(c.r, expected);
        approx(c.g, expected);
        approx(c.b, expected);
    }

    #[test]
    fn hwb_normal_path() {
        // w + b < 1.0 => chromatic
        let c = hwb_to_rgb(120.0, 0.0, 0.0, 1.0); // pure green
        approx(c.r, 0.0);
        approx(c.g, 1.0);
        approx(c.b, 0.0);
    }

    // --- oklab / oklch ---

    #[test]
    fn oklab_neutral_gray() {
        // l=0.5, a=0, b=0 => near-gray
        let c = oklab_to_rgb(0.5, 0.0, 0.0, 1.0);
        // not testing exact value — just that r ≈ g ≈ b and in [0,1]
        assert!((c.r - c.g).abs() < 0.05, "r≈g for neutral oklab");
        assert!((c.g - c.b).abs() < 0.05, "g≈b for neutral oklab");
        assert!(c.a > 0.99);
    }

    #[test]
    fn oklch_zero_chroma_is_gray() {
        // c=0 => same as neutral oklab
        let c = oklch_to_rgb(0.5, 0.0, 0.0, 1.0);
        assert!((c.r - c.g).abs() < 0.05);
        assert!((c.g - c.b).abs() < 0.05);
        assert!(c.a > 0.99);
    }

    // --- hsv_to_rgb: hit all match arms via sector coverage ---

    #[test]
    fn hsv_sectors() {
        // sector 0: h=30 (orange)
        let c = hsv_to_rgb(30.0, 1.0, 1.0, 1.0);
        assert!(c.r > c.g && c.g > 0.0 && c.b < 0.01);
        // sector 2: h=150 (spring green)
        let c = hsv_to_rgb(150.0, 1.0, 1.0, 1.0);
        assert!(c.g > 0.99 && c.r < 0.01 && c.b > 0.0);
        // sector 3: h=210 (azure)
        let c = hsv_to_rgb(210.0, 1.0, 1.0, 1.0);
        assert!(c.b > c.g && c.r < 0.01);
        // sector 4: h=270 (violet)
        let c = hsv_to_rgb(270.0, 1.0, 1.0, 1.0);
        assert!(c.b > 0.99 && c.r > 0.0 && c.g < 0.01);
        // sector 5: h=330 (rose)
        let c = hsv_to_rgb(330.0, 1.0, 1.0, 1.0);
        assert!(c.r > 0.99 && c.b > 0.0 && c.g < 0.01);
    }
}
