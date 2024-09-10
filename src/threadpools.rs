use once_cell::sync::Lazy;
use rayon::ThreadPool;

pub static READER_THREAD_POOL: Lazy<ThreadPool> = Lazy::new(|| {
    rayon::ThreadPoolBuilder::new()
        .num_threads(6)
        .build()
        .unwrap()
});

pub static WRITER_THREAD_POOL: Lazy<ThreadPool> = Lazy::new(|| {
    rayon::ThreadPoolBuilder::new()
        .num_threads(6)
        .build()
        .unwrap()
});
