use crate::optimization::Optimization;
use clap::Parser;
use std::{env, path::Path};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(
        short = 'p',
        long,
        help = "要进行压缩png图的文件夹路径，传入当前工作路径的相对路径。 默认当前工作路径"
    )]
    path: Option<String>,

    #[arg(
        short = 's',
        long,
        help = "1-10，更快的速度生成的图像质量更低，可用于实时生成图像。默认值为 4"
    )]
    speed: Option<u8>,

    #[arg(long, help = "0-100，优化的最低质量，默认最低 0，不能高于最大值")]
    quality_min: Option<u8>,

    #[arg(long, help = "0-100，优化的最大质量，默认最高100，不能低于最小值")]
    quality_max: Option<u8>,
}

/**
 * # 处理命令行参数
 */
pub fn args_handle<'a>() {
    let args = Args::parse();
    println!("{:?}", args);

    let path = if let Some(path) = args.path {
        path
    } else {
        env::current_dir().unwrap().to_string_lossy().to_string()
    };

    let mut optimization = Optimization::new(
        Path::new(&path),
        args.speed,
        args.quality_min,
        args.quality_max,
    );
    optimization.quality();
    println!("{:?}", optimization);
}
