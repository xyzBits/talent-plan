use std::cell::RefCell;
use std::collections::BTreeMap;
use std::ffi::OsStr;
use std::fs::{self, File, OpenOptions};
use std::io::{self, BufReader, BufWriter, Read, Seek, SeekFrom, Write};
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use crossbeam_skiplist::SkipMap;
use log::error;
use serde::{Deserialize, Serialize};
use serde_json::Deserializer;

use super::KvsEngine;
use crate::{KvsError, Result};

const COMPACTION_THRESHOLD: u64 = 1024 * 1024;

/// The `KvStore` stores string key/value pairs.
///
/// Key/value pairs are persisted to disk in log files. Log files are named after
/// monotonically increasing generation numbers with a `log` extension name.
/// A skip list in memory stores the keys and the value locations for fast query.
///
/// ```rust
/// # use kvs::{KvStore, Result};
/// # fn try_main() -> Result<()> {
/// use std::env::current_dir;
/// use kvs::KvsEngine;
/// let mut store = KvStore::open(current_dir()?)?;
/// store.set("key".to_owned(), "value".to_owned())?;
/// let val = store.get("key".to_owned())?;
/// assert_eq!(val, Some("value".to_owned()));
/// # Ok(())
/// # }
/// ```
#[derive(Clone)]
pub struct KvStore {
    // directory for the log and other data
    path: Arc<PathBuf>,
    // map generation number to the file reader
    // skipMap 提供高并发的全局无锁访问，减少锁的竞争，也可以按顺序遍历
    // ConcurrentSkipListMap
    index: Arc<SkipMap<String, CommandPos>>,
    // 外部 get 调用时使用，读取文件
    reader: KvStoreReader,

    // 写入必须是串行的，所以要回销 读：走index + reader 无锁 ，写 走writer 互斥锁，串行化
    // 里面的 reader 在 压缩时使用
    writer: Arc<Mutex<KvStoreWriter>>,
}

// 详细中文注释（补充）：
// 设计背景与总体说明：
// 1) 存储模型：本实现采用 Log-Structured 的设计思想——
//    所有变更都追加写到日志文件（按 generation 编号），读取通过内存索引定位到磁盘上的位置然后读取对应字节区间。
// 2) 并发与同步策略：
//    - 读取路径尽量无锁：使用 `SkipMap`（并发跳表）作为内存索引，支持并发读访问，减少锁竞争。
//    - 写入路径串行化：所有写操作都通过 `KvStoreWriter` 串行化执行（由 `Arc<Mutex<KvStoreWriter>>` 保护），保证 append 写入的顺序性与一致性。
//    - 这种 "读无锁、写串行" 的模式在很多键值存储中被采用，因为写入需要维护磁盘上的顺序语义，而读取是高频操作。
// 3) 恢复与可靠性：
//    - 启动时会扫描现有日志（`sorted_gen_list` + `load`），通过重放日志重建内存索引（index），从而实现崩溃恢复与持久性保证。
// 4) 空间回收（压缩/compaction）：
//    - 当日志中存在大量被覆盖或删除的旧记录时，会触发 `compact()`，将当前有效的数据搬运到新的代数文件中，删除旧文件以释放磁盘空间。
//    - `KvStoreReader` 与 `safe_point` 协同工作：在压缩期间，reader 会通过 `safe_point` 判断哪些文件句柄可以关闭，从而避免并发读取时访问已删除文件。
// 5) 对 Rust 新手的阅读建议：
//    - 先关注 `set/get/remove` 的高层逻辑（在 `impl KvsEngine for KvStore` 中）。
//    - 理解 `KvStoreWriter` 和 `KvStoreReader` 的职责分离：writer 负责写入与 compaction，reader 负责按需打开/读取文件。
//    - `Arc`/`Mutex`/`RefCell`/`SkipMap` 是关键的并发原语，分别用于跨线程共享、互斥串行化、内部可变性和并发索引。

