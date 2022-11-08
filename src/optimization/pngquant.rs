use imagequant::Histogram;
use png::{ColorType, Compression, Decoder, Reader};
use std::{
    fs::File,
    io::BufWriter,
    path::Path,
    sync::{atomic::AtomicUsize, Arc, Mutex},
};

use super::Frame;
use crate::error::Error;

/// PNG优化结构体
pub struct Pngquant<'a> {
    /// png文件路径
    pub path: &'a Path,
    reader: Reader<File>,
    /// 图像数据
    bytes: Option<Vec<imagequant::RGBA>>,
    /// apng 帧数据
    frames: Option<Vec<Frame>>,
    histogram: Option<Histogram>,
    imagequant_attr: Option<imagequant::Attributes>,
    /// 默认优化的最大质量
    def_quality_max: u8,
    /// 平滑图像参数
    dithering_level: Option<f32>,
    progress: Arc<AtomicUsize>,
}

impl<'a> Pngquant<'a> {
    pub fn new(
        path: &'a Path,
        speed: Option<u8>,
        quality_min: Option<u8>,
        quality_max: Option<u8>,
        dithering_level: Option<f32>,
        progress: Arc<AtomicUsize>,
    ) -> Result<Pngquant<'a>, Error> {
        let decoder = Decoder::new(File::open(path).unwrap());
        let reader = decoder.read_info().unwrap();
        let info = reader.info();
        let def_quality_max: u8 = 60;
        // 根据颜色模式实例化不同的优化结构体，目前只支持优化Rgba模式的png图像
        match info.color_type {
            ColorType::Rgba => {
                // 是否是apng
                if info.is_animated() {
                    Ok(Pngquant::decoder_rgba_apng(
                        path,
                        reader,
                        speed,
                        quality_min,
                        quality_max,
                        def_quality_max,
                        dithering_level,
                        progress,
                    ))
                } else {
                    Ok(Pngquant::decoder_rgba_png(
                        path,
                        reader,
                        def_quality_max,
                        dithering_level,
                        progress,
                    ))
                }
            }
            // ColorType::Indexed => Err(Error::UnsupportedColorMode),
            _ => Err(Error::UnsupportedColorMode),
        }
    }

    /// 解码rgba的图像数据
    fn decoder_rgba_png(
        path: &'a Path,
        mut reader: Reader<File>,
        def_quality_max: u8,
        dithering_level: Option<f32>,
        progress: Arc<AtomicUsize>,
    ) -> Pngquant<'a> {
        let mut buf = vec![0; reader.output_buffer_size()];
        let output_info = reader.next_frame(&mut buf).unwrap();
        let bytes = Some(rgb::FromSlice::as_rgba(&buf[..output_info.buffer_size()]).to_vec());
        Pngquant {
            path,
            reader,
            bytes,
            frames: None,
            histogram: None,
            imagequant_attr: None,
            def_quality_max,
            dithering_level,
            progress,
        }
    }

    /// 解码rgba的apng图像数据
    fn decoder_rgba_apng(
        path: &'a Path,
        mut reader: Reader<File>,
        speed: Option<u8>,
        quality_min: Option<u8>,
        quality_max: Option<u8>,
        def_quality_max: u8,
        dithering_level: Option<f32>,
        progress: Arc<AtomicUsize>,
    ) -> Pngquant<'a> {
        let mut frames: Vec<Frame> = vec![];
        // 因为要为多个图像生成一个共享调色板，所以要提前生成
        let mut attr = imagequant::new();

        if let Some(speed) = speed {
            attr.set_speed(speed as i32).unwrap();
        }

        // 默认质量的参数设置
        match (quality_min, quality_max) {
            (Some(quality_min), Some(quality_max)) => {
                attr.set_quality(quality_min, quality_max).unwrap()
            }
            (Some(quality_min), None) => attr.set_quality(quality_min, def_quality_max).unwrap(),
            (None, Some(quality_max)) => attr.set_quality(0, quality_max).unwrap(),
            (None, None) => attr.set_quality(0, def_quality_max).unwrap(),
            _ => {}
        }

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
            imagequant_attr: Some(attr),
            def_quality_max,
            dithering_level,
            progress,
        }
    }

    pub fn encoder(
        &mut self,
        path: &Path,
        speed: Option<u8>,
        quality_min: Option<u8>,
        quality_max: Option<u8>,
        compression: Compression,
    ) {
        if let Some(bytes) = &self.bytes {
            self.encoder_png(bytes, path, speed, quality_min, quality_max, compression)
        }
        if let Some(_) = &self.frames {
            self.encoder_apng(path, compression)
        }
    }

    fn encoder_apng(&mut self, path: &Path, compression: Compression) {
        if let (Some(histogram), Some(attr), Some(frames)) = (
            self.histogram.as_mut(),
            &self.imagequant_attr,
            self.frames.as_mut(),
        ) {
            let mut res = histogram.quantize(&attr).unwrap();
            res.set_dithering_level(self.dithering_level.unwrap())
                .unwrap();

            let mut histogram_palette: Vec<imagequant::RGBA> = vec![];

            for frame in frames.iter_mut() {
                let pixels = rgb::FromSlice::as_rgba(&frame.data[..]);
                let mut image = imagequant::Image::new_borrowed(
                    &attr,
                    pixels,
                    frame.width as usize,
                    frame.height as usize,
                    0.0,
                )
                .unwrap();
                let (palette, pixels) = res.remapped(&mut image).unwrap();
                if histogram_palette.len() == 0 {
                    histogram_palette = palette;
                }
                frame.pixels = Some(pixels);
            }

            let mut rbg_palette: Vec<u8> = Vec::new();
            let mut trns: Vec<u8> = Vec::new();

            for f in histogram_palette.iter() {
                rbg_palette.push(f.r);
                rbg_palette.push(f.g);
                rbg_palette.push(f.b);
                trns.push(f.a);
            }

            let info = self.reader.info();

            let file = File::create(path).unwrap();
            let ref mut w = BufWriter::new(file);

            let mut encoder = png::Encoder::new(w, info.width, info.height); // Width is 2 pixels and height is 1.
            encoder.set_depth(png::BitDepth::Eight);
            encoder.set_compression(compression);
            encoder.set_color(png::ColorType::Indexed);
            encoder.set_trns(trns);
            encoder.set_palette(rbg_palette);

            if let Some(animation) = info.animation_control {
                encoder
                    .set_animated(animation.num_frames, animation.num_plays)
                    .unwrap();
                let mut writer = encoder.write_header().unwrap();

                for frame in frames.iter() {
                    if let Some(pixels) = &frame.pixels {
                        writer.reset_frame_position().unwrap();
                        writer
                            .set_frame_dimension(frame.width, frame.height)
                            .unwrap();
                        writer
                            .set_frame_position(frame.x_offset, frame.y_offset)
                            .unwrap();
                        writer
                            .set_frame_delay(frame.delay_num, frame.delay_den)
                            .unwrap();
                        writer.set_blend_op(frame.blend_op).unwrap();
                        writer.set_dispose_op(frame.dispose_op).unwrap();
                        writer.write_image_data(&pixels).unwrap(); // Save
                    }
                }
            }
        }
    }

    fn encoder_png(
        &self,
        bytes: &Vec<imagequant::RGBA>,
        path: &Path,
        speed: Option<u8>,
        quality_min: Option<u8>,
        quality_max: Option<u8>,
        compression: Compression,
    ) {
        let info = self.reader.info();
        let mut attr = imagequant::new();
        let val = Arc::clone(&self.progress);
        attr.set_progress_callback(move |progress| {
            val.fetch_add(progress as usize, std::sync::atomic::Ordering::SeqCst);
            imagequant::ControlFlow::Continue
        });

        if let Some(speed) = speed {
            attr.set_speed(speed as i32).unwrap();
        }
        match (quality_min, quality_max) {
            (Some(quality_min), Some(quality_max)) => {
                attr.set_quality(quality_min, quality_max).unwrap()
            }
            (Some(quality_min), None) => {
                attr.set_quality(quality_min, self.def_quality_max).unwrap()
            }
            (None, Some(quality_max)) => attr.set_quality(0, quality_max).unwrap(),
            (None, None) => attr.set_quality(0, self.def_quality_max).unwrap(),
            _ => {}
        }

        // Describe the bitmap
        let mut img = attr
            .new_image(&bytes[..], info.width as usize, info.height as usize, 0.0)
            .unwrap();

        // The magic happens in quantize()
        let mut res = match attr.quantize(&mut img) {
            Ok(res) => res,
            Err(err) => panic!("Quantization failed, because: {:?}", err),
        };

        // Enable dithering for subsequent remappings
        res.set_dithering_level(self.dithering_level.unwrap())
            .unwrap();

        // You can reuse the result to generate several images with the same palette
        let (palette, pixels) = res.remapped(&mut img).unwrap();

        let mut rbg_palette: Vec<u8> = Vec::new();
        let mut trns: Vec<u8> = Vec::new();

        for f in palette.iter() {
            rbg_palette.push(f.r);
            rbg_palette.push(f.g);
            rbg_palette.push(f.b);
            trns.push(f.a);
        }

        let file = File::create(path).unwrap();
        let ref mut w = BufWriter::new(file);

        let mut encoder = png::Encoder::new(w, info.width, info.height); // Width is 2 pixels and height is 1.
        encoder.set_depth(png::BitDepth::Eight);
        encoder.set_compression(compression);
        encoder.set_color(png::ColorType::Indexed);
        encoder.set_trns(trns);
        encoder.set_palette(rbg_palette);

        let mut writer = encoder.write_header().unwrap();
        writer.write_image_data(&pixels).unwrap(); // Save
    }
}
