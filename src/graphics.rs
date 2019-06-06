use quicksilver::graphics::Color;
use std::time::Duration;

fn color_to_u8(c: &Color) -> (u8, u8, u8) {
    let convert = |f: f32| (f * u8::max_value() as f32) as u8;
    (convert(c.r), convert(c.g), convert(c.b))
}

pub trait Strobe {
    fn strobe(&self, time: &Duration, rate: Duration) -> Color;
}

impl Strobe for Color {
    fn strobe(&self, time: &Duration, rate: Duration) -> Color {
        let color8 = color_to_u8(self);
        let period = time.as_millis() as f32 / rate.as_millis() as f32;
        let blend = |cdiff| (cdiff as f32 * ((period as f32 * std::f32::consts::PI * 2.).cos() * 0.5 + 0.5)) as u8;
        Color::from_rgba(
            color8.0 + blend(u8::max_value() - color8.0),
            color8.1 + blend(u8::max_value() - color8.1),
            color8.2 + blend(u8::max_value() - color8.2),
            self.a
        )
    }
}