impl KvStore {
    /// Opens a `KvStore` with the given path.
    ///
    /// This will create a new directory if the given one does not exist.
    ///
    /// # Errors
    ///
    /// It propagates I/O or deserialization errors during the log replay.
    /// 组装 文件、内存索引 、读写器
    /// 组装过程中，构建好线程安全和并发隔离的基础设施
    pub fn open(path: impl Into<PathBuf>) -> Result<KvStore> {
        let path = Arc::new(path.into());
        // let buf: PathBuf = *path;
        // fs::create_dir_all(path.as_ref())?;
        fs::create_dir_all(&*path)?;

        let mut readers = BTreeMap::new();

        // skipMap 允许无锁并发读取
        let index = Arc::new(SkipMap::new());

        let gen_list = sorted_gen_list(&path)?;
        let mut uncompacted = 0;

        for &gen in &gen_list {
            let mut reader = BufReaderWithPos::new(File::open(log_path(&path, gen))?)?;
            uncompacted += load(gen, &mut reader, &*index)?;

            // 历史文件的读取器都缓存 起来
            readers.insert(gen, reader);
        }

        let current_gen = gen_list.last().unwrap_or(&0) + 1;

        // 旧文件只读，不能写入，所以每次重启都生成新的
        let writer = new_log_file(&path, current_gen)?;
        let safe_point = Arc::new(AtomicU64::new(0));

        let reader = KvStoreReader {
            path: Arc::clone(&path),
            safe_point,
            readers: RefCell::new(readers),
        };

        let writer = KvStoreWriter {
            reader: reader.clone(),// writer 中也装了一个 reader ，因为在压缩时，要使用reader读取旧数据
            writer,// 当前需要写的
            current_gen,
            uncompacted,
            path: Arc::clone(&path),
            index: Arc::clone(&index),
        };

        Ok(KvStore {
            path,
            reader,
            index,
            writer: Arc::new(Mutex::new(writer)),
        })
    }
}

impl KvsEngine for KvStore {
    /// Sets the value of a string key to a string.
    ///
    /// If the key already exists, the previous value will be overwritten.
    ///
    /// # Errors
    ///
    /// It propagates I/O or serialization errors during writing the log.
    fn set(&self, key: String, value: String) -> Result<()> {
        // 在这一层加锁了，所以下面的 set 不用考虑锁
        self.writer.lock().unwrap().set(key, value)
    }

    /// Gets the string value of a given string key.
    ///
    /// Returns `None` if the given key does not exist.
    fn get(&self, key: String) -> Result<Option<String>> {
        // 查索引  skipMap 索引不存在，直接返回
        if let Some(cmd_pos) = self.index.get(&key) {

            // 索引中有，拿到位置信息，(id offset length) 去 disk 读， read_command 将 disk 二进制 变成 command
            if let Command::Set { value, .. } = self.reader.read_command(*cmd_pos.value())? {
                Ok(Some(value))
            } else {
                Err(KvsError::UnexpectedCommandType)
            }
        } else {
            Ok(None)
        }
    }

    /// Removes a given key.
    ///
    /// # Error
    ///
    /// It returns `KvsError::KeyNotFound` if the given key is not found.
    ///
    /// It propagates I/O or serialization errors during writing the log.
    fn remove(&self, key: String) -> Result<()> {
        self.writer.lock().unwrap().remove(key)
    }
}

/// A single thread reader.
///
/// Each `KvStore` instance has its own `KvStoreReader` and
/// `KvStoreReader`s open the same files separately. So the user
/// can read concurrently through multiple `KvStore`s in different
/// threads.
// 详细中文注释（补充）：
// KvStoreReader 的目的与行为：
// - 每个 `KvStore` 实例包含一个 `KvStoreReader`，用于按需打开并复用文件句柄以便读取日志中的命令。
// - `KvStoreReader` 内部使用 `RefCell<BTreeMap<u64, BufReaderWithPos<File>>>` 缓存已经打开的文件句柄，避免频繁 open/close。
// - `safe_point` 表示最近一次 compaction 生成的代数（generation），当某个文件的代数小于 `safe_point` 时，意味着该文件已经是陈旧的，
//   可以在保证没有并发读取的前提下关闭对应句柄并删除物理文件（在 Windows 上文件删除需要句柄释放后才能完成）。
// - 设计要点：读取路径要尽量避免阻塞写路径，`KvStoreReader` 的 `read_and` 方法采用借用（borrow_mut）打开/复用句柄并定位到指定偏移再读取固定长度，
//   从而保证读取的局部性和效率。
struct KvStoreReader {
    // arc 共享所有权，共享路径对象
    path: Arc<PathBuf>,

    // 安全水位线，记录了最新一次压缩产生的文件代号民，是读线程和写线程之间的信号号
    // 压缩发生时，旧的日志文件会被合并成一个新的大文件
    // atomic64 保证了原子性，多个线程可以安全的读取
    // 作用：防止读取已经失效或被删除的旧文件，如果reader试图访问一个小于 safe_point 的是文件id，或能需要重定向去读新的压缩文件，或者直接报错
    // generation of the latest compaction file
    safe_point: Arc<AtomicU64>,

