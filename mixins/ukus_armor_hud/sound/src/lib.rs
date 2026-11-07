// uku's Armor HUD: the "armor_breaking" sound (category "player").
//
// A .hnmod can't add a sound event to Horizon's resources, so the original
// OGG is compiled in, decoded with symphonia (already in Horizon's lock) and
// played on its own cpal output stream at the PLAYERS volume the game passes.
mod ogg;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::OnceLock;
use std::time::Duration;

struct Pcm {
    /// Interleaved samples.
    samples: Vec<f32>,
    channels: usize,
    rate: u32,
}

fn decode() -> Option<Pcm> {
    use symphonia::core::audio::SampleBuffer;
    use symphonia::core::codecs::DecoderOptions;
    use symphonia::core::formats::FormatOptions;
    use symphonia::core::io::MediaSourceStream;
    use symphonia::core::meta::MetadataOptions;
    use symphonia::core::probe::Hint;

    let mss = MediaSourceStream::new(Box::new(std::io::Cursor::new(&ogg::OGG[..])), Default::default());
    let mut hint = Hint::new();
    hint.with_extension("ogg");
    let probed = symphonia::default::get_probe()
        .format(&hint, mss, &FormatOptions::default(), &MetadataOptions::default())
        .ok()?;
    let mut format = probed.format;
    let track = format.default_track()?;
    let id = track.id;
    let rate = track.codec_params.sample_rate.unwrap_or(44_100);
    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &DecoderOptions::default())
        .ok()?;
    let mut samples = Vec::new();
    let mut channels = 1;
    while let Ok(packet) = format.next_packet() {
        if packet.track_id() != id {
            continue;
        }
        let Ok(decoded) = decoder.decode(&packet) else { continue };
        let spec = *decoded.spec();
        channels = spec.channels.count().max(1);
        let mut buf = SampleBuffer::<f32>::new(decoded.capacity() as u64, spec);
        buf.copy_interleaved_ref(decoded);
        samples.extend_from_slice(buf.samples());
    }
    (!samples.is_empty()).then_some(Pcm { samples, channels, rate })
}

fn pcm() -> Option<&'static Pcm> {
    static PCM: OnceLock<Option<Pcm>> = OnceLock::new();
    PCM.get_or_init(decode).as_ref()
}

/// The sample of channel `ch` at output frame `frame` (linear resampling;
/// a mono sound goes to every output channel).
fn sample(p: &Pcm, frame: usize, ch: usize, out_rate: u32) -> Option<f32> {
    let pos = frame as f64 * p.rate as f64 / out_rate as f64;
    let i = pos.floor() as usize;
    let t = (pos - i as f64) as f32;
    let frames = p.samples.len() / p.channels;
    if i >= frames {
        return None;
    }
    let c = ch.min(p.channels - 1);
    let a = p.samples[i * p.channels + c];
    let b = if i + 1 < frames { p.samples[(i + 1) * p.channels + c] } else { a };
    Some(a + (b - a) * t)
}

fn run<T>(device: &cpal::Device, config: &cpal::StreamConfig, p: &'static Pcm, volume: f32) -> Option<()>
where
    T: cpal::SizedSample + cpal::FromSample<f32>,
{
    let channels = config.channels as usize;
    let rate = config.sample_rate.0;
    let mut frame = 0usize;
    let stream = device
        .build_output_stream(
            config,
            move |data: &mut [T], _: &cpal::OutputCallbackInfo| {
                for out in data.chunks_mut(channels) {
                    for (ch, s) in out.iter_mut().enumerate() {
                        let v = sample(p, frame, ch, rate).unwrap_or(0.0) * volume;
                        *s = T::from_sample(v.clamp(-1.0, 1.0));
                    }
                    frame += 1;
                }
            },
            |_| {},
            None,
        )
        .ok()?;
    stream.play().ok()?;
    let secs = (p.samples.len() / p.channels) as f64 / p.rate as f64;
    std::thread::sleep(Duration::from_secs_f64(secs + 0.15));
    drop(stream);
    Some(())
}

/// Plays the sound once, at `volume` (master times players), off the
/// game's thread. Does nothing at volume 0 or without an output device.
pub fn play(volume: f32) {
    if !(volume > 0.0) {
        return;
    }
    let _ = std::thread::Builder::new()
        .name("ukus_armor_hud_sound".into())
        .spawn(move || {
            let Some(p) = pcm() else { return };
            let host = cpal::default_host();
            let Some(device) = host.default_output_device() else { return };
            let Ok(supported) = device.default_output_config() else { return };
            let format = supported.sample_format();
            let config: cpal::StreamConfig = supported.into();
            let _ = match format {
                cpal::SampleFormat::F32 => run::<f32>(&device, &config, p, volume),
                cpal::SampleFormat::I16 => run::<i16>(&device, &config, p, volume),
                cpal::SampleFormat::U16 => run::<u16>(&device, &config, p, volume),
                cpal::SampleFormat::I32 => run::<i32>(&device, &config, p, volume),
                _ => None,
            };
        });
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_ogg_decodes() {
        let p = super::pcm().expect("decodes");
        assert!(p.rate > 0 && p.channels > 0);
        assert!(p.samples.len() / p.channels > 1000);
    }
}
