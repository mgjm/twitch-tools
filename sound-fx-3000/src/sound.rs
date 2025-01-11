use std::{fs::File, io, path::Path, sync::Arc};

use anyhow::{Context, Result};
use symphonia::core::{
    audio::{AudioBufferRef, Signal, SignalSpec},
    codecs::DecoderOptions,
    errors::Error,
    formats::FormatOptions,
    io::MediaSourceStream,
    probe::{Hint, ProbeResult},
};

/// A decoded sound sample.
#[derive(Clone)]
pub struct Sound {
    frames: Arc<[[f32; 2]]>,
    spec: SignalSpec,
}

impl Sound {
    /// Open and decode a sound file (e.g. mp3)
    pub fn open(path: &Path) -> Result<Self> {
        let mut hint = Hint::new();

        if let Some(ext) = path.extension() {
            if let Some(ext) = ext.to_str() {
                hint.with_extension(ext);
            }
        }

        let source = Box::new(File::open(path).context("open audio file")?);
        let source = MediaSourceStream::new(source, Default::default());

        let format_options = FormatOptions {
            ..Default::default()
        };

        let ProbeResult {
            mut format,
            metadata: _,
        } = symphonia::default::get_probe()
            .format(&hint, source, &format_options, &Default::default())
            .context("probe audio file")?;

        // eprintln!("{:#?}", metadata.get());
        // eprintln!("{:#?}", format.metadata().current());

        let decoder_options = DecoderOptions { verify: true };

        anyhow::ensure!(
            format.tracks().len() == 1,
            "expected one track found {} tracks",
            format.tracks().len(),
        );
        let track = format.default_track().context("no default track")?;
        let track_id = track.id;

        let mut decoder = symphonia::default::get_codecs()
            .make(&track.codec_params, &decoder_options)
            .context("init codec")?;

        anyhow::ensure!(
            track.codec_params.start_ts == 0,
            "expected to start from beginning, start ts {}",
            track.codec_params.start_ts,
        );

        let mut spec = None;
        let mut buffer = Buffer::default();

        while let Some(packet) = format
            .next_packet()
            .map(Some)
            .or_else(|err| {
                if matches!(&err, Error::IoError(err) if err.kind() == io::ErrorKind::UnexpectedEof)
                {
                    Ok(None)
                } else {
                    Err(err)
                }
            })
            .context("next packet")?
        {
            if packet.track_id() != track_id {
                continue;
            }

            while !format.metadata().is_latest() {
                format.metadata().pop();

                // if let Some(metadata) = format.metadata().current() {
                //     eprintln!("{metadata:#?}")
                // }
            }

            let decoded = decoder.decode(&packet).context("decode packet")?;

            if spec.is_none() {
                spec = Some(*decoded.spec());
            }

            buffer.write(decoded)?;
        }

        Ok(Self {
            frames: buffer.buffer.into(),
            spec: spec.context("no spec found")?,
        })
    }

    pub fn set_volume(&mut self, volume: f32) {
        for frame in Arc::make_mut(&mut self.frames) {
            frame[0] *= volume;
            frame[1] *= volume;
        }
    }

    /// Return the first signal spec of the decoded sound packets
    pub fn spec(&self) -> SignalSpec {
        self.spec
    }

    /// Get a shared reference to the decoded sound frames
    pub fn frames(&self) -> Arc<[[f32; 2]]> {
        self.frames.clone()
    }
}

#[derive(Default)]
struct Buffer {
    buffer: Vec<[f32; 2]>,
}

impl Buffer {
    fn write(&mut self, decoded: AudioBufferRef) -> Result<()> {
        if decoded.frames() == 0 {
            return Ok(());
        }

        let decoded = match decoded {
            AudioBufferRef::U8(_) => todo!("handle U8 audio buffer"),
            AudioBufferRef::U16(_) => todo!("handle U16 audio buffer"),
            AudioBufferRef::U24(_) => todo!("handle U24 audio buffer"),
            AudioBufferRef::U32(_) => todo!("handle U32 audio buffer"),
            AudioBufferRef::S8(_) => todo!("handle S8 audio buffer"),
            AudioBufferRef::S16(_) => todo!("handle S16 audio buffer"),
            AudioBufferRef::S24(_) => todo!("handle S24 audio buffer"),
            AudioBufferRef::S32(_) => todo!("handle S32 audio buffer"),
            AudioBufferRef::F32(decoded) => decoded,
            AudioBufferRef::F64(_) => todo!("handle F64 audio buffer"),
        };

        anyhow::ensure!(
            decoded.spec().channels.count() == 2,
            "expected stereo sound, found {} channels",
            decoded.spec().channels.count(),
        );

        for (l, r) in std::iter::zip(decoded.chan(0), decoded.chan(1)) {
            self.buffer.push([*l, *r]);
        }

        Ok(())
    }
}