    // 在读的时候，还要修改reader的位置，但get方法的签名是 &self
    // 这里还是没太懂
    readers: RefCell<BTreeMap<u64, BufReaderWithPos<File>>>,
}

impl KvStoreReader {
    /// Close file handles with generation number less than safe_point.
    ///
    /// `safe_point` is updated to the latest compaction gen after a compaction finishes.
    /// The compaction generation contains the sum of all operations before it and the
    /// in-memory index contains no entries with generation number less than safe_point.
    /// So we can safely close those file handles and the stale files can be deleted.
    /// 关闭交移除那些已经被压缩过的，过期的文件handler，防止 handler 泄露
    /// bitcask 中，执行 compact 后，旧文件中的有效数据搬到新的，
    /// 更新水位，全局变量 safe_point 更新为3，id < 3的都是垃圾
    ///
    fn close_stale_handles(&self) {
        // 获取写锁，RefCell
        // 要从map 中删除元素，所以需要可变借用
        let mut readers = self.readers.borrow_mut();

        while !readers.is_empty() {
            // 拿出 map 中id 最小的言论的 id,BTreemap 是有序的，next 返回的永远是最小的
            let first_gen = *readers.keys().next().unwrap();
            if self.safe_point.load(Ordering::SeqCst) <= first_gen {
                break;
            }

            // remove 会 drop file 对象
            readers.remove(&first_gen);
        }
    }

    /// Read the log file at the given `CommandPos`.
    fn read_and<F, R>(&self, cmd_pos: CommandPos, f: F) -> Result<R>
    where
        // 定义闭包类型，接收一个受限的文件流，返回任意结果 R
        // read_and 不关心读出来的数据做什么，只读
        // io::Take 划定安全边界，防止多读
        F: FnOnce(io::Take<&mut BufReaderWithPos<File>>) -> Result<R>,
    {
        // 清理过期文件句柄，如果有压缩发生
        self.close_stale_handles();

        // borrow_mut 拿到独占访问权，可以insert 或者 seek 指针
        let mut readers = self.readers.borrow_mut();
        // Open the file if we haven't opened it in this `KvStoreReader`.
        // We don't use entry API here because we want the errors to be propogated.
        // 懒加载，如果这个 id 的文件还没打开过，现在打开并存入 缓存
        if !readers.contains_key(&cmd_pos.gen) {
            let reader = BufReaderWithPos::new(File::open(log_path(&self.path, cmd_pos.gen))?)?;
            readers.insert(cmd_pos.gen, reader);
        }

        // 拿到文件 handler
        let reader = readers.get_mut(&cmd_pos.gen).unwrap();
        // 定位
        reader.seek(SeekFrom::Start(cmd_pos.pos))?;
        // 读取固定长度
        let cmd_reader = reader.take(cmd_pos.len);

        // 执行回调
        // 把准备好的，对准了位置的，限制了长度的 reader 交给闭包处理
        f(cmd_reader)
    }

    // Read the log file at the given `CommandPos` and deserialize it to `Command`.
    fn read_command(&self, cmd_pos: CommandPos) -> Result<Command> {
        // 调用底层的读取器 read_and
        self.read_and(cmd_pos, |cmd_reader| {
            // 传入一个闭包，（回调函数 ）
            // 给你一个已经对准标准公交车的文件流，把它解析为 json command
            Ok(serde_json::from_reader(cmd_reader)?)
        })
    }
}

impl Clone for KvStoreReader {
    fn clone(&self) -> KvStoreReader {
        KvStoreReader {
            path: Arc::clone(&self.path),
            safe_point: Arc::clone(&self.safe_point),
            // don't use other KvStoreReader's readers
            readers: RefCell::new(BTreeMap::new()),
        }
    }
}

