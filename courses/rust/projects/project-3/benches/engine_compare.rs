use criterion::{black_box, Criterion, criterion_group, criterion_main};
use kvs::{KvStore, KvsEngine, SledKvsEngine};
use tempfile::TempDir;

/// 基准测试：KvStore set
fn bench_kvstore_set(c: &mut Criterion) {
    let temp_dir = TempDir::new().unwrap();
    let mut store = KvStore::open(temp_dir.path()).unwrap();

    c.bench_function("KvStore set 1000", |b| {
        b.iter(|| {
            for i in 0..1000 {
                store.set(format!("key{}", i), format!("value{}", i)).unwrap();
            }
        })
    });
}

/// 基准测试：KvStore get
fn bench_kvstore_get(c: &mut Criterion) {
    let temp_dir = TempDir::new().unwrap();
    let mut store = KvStore::open(temp_dir.path()).unwrap();

    for i in 0..1000 {
        store.set(format!("key{}", i), format!("value{}", i)).unwrap();
    }

    c.bench_function("KvStore get 1000", |b| {
        b.iter(|| {
            for i in 0..1000 {
                black_box(store.get(format!("key{}", i)).unwrap());
            }
        })
    });
}

/// 基准测试：sled set
fn bench_sled_set(c: &mut Criterion) {
    let temp_dir = TempDir::new().unwrap();
    let mut store = SledKvsEngine::new(sled::open(temp_dir.path()).unwrap());

    c.bench_function("sled set 1000", |b| {
        b.iter(|| {
            for i in 0..1000 {
                store.set(format!("key{}", i), format!("value{}", i)).unwrap();
            }
        })
    });
}

/// 基准测试：sled get
fn bench_sled_get(c: &mut Criterion) {
    let temp_dir = TempDir::new().unwrap();
    let mut store = SledKvsEngine::new(sled::open(temp_dir.path()).unwrap());

    for i in 0..1000 {
        store.set(format!("key{}", i), format!("value{}", i)).unwrap();
    }

    c.bench_function("sled get 1000", |b| {
        b.iter(|| {
            for i in 0..1000 {
                black_box(store.get(format!("key{}", i)).unwrap());
            }
        })
    });
}

criterion_group!(
    benches,
    bench_kvstore_set,
    bench_kvstore_get,
    bench_sled_set,
    bench_sled_get
);
criterion_main!(benches);