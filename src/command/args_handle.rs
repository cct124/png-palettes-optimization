use crate::optimization::Optimization;
use clap::Parser;
use std::{env, path::PathBuf};

#[derive(clap::ValueEnum, Clone, Debug)]
enum Compression {
    Default,
    Fast,
    Equal,
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(
        short = 'p',
        long,
        help = "要进行压缩png图的文件夹路径，传入当前工作路径的相对路径。 默认当前工作路径"
    )]
    path: Option<PathBuf>,

    #[arg(
        short = 's',
        long,
        help = "1-10，更快的速度生成的图像质量更低，可用于实时生成图像。默认值为 4"
    )]
    speed: Option<u8>,

    #[arg(
        short = 'n',
        long,
        help = "0-100，优化的最低质量，默认最低 0，不能高于最大值"
    )]
    quality_min: Option<u8>,

    #[arg(
        short = 'x',
        long,
        help = "0-100，优化的最大质量，默认最高100，不能低于最小值"
    )]
    quality_max: Option<u8>,

    #[arg(short = 'd', long, help = "设置为1.0可获得漂亮的平滑图像，默认 1.0")]
    dithering_level: Option<f32>,

    #[arg(
        short = 'c',
        long,
        help = "施加压缩的类型和强度，三种类型default、fast、equal，默认default，最好的压缩但时间会更长"
    )]
    compression: Option<Compression>,
}

/// 处理命令行参数
pub fn args_handle<'a>() {
    // 获取命令行参数
    let args = Args::parse();

    // 获取工作路径
    let path = if let Some(path) = args.path {
        path
    } else {
        env::current_dir().unwrap()
    };

    // 设置压缩等级

    let compression = match args.compression {
        Some(Compression::Fast) => png::Compression::Fast,
        Some(Compression::Equal) => png::Compression::Best,
        _ => png::Compression::Best,
    };

    // 实例化优化结构体
    let mut optimization = Optimization::new(
        &path,
        args.speed,
        args.quality_min,
        args.quality_max,
        args.dithering_level,
        compression,
    );
    // 优化压缩png图像
    optimization.quality();
}