// 详细中文注释（补充）：
// KvStoreWriter 的职责与重要设计点：
// - 负责将 `set`/`remove` 命令序列化并追加写入当前代数日志文件，维护 `index`（内存索引）以及记录 `uncompacted` 大小。
// - 写入必须串行：因此 `KvStoreWriter` 被放在 `Arc<Mutex<...>>` 之内，外部在执行 `set`/`remove` 时会获取互斥锁，
//   保证不会同时有多个写线程破坏日志顺序或索引一致性。
// - `uncompacted`：统计可以回收的“垃圾”字节数（旧的被覆盖或删除的记录占用的空间），用来触发 `compact()`。
// - `compact()`：将当前 index 指向的有效数据搬运到新的 compaction 文件中，更新 index 并删除旧日志文件，释放空间。
// - 关于为什么读写分离：读者通过 `KvStoreReader` 使用 `SkipMap` 无锁读取索引并定位到磁盘位置，然后直接读磁盘数据；写操作走串行化路径，避免了复杂的并发控制。
// - 对新手的提示：保证 `KvStoreWriter` 的操作尽量短小（快速 append + flush），避免在持锁期间做大量 CPU 或阻塞 IO 操作，以减少对读操作的影响。
struct KvStoreWriter {
    reader: KvStoreReader,
    writer: BufWriterWithPos<File>,
    current_gen: u64,
    // the number of bytes representing "stale" commands that could be
    // deleted during a compaction
    uncompacted: u64,
    path: Arc<PathBuf>,
    index: Arc<SkipMap<String, CommandPos>>,
}

impl KvStoreWriter {
    fn set(&mut self, key: String, value: String) -> Result<()> {
        let cmd = Command::set(key, value);

        // writer 当前写到哪个位置了
        let pos = self.writer.pos;
        serde_json::to_writer(&mut self.writer, &cmd)?;

        self.writer.flush()?;
        if let Command::Set { key, .. } = cmd {
            if let Some(old_cmd) = self.index.get(&key) {
                self.uncompacted += old_cmd.value().len;
            }
            self.index
                .insert(key, (self.current_gen, pos..self.writer.pos).into());
        }

        if self.uncompacted > COMPACTION_THRESHOLD {
            self.compact()?;
        }
        Ok(())
    }

    fn remove(&mut self, key: String) -> Result<()> {
        if self.index.contains_key(&key) {
            // 先将命令 log，再append log
            let cmd = Command::remove(key);
            let pos = self.writer.pos;
            serde_json::to_writer(&mut self.writer, &cmd)?;
            self.writer.flush()?;

            if let Command::Remove { key } = cmd {
                let old_cmd = self.index.remove(&key).expect("key not found");

                // set 命令的长度
                self.uncompacted += old_cmd.value().len;
                // the "remove" command itself can be deleted in the next compaction
                // so we add its length to `uncompacted`
                // remove 命令的长度
                self.uncompacted += self.writer.pos - pos;
            }

            if self.uncompacted > COMPACTION_THRESHOLD {
                self.compact()?;
            }
            Ok(())
        } else {
            Err(KvsError::KeyNotFound)
        }
    }

    /// Clears stale entries in the log.
    fn compact(&mut self) -> Result<()> {
        // increase current gen by 2. current_gen + 1 is for the compaction file
        let compaction_gen = self.current_gen + 1;
        self.current_gen += 2;
        self.writer = new_log_file(&self.path, self.current_gen)?;

        let mut compaction_writer = new_log_file(&self.path, compaction_gen)?;

        let mut new_pos = 0; // pos in the new log file
        for entry in self.index.iter() {
            let len = self.reader.read_and(*entry.value(), |mut entry_reader| {
                Ok(io::copy(&mut entry_reader, &mut compaction_writer)?)
            })?;
            self.index.insert(
                entry.key().clone(),
                (compaction_gen, new_pos..new_pos + len).into(),
            );
            new_pos += len;
        }
        compaction_writer.flush()?;

        self.reader
            .safe_point
            .store(compaction_gen, Ordering::SeqCst);
        self.reader.close_stale_handles();

        // remove stale log files
        // Note that actually these files are not deleted immediately because `KvStoreReader`s
        // still keep open file handles. When `KvStoreReader` is used next time, it will clear
        // its stale file handles. On Unix, the files will be deleted after all the handles
        // are closed. On Windows, the deletions below will fail and stale files are expected
        // to be deleted in the next compaction.

        let stale_gens = sorted_gen_list(&self.path)?
            .into_iter()
            .filter(|&gen| gen < compaction_gen);
        for stale_gen in stale_gens {
            let file_path = log_path(&self.path, stale_gen);
            if let Err(e) = fs::remove_file(&file_path) {
                error!("{:?} cannot be deleted: {}", file_path, e);
            }
        }
        self.uncompacted = 0;

        Ok(())
    }
}

/// Create a new log file with given generation number and add the reader to the readers map.
///
/// Returns the writer to the log.
fn new_log_file(path: &Path, gen: u64) -> Result<BufWriterWithPos<File>> {
    let path = log_path(&path, gen);
    let writer = BufWriterWithPos::new(
        OpenOptions::new()
            .create(true)
            .write(true)
            .append(true)
            .open(&path)?,
    )?;
    Ok(writer)
}

