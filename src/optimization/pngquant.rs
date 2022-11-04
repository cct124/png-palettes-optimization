use imagequant::Histogram;
use png::{ColorType, Decoder, Reader};
use std::{fs::File, path::Path};

use super::Frame;
use crate::error::Error;

pub struct Pngquant<'a> {
    path: &'a Path,
    reader: Reader<File>,
    bytes: Option<Vec<imagequant::RGBA>>,
    frames: Option<Vec<Frame>>,
    histogram: Option<Histogram>,
}

impl<'a> Pngquant<'a> {
    pub fn new(path: &'a Path) -> Result<Pngquant, Error> {
        let decoder = Decoder::new(File::open(path).unwrap());
        let reader = decoder.read_info().unwrap();
        let info = reader.info();

        match info.color_type {
            ColorType::Rgba => {
                if info.is_animated() {
                    Ok(Pngquant::decoder_rgba_png(path, reader))
                } else {
                    Ok(Pngquant::decoder_rgba_apng(path, reader))
                }
            }
            ColorType::Indexed => Err(Error::UnsupportedColorMode),
            _ => Err(Error::UnsupportedColorMode),
        }
    }

    fn decoder_rgba_png(path: &'a Path, mut reader: Reader<File>) -> Pngquant {
        let mut buf = vec![0; reader.output_buffer_size()];
        let output_info = reader.next_frame(&mut buf).unwrap();
        let bytes = Some(rgb::FromSlice::as_rgba(&buf[..output_info.buffer_size()]).to_vec());
        Pngquant {
            path,
            reader,
            bytes,
            frames: None,
            histogram: None,
        }
    }

    fn decoder_rgba_apng(path: &'a Path, mut reader: Reader<File>) -> Pngquant {
        let mut frames: Vec<Frame> = vec![];
        let mut attr = imagequant::new();
        attr.set_speed(1).unwrap();
        attr.set_quality(20, 100).unwrap();
        let mut histogram = imagequant::Histogram::new(&attr);
        loop {
            let mut buf = vec![0; reader.output_buffer_size()];
            if let Result::Ok(output) = reader.next_frame(&mut buf) {
                let info = reader.info();
                let bytes = &buf[..output.buffer_size()];
                if let Some(control) = info.frame_control() {
                    let frame = Frame::new(
                        bytes.to_vec(),
                        control.width,
                        control.height,
                        control.x_offset,
                        control.y_offset,
                        control.delay_num,
                        control.delay_den,
                        control.dispose_op,
                        control.blend_op,
                    );
                    let pixels = rgb::FromSlice::as_rgba(bytes);
                    // merge.extend_from_slice(pixels);
                    let mut image = imagequant::Image::new_borrowed(
                        &attr,
                        pixels,
                        control.width as usize,
                        control.height as usize,
                        0.0,
                    )
                    .unwrap();
                    histogram.add_image(&attr, &mut image).unwrap();
                    frames.push(frame);
                }
            } else {
                break;
            }
        }

        Pngquant {
            path,
            reader,
            bytes: None,
            frames: Some(frames),
            histogram: Some(histogram),
        }
    }
}
