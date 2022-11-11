mod command;
mod error;
mod optimization;
mod thread;

/// 用来转换文件大小
const BYTES_INTEGER: f64 = 1024.00;
/// 最终进度，保存文件占总进度的10%，所以设定为110
const PROGRESS_CONSTANT: f32 = 110.00;
/// 转换毫秒为秒
const SECOND_CONSTANT: f64 = 1000.00;

fn main() {
    command::args_handle();
}
