// 假设这是你要测试的函数

use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn fibonacci(n: u64) -> u64 {
    match n {
        0 => 1,
        1 => 1,
        n => fibonacci(n - 1) + fibonacci(n - 2),
    }
}

fn criterion_benchmark(c: &mut Criterion) {
    // fib 20 是这个测试的名字
    c.bench_function("fib 20", |b| {
       // b.iter 会运行这个闭包很多次
        // black_box 防止编译器因为结果 未被使用而把代码优化掉
        b.iter(|| fibonacci(black_box(20)))
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);