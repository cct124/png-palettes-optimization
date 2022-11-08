use super::Pngquant;
use crate::thread::{ThreadPool, WorkStatus};
use png::Compression;
use std::ffi::OsStr;
use std::fs::{self, DirEntry};
use std::io;
use std::path::Path;
use std::sync::atomic::AtomicUsize;
use std::sync::Arc;
use std::thread::available_parallelism;

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
    /// 文件扩展名，用于检测png文件
    extension: &'a [&'a str],
    /// 扫描到的png文件路径都保存到这里
    worklist: Vec<Work>,
    /// 线程池
    thread_pool: ThreadPool,
    /// 记录完成的工作任务
    end_num: usize,
    /// png编码压缩等级
    compression: Compression,
}

impl<'a> Optimization<'a> {
    pub fn new(
        path: &'a Path,
        speed: Option<u8>,
        quality_min: Option<u8>,
        quality_max: Option<u8>,
        dithering_level: Option<f32>,
        compression: Compression,
    ) -> Optimization {
        // 系统并行资源
        let available_parallelism = available_parallelism().unwrap().get();
        // 根据并行资源数量创建线程池
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
            compression,
        }
    }

    /// 遍历工作路径下的所有目录文件
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
        // 文件扩展名是否是png文件
        if self.has_extension(&entry.path()) {
            // 是png文件存入数组
            paths.push(Work {
                id: paths.len(),
                path: entry,
                status: WorkStatus::INIT,
                progress: Arc::new(AtomicUsize::new(0)),
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

    /// 生成工作列表
    fn generate_worklist(&mut self) {
        let mut paths: Vec<Work> = vec![];
        self.visit_dirs(self.path, &mut |entry| self.iterate_pngs(entry, &mut paths))
            .unwrap();
        self.worklist = paths;
    }

    /// 执行数组中的工作任务
    fn run_worklist(&mut self) {
        // 主线程循环不断检查工作任务状态
        loop {
            for work in self.worklist.iter_mut() {
                // 只执行初始化的工作任务
                if let WorkStatus::INIT = work.status {
                    // 开始执行，工作任务状态改为等待
                    work.status = WorkStatus::WAIT;
                    let path = work.path.path();
                    let speed = self.speed;
                    let quality_max = self.quality_max;
                    let quality_min = self.quality_min;
                    let dithering_level = self.dithering_level;
                    let compression = self.compression;
                    let progress = Arc::clone(&work.progress);
                    // 多线程执行工作任务
                    self.thread_pool.execute(
                        move || {
                            if let Ok(pngquant) = Pngquant::new(
                                &path,
                                speed,
                                quality_min,
                                quality_max,
                                dithering_level,
                                progress,
                            )
                            .as_mut()
                            {
                                // 执行编码覆盖原文件
                                pngquant.encoder(
                                    pngquant.path,
                                    speed,
                                    quality_min,
                                    quality_max,
                                    compression,
                                );
                            }
                        },
                        work.id,
                    )
                }

                println!(
                    "{}",
                    work.get_progress()
                        .load(std::sync::atomic::Ordering::Relaxed)
                );
            }

            // 检查通道消息，执行工作的线程任务结束后将发消息到此通道
            if let Ok(status) = self.thread_pool.status_receiver.try_recv() {
                // 确定是哪个工作任务发出的消息
                let work = self.worklist.iter_mut().find(|work| work.id == status.id);
                if let Some(work) = work {
                    // 将工作任务状态改为已结束
                    work.status = WorkStatus::End;
                    self.end_num += 1;
                }
            };

            // 判断是否所有任务已完成
            if self.worklist.len() == self.end_num {
                // 退出循环
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
    // 工作id
    id: usize,
    // 工作路径
    path: DirEntry,
    // 工作状态
    status: WorkStatus,
    // 工作进度
    progress: Arc<AtomicUsize>,
}

impl Work {
    pub fn get_progress(&self) -> &Arc<AtomicUsize> {
        &self.progress
    }
}
