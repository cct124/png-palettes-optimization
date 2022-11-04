use std::ffi::OsStr;
use std::fs::{self, DirEntry};
use std::io;
use std::path::Path;

#[derive(Debug)]
pub struct Optimization<'a> {
    /// 优化器工作路径
    path: &'a Path,
    /// `1-10`.
    ///更快的速度生成的图像质量更低，用于实时生成图像。
    ///默认值为 `4`。
    speed: Option<u8>,
    /// `0-100`，优化的最低质量，默认最低`0`，不能高于最大值
    quality_min: Option<u8>,
    /// `0-100`，优化的最大质量，默认最高`100`，不能低于最小值
    quality_max: Option<u8>,
    // 文件扩展名，用于检测png文件
    extension: &'a [&'a str],
    // 扫描到的png文件路径都保存到这里
    pngs_path: Vec<DirEntry>,
}

impl<'a> Optimization<'a> {
    pub fn new(
        path: &'a Path,
        speed: Option<u8>,
        quality_min: Option<u8>,
        quality_max: Option<u8>,
    ) -> Optimization {
        Optimization {
            path,
            speed,
            quality_min,
            quality_max,
            extension: &["png"],
            pngs_path: vec![],
        }
    }

    /// 遍历目录
    fn visit_dirs(&self, dir: &Path, cb: &mut dyn FnMut(DirEntry)) -> io::Result<()> {
        match dir.metadata() {
            Ok(_) => {
                for entry in fs::read_dir(dir)? {
                    let entry = entry?;
                    let path = entry.path();
                    if path.is_dir() {
                        self.visit_dirs(&path, cb)?;
                    } else {
                        cb(entry);
                    }
                }
                Ok(())
            }
            Err(err) => Err(err),
        }
    }

    /// 遍历目录查找png图片
    fn iterate_pngs(&self, entry: DirEntry, paths: &mut Vec<DirEntry>) {
        if self.has_extension(&entry.path()) {
            paths.push(entry)
        }
    }

    /// 检查文件扩展名
    fn has_extension(&self, path: &Path) -> bool {
        if let Some(ref extension) = path.extension().and_then(OsStr::to_str) {
            return self
                .extension
                .iter()
                .any(|x| x.eq_ignore_ascii_case(extension));
        }

        false
    }

    /// 优化图片
    pub fn quality(&mut self) {
        let mut paths: Vec<DirEntry> = vec![];
        self.visit_dirs(self.path, &mut |entry| self.iterate_pngs(entry, &mut paths))
            .unwrap();
        self.pngs_path = paths;
    }
}
