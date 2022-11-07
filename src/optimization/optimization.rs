use std::ffi::OsStr;
use std::fs::{self, DirEntry};
use std::io;
use std::path::Path;
use std::thread::available_parallelism;

use crate::thread::{ThreadPool, WorkStatus};

use super::Pngquant;

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
    /// 设置为1.0可获得漂亮的平滑图像，默认 1.0
    dithering_level: Option<f32>,
    // 文件扩展名，用于检测png文件
    extension: &'a [&'a str],
    // 扫描到的png文件路径都保存到这里
    worklist: Vec<Work>,
    thread_pool: ThreadPool,
    end_num: usize,
}

impl<'a> Optimization<'a> {
    pub fn new(
        path: &'a Path,
        speed: Option<u8>,
        quality_min: Option<u8>,
        quality_max: Option<u8>,
        dithering_level: Option<f32>,
    ) -> Optimization {
        // 并行资源
        let available_parallelism = available_parallelism().unwrap().get();
        let thread_pool = ThreadPool::new(available_parallelism);
        Optimization {
            path,
            speed,
            quality_min,
            quality_max,
            extension: &["png"],
            worklist: vec![],
            thread_pool,
            end_num: 0,
            dithering_level: Some(dithering_level.unwrap_or(1.0)),
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
    fn iterate_pngs(&self, entry: DirEntry, paths: &mut Vec<Work>) {
        if self.has_extension(&entry.path()) {
            paths.push(Work {
                id: paths.len(),
                path: entry,
                status: WorkStatus::INIT,
            })
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

    //生成工作列表
    fn generate_worklist(&mut self) {
        let mut paths: Vec<Work> = vec![];
        self.visit_dirs(self.path, &mut |entry| self.iterate_pngs(entry, &mut paths))
            .unwrap();
        self.worklist = paths;
    }

    fn run_worklist(&mut self) {
        loop {
            for work in self.worklist.iter_mut() {
                if let WorkStatus::INIT = work.status {
                    work.status = WorkStatus::WAIT;
                    let path = work.path.path();
                    let speed = self.speed;
                    let quality_max = self.quality_max;
                    let quality_min = self.quality_min;
                    let dithering_level = self.dithering_level;
                    self.thread_pool.execute(
                        move || {
                            if let Ok(pngquant) = Pngquant::new(
                                &path,
                                speed,
                                quality_min,
                                quality_max,
                                dithering_level,
                            )
                            .as_mut()
                            {
                                pngquant.encoder(pngquant.path, speed, quality_min, quality_max);
                            }
                        },
                        work.id,
                    )
                }
            }
            if let Ok(status) = self.thread_pool.status_receiver.try_recv() {
                let work = self.worklist.iter_mut().find(|work| work.id == status.id);
                if let Some(work) = work {
                    work.status = WorkStatus::End;
                    self.end_num += 1;
                }
            };

            if self.worklist.len() == self.end_num {
                break;
            }
        }
    }

    /// 优化图片
    pub fn quality(&mut self) {
        self.generate_worklist();
        self.run_worklist();
    }
}

#[derive(Debug)]
struct Work {
    id: usize,
    path: DirEntry,
    status: WorkStatus,
}
