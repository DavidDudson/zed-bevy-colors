#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rgba {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Rgba {
    pub const fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    pub const WHITE: Self = Self::new(1.0, 1.0, 1.0, 1.0);
    pub const BLACK: Self = Self::new(0.0, 0.0, 0.0, 1.0);
    pub const NONE: Self = Self::new(0.0, 0.0, 0.0, 0.0);

    pub fn from_u8(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self::new(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, a as f32 / 255.0)
    }

    pub fn from_linear(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self::new(linear_to_srgb(r), linear_to_srgb(g), linear_to_srgb(b), a)
    }

    pub fn clamped(self) -> Self {
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

pub fn hsv_to_rgb(h: f32, s: f32, v: f32, a: f32) -> Rgba {
    let h = ((h % 360.0) + 360.0) % 360.0;
    let s = s.clamp(0.0, 1.0);
    let v = v.clamp(0.0, 1.0);
    let c = v * s;
    let hp = h / 60.0;
    let x = c * (1.0 - (hp % 2.0 - 1.0).abs());
    let (r1, g1, b1) = match hp as u32 {
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

pub fn oklab_to_rgb(l: f32, a_chan: f32, b_chan: f32, alpha: f32) -> Rgba {
    let l_ = l + 0.396_337_78 * a_chan + 0.215_803_76 * b_chan;
    let m_ = l - 0.105_561_346 * a_chan - 0.063_854_17 * b_chan;
    let s_ = l - 0.089_484_18 * a_chan - 1.291_485_5 * b_chan;
    let l3 = l_ * l_ * l_;
    let m3 = m_ * m_ * m_;
    let s3 = s_ * s_ * s_;
    let lr = 4.076_741_7 * l3 - 3.307_711_6 * m3 + 0.230_969_94 * s3;
    let lg = -1.268_438 * l3 + 2.609_757_4 * m3 - 0.341_319_38 * s3;
    let lb = -0.0041960863 * l3 - 0.703_418_6 * m3 + 1.707_614_7 * s3;
    Rgba::from_linear(lr, lg, lb, alpha)
}

pub fn oklch_to_rgb(l: f32, c: f32, h: f32, a: f32) -> Rgba {
    let h_rad = h.to_radians();
    oklab_to_rgb(l, c * h_rad.cos(), c * h_rad.sin(), a)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    fn approx(a: f32, b: f32) {
        assert!((a - b).abs() < 0.01, "{} != {}", a, b);
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
        approx(c.r, 0xaa as f32 / 255.0);
        approx(c.g, 0xbb as f32 / 255.0);
        approx(c.b, 0xcc as f32 / 255.0);
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
}
