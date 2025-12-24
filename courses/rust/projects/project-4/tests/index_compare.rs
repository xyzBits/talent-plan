// use std::collections::BTreeMap;
// use std::sync::{Arc, RwLock};
// use std::thread;
// use std::time::Instant;
//
// use crossbeam_skiplist::SkipMap;
//
// fn run_skipmap(num_keys: usize, num_readers: usize, num_writers: usize, ops_per_thread: usize) -> std::time::Duration {
//     let map = Arc::new(SkipMap::new());
//     for i in 0..num_keys {
//         map.insert(format!("k{}", i), format!("v{}", i));
//     }
//
//     let start = Instant::now();
//     let mut handles = Vec::new();
//
//     for _ in 0..num_readers {
//         let map = Arc::clone(&map);
//         handles.push(thread::spawn(move || {
//             for i in 0..ops_per_thread {
//                 let key = format!("k{}", i % num_keys);
//                 let _ = map.get(&key).map(|e| e.value().clone());
//             }
//         }));
//     }
//
//     for w in 0..num_writers {
//         let map = Arc::clone(&map);
//         handles.push(thread::spawn(move || {
//             for i in 0..ops_per_thread {
//                 let key = format!("k{}", (i + w * 13) % num_keys);
//                 map.insert(key, format!("v{}", i));
//                 if i % 10 == 0 {
//                     let _ = map.remove(&format!("k{}", i % num_keys));
//                 }
//             }
//         }));
//     }
//
//     for h in handles {
//         h.join().unwrap();
//     }
//     start.elapsed()
// }
//
// fn run_btreemap(num_keys: usize, num_readers: usize, num_writers: usize, ops_per_thread: usize) -> std::time::Duration {
//     let map = Arc::new(RwLock::new(BTreeMap::new()));
//     {
//         let mut g = map.write().unwrap();
//         for i in 0..num_keys {
//             g.insert(format!("k{}", i), format!("v{}", i));
//         }
//     }
//
//     let start = Instant::now();
//     let mut handles = Vec::new();
//
//     for _ in 0..num_readers {
//         let map = Arc::clone(&map);
//         handles.push(thread::spawn(move || {
//             for i in 0..ops_per_thread {
//                 let key = format!("k{}", i % num_keys);
//                 let g = map.read().unwrap();
//                 let _ = g.get(&key).cloned();
//             }
//         }));
//     }
//
//     for w in 0..num_writers {
//         let map = Arc::clone(&map);
//         handles.push(thread::spawn(move || {
//             for i in 0..ops_per_thread {
//                 let key = format!("k{}", (i + w * 13) % num_keys);
//                 let mut g = map.write().unwrap();
//                 g.insert(key.clone(), format!("v{}", i));
//                 if i % 10 == 0 {
//                     g.remove(&format!("k{}", i % num_keys));
//                 }
//                 drop(g);
//             }
//         }));
//     }
//
//     for h in handles {
//         h.join().unwrap();
//     }
//     start.elapsed()
// }
//
// #[test]
// fn compare_skipmap_vs_rwlock_btreemap() {
//     // Parameters — tuned to be moderate so tests finish quickly
//     let num_keys = 1000;
//     let num_readers = 8;
//     let num_writers = 4;
//     let ops_per_thread = 5_000;
//
//     let t1 = run_skipmap(num_keys, num_readers, num_writers, ops_per_thread);
//     println!("SkipMap elapsed: {:?}", t1);
//
//     let t2 = run_btreemap(num_keys, num_readers, num_writers, ops_per_thread);
//     println!("RwLock<BTreeMap> elapsed: {:?}", t2);
//
//     // test always passes — this file is for comparison/measurement
//     assert!(true);
// }
