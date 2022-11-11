use imagequant::Histogram;
use png::{ColorType, Compression, Decoder, Reader};
use std::{fs::File, io::BufWriter, path::Path, sync::mpsc::SyncSender};

use super::Frame;
use crate::{error::Error, PROGRESS_CONSTANT};

#[derive(Debug)]
pub struct Progress {
    pub id: usize,
    pub value: f32,
}

/// PNG优化结构体
pub struct Pngquant<'a> {
    id: usize,
    /// png文件路径
    pub path: &'a Path,
    reader: Reader<File>,
    /// 图像数据
    bytes: Option<Vec<imagequant::RGBA>>,
    /// apng 帧数据
    frames: Option<Vec<Frame>>,
    histogram: Option<Histogram>,
    /// 优化参数设置
    imagequant_attr: Option<imagequant::Attributes>,
    /// 默认优化的最大质量
    def_quality_max: u8,
    /// 平滑图像参数
    dithering_level: Option<f32>,
    /// 进度发送
    progress_sender: SyncSender<Progress>,
    /// 源文件大小
    pub original_size: Option<u64>,
    /// 压缩文件大小
    pub size: Option<u64>,
}

impl<'a> Pngquant<'a> {
    pub fn new(
        id: usize,
        path: &'a Path,
        speed: Option<u8>,
        quality_min: Option<u8>,
        quality_max: Option<u8>,
        dithering_level: Option<f32>,
        progress_sender: SyncSender<Progress>,
    ) -> Result<Pngquant<'a>, Error> {
        let file = File::open(path).unwrap();
        let original_size = file.metadata().unwrap().len();
        let decoder = Decoder::new(file);
        let reader = decoder.read_info().unwrap();
        let info = reader.info();
        let def_quality_max: u8 = 60;
        // 根据颜色模式实例化不同的优化结构体，目前只支持优化Rgba模式的png图像
        match info.color_type {
            ColorType::Rgba => {
                // 是否是apng
                if info.is_animated() {
                    Ok(Pngquant::decoder_rgba_apng(
                        id,
                        path,
                        reader,
                        speed,
                        quality_min,
                        quality_max,
                        def_quality_max,
                        dithering_level,
                        progress_sender,
                        original_size,
                    ))
                } else {
                    Ok(Pngquant::decoder_rgba_png(
                        id,
                        path,
                        reader,
                        def_quality_max,
                        dithering_level,
                        progress_sender,
                        original_size,
                    ))
                }
            }
            // ColorType::Indexed => Err(Error::UnsupportedColorMode),
            _ => Err(Error::UnsupportedColorMode),
        }
    }

    /// 解码rgba的图像数据
    fn decoder_rgba_png(
        id: usize,
        path: &'a Path,
        mut reader: Reader<File>,
        def_quality_max: u8,
        dithering_level: Option<f32>,
        progress_sender: SyncSender<Progress>,
        original_size: u64,
    ) -> Pngquant<'a> {
        let mut buf = vec![0; reader.output_buffer_size()];
        let output_info = reader.next_frame(&mut buf).unwrap();
        let bytes = Some(rgb::FromSlice::as_rgba(&buf[..output_info.buffer_size()]).to_vec());
        Pngquant {
            id,
            path,
            reader,
            bytes,
            frames: None,
            histogram: None,
            imagequant_attr: None,
            def_quality_max,
            dithering_level,
            progress_sender,
            original_size: Some(original_size),
            size: None,
        }
    }

    /// 解码rgba的apng图像数据
    fn decoder_rgba_apng(
        id: usize,
        path: &'a Path,
        mut reader: Reader<File>,
        speed: Option<u8>,
        quality_min: Option<u8>,
        quality_max: Option<u8>,
        def_quality_max: u8,
        dithering_level: Option<f32>,
        progress_sender: SyncSender<Progress>,
        original_size: u64,
    ) -> Pngquant<'a> {
        let mut frames: Vec<Frame> = vec![];
        // 因为要为多个图像生成一个共享调色板，所以要提前生成
        let mut attr = imagequant::new();
        let sender = progress_sender.clone();

        // 调色板生成进度更新回调
        attr.set_progress_callback(move |progress| {
            // 将进度发送到主线程
            sender
                .send(Progress {
                    id,
                    value: progress,
                })
                .unwrap();
            imagequant::ControlFlow::Continue
        });

        // 设置压缩算法执行速度
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
        }

        // 为多个图像生成一个共享调色板
        let mut histogram = imagequant::Histogram::new(&attr);
        // 循环读取帧数据
        loop {
            let mut buf = vec![0; reader.output_buffer_size()];

            // 是否是最后一帧
            if let Result::Ok(output) = reader.next_frame(&mut buf) {
                let info = reader.info();
                let bytes = &buf[..output.buffer_size()];
                if let Some(control) = info.frame_control() {
                    // 将每帧数据保存
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
                    // 保存图像直方图，用于稍后的调色板生成
                    histogram.add_image(&attr, &mut image).unwrap();
                    frames.push(frame);
                }
            } else {
                break;
            }
        }

        Pngquant {
            id,
            path,
            reader,
            bytes: None,
            frames: Some(frames),
            histogram: Some(histogram),
            imagequant_attr: Some(attr),
            def_quality_max,
            dithering_level,
            progress_sender,
            original_size: Some(original_size),
            size: None,
        }
    }

    // 编码png
    pub fn encoder(
        &mut self,
        path: &Path,
        speed: Option<u8>,
        quality_min: Option<u8>,
        quality_max: Option<u8>,
        compression: Compression,
    ) {
        // 是否是apng根据类型执行不同的逻辑
        if let Some(bytes) = &self.bytes {
            let bytes = bytes.to_vec();
            self.encoder_png(bytes, path, speed, quality_min, quality_max, compression)
        }
        if let Some(_) = &self.frames {
            self.encoder_apng(path, compression)
        }
    }

    // 编码apng
    fn encoder_apng(&mut self, path: &Path, compression: Compression) {
        // apng对象数据
        if let (Some(histogram), Some(attr), Some(frames)) = (
            self.histogram.as_mut(),
            &self.imagequant_attr,
            self.frames.as_mut(),
        ) {
            // 为添加到直方图的所有图像/颜色生成调色板。
            let mut res = histogram.quantize(&attr).unwrap();
            // 设置平滑图像参数
            res.set_dithering_level(self.dithering_level.unwrap())
                .unwrap();
            // 用于保存调色板
            let mut histogram_palette: Vec<imagequant::RGBA> = vec![];

            // 读取每帧数据，将图像重新映射到调色板+索引中
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
                // 生成调色板和索引
                let (palette, pixels) = res.remapped(&mut image).unwrap();

                // 因为是共享调色板，保存一次就行了
                if histogram_palette.len() == 0 {
                    histogram_palette = palette;
                }
                // 保存索引数据
                frame.pixels = Some(pixels);
            }

            // 调色板数据格式转换为png规范
            let mut rbg_palette: Vec<u8> = Vec::new();
            let mut trns: Vec<u8> = Vec::new();

            for f in histogram_palette.iter() {
                rbg_palette.push(f.r);
                rbg_palette.push(f.g);
                rbg_palette.push(f.b);
                trns.push(f.a);
            }

            // 下面开始写入覆盖原png图像
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
                let id = self.id;
                encoder
                    .set_animated(animation.num_frames, animation.num_plays)
                    .unwrap();
                let mut writer = encoder.write_header().unwrap();

                // 每帧写入
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

                // 结束工作发送总进度
                let progress_sender = self.progress_sender.clone();
                progress_sender
                    .send(Progress { id, value: PROGRESS_CONSTANT })
                    .unwrap();

                // 记录压缩后的文件大小
                let file = File::open(path).unwrap();
                let size = file.metadata().unwrap().len();
                self.set_size(size);
            }
        }
    }

    fn encoder_png(
        &mut self,
        bytes: Vec<imagequant::RGBA>,
        path: &Path,
        speed: Option<u8>,
        quality_min: Option<u8>,
        quality_max: Option<u8>,
        compression: Compression,
    ) {
        let info = self.reader.info();
        let mut attr = imagequant::new();
        let progress_sender = self.progress_sender.clone();
        let id = self.id;

        // 调色板生成进度更新回调
        attr.set_progress_callback(move |progress| {
            // 将进度发送到主线程
            progress_sender
                .send(Progress {
                    id,
                    value: progress,
                })
                .unwrap();
            imagequant::ControlFlow::Continue
        });

        // 设置压缩算法执行速度
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
        }

        // 描述位图
        let mut img = attr
            .new_image(&bytes[..], info.width as usize, info.height as usize, 0.0)
            .unwrap();

        // 生成调色板
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
        let progress_sender = self.progress_sender.clone();
        // 结束工作发送总进度
        progress_sender
            .send(Progress { id, value: PROGRESS_CONSTANT })
            .unwrap();

        // 记录压缩后的文件大小
        let file = File::open(path).unwrap();
        let size = file.metadata().unwrap().len();
        self.set_size(size);
    }

    /// 记录压缩后的文件大小
    fn set_size(&mut self, size: u64) {
        self.size = Some(size);
    }
}