/// Returns sorted generation numbers in the given directory
fn sorted_gen_list(path: &Path) -> Result<Vec<u64>> {
    let mut gen_list: Vec<u64> = fs::read_dir(&path)?
        .flat_map(|res| -> Result<_> { Ok(res?.path()) })
        .filter(|path| path.is_file() && path.extension() == Some("log".as_ref()))
        .flat_map(|path| {
            path.file_name()
                .and_then(OsStr::to_str)
                .map(|s| s.trim_end_matches(".log"))
                .map(str::parse::<u64>)
        })
        .flatten()
        .collect();
    gen_list.sort_unstable();
    Ok(gen_list)
}

/// Load the whole log file and store value locations in the index map.
///
/// Returns how many bytes can be saved after a compaction.
fn load(
    gen: u64,
    reader: &mut BufReaderWithPos<File>,
    index: &SkipMap<String, CommandPos>,
) -> Result<u64> {
    // To make sure we read from the beginning of the file
    let mut pos = reader.seek(SeekFrom::Start(0))?;
    let mut stream = Deserializer::from_reader(reader).into_iter::<Command>();
    let mut uncompacted = 0; // number of bytes that can be saved after a compaction
    while let Some(cmd) = stream.next() {
        let new_pos = stream.byte_offset() as u64;
        match cmd? {
            Command::Set { key, .. } => {
                if let Some(old_cmd) = index.get(&key) {
                    uncompacted += old_cmd.value().len;
                }
                index.insert(key, (gen, pos..new_pos).into());
            }
            Command::Remove { key } => {
                if let Some(old_cmd) = index.remove(&key) {
                    uncompacted += old_cmd.value().len;
                }
                // the "remove" command itself can be deleted in the next compaction
                // so we add its length to `uncompacted`
                uncompacted += new_pos - pos;
            }
        }
        pos = new_pos;
    }
    Ok(uncompacted)
}

fn log_path(dir: &Path, gen: u64) -> PathBuf {
    dir.join(format!("{}.log", gen))
}

/// Struct representing a command
#[derive(Serialize, Deserialize, Debug)]
enum Command {
    Set { key: String, value: String },
    Remove { key: String },
}

impl Command {
    fn set(key: String, value: String) -> Command {
        Command::Set { key, value }
    }

    fn remove(key: String) -> Command {
        Command::Remove { key }
    }
}

/// Represents the position and length of a json-serialized command in the log
#[derive(Debug, Clone, Copy)]
struct CommandPos {
    gen: u64,
    pos: u64,
    len: u64,
}

impl From<(u64, Range<u64>)> for CommandPos {
    fn from((gen, range): (u64, Range<u64>)) -> Self {
        CommandPos {
            gen,
            pos: range.start,
            len: range.end - range.start,
        }
    }
}

struct BufReaderWithPos<R: Read + Seek> {
    reader: BufReader<R>,
    pos: u64,
}

impl<R: Read + Seek> BufReaderWithPos<R> {
    fn new(mut inner: R) -> Result<Self> {
        let pos = inner.seek(SeekFrom::Current(0))?;
        Ok(BufReaderWithPos {
            reader: BufReader::new(inner),
            pos,
        })
    }
}

impl<R: Read + Seek> Read for BufReaderWithPos<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let len = self.reader.read(buf)?;
        self.pos += len as u64;
        Ok(len)
    }
}

impl<R: Read + Seek> Seek for BufReaderWithPos<R> {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        self.pos = self.reader.seek(pos)?;
        Ok(self.pos)
    }
}

struct BufWriterWithPos<W: Write + Seek> {
    writer: BufWriter<W>,
    pos: u64,
}

impl<W: Write + Seek> BufWriterWithPos<W> {
    fn new(mut inner: W) -> Result<Self> {
        let pos = inner.seek(SeekFrom::Current(0))?;
        Ok(BufWriterWithPos {
            writer: BufWriter::new(inner),
            pos,
        })
    }
}

impl<W: Write + Seek> Write for BufWriterWithPos<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let len = self.writer.write(buf)?;
        self.pos += len as u64;
        Ok(len)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.writer.flush()
    }
}

impl<W: Write + Seek> Seek for BufWriterWithPos<W> {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        self.pos = self.writer.seek(pos)?;
        Ok(self.pos)
    }
}
