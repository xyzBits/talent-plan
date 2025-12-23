use std::collections::{BTreeMap, HashMap};
use std::fs::{self, File, OpenOptions};
use std::io::{self, BufReader, BufWriter, Read, Seek, SeekFrom, Write};
use std::ops::Range;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Deserializer;

use crate::{KvsError, Result};
use std::ffi::OsStr;

// 日志压缩阈值：1MB
const COMPACTION_THRESHOLD: u64 = 1024 * 1024;

/// `KvStore` 存储字符串类型的键值对。
///
/// 键值对被持久化到磁盘上的日志文件中。日志文件以单调递增的代数 (generation number) 命名，
/// 扩展名为 `.log`。内存中的 `BTreeMap` 存储键及其在磁盘上的位置，以便快速查询。
///
/// ```rust
/// # use kvs::{KvStore, Result};
/// # fn try_main() -> Result<()> {
/// use std::env::current_dir;
/// let mut store = KvStore::open(current_dir()?)?;
/// store.set("key".to_owned(), "value".to_owned())?;
/// let val = store.get("key".to_owned())?;
/// assert_eq!(val, Some("value".to_owned()));
/// # Ok(())
/// # }
/// ```
pub struct KvStore {
    // 日志和其他数据所在的目录。// 要创建路径或者修改路径，就用 PathBuf，有缓冲区，可以变更
    path: PathBuf,
    // 将代数映射到文件读取器。
    readers: HashMap<u64, BufReaderWithPos<File>>,
    // 当前日志文件的写入器。
    writer: BufWriterWithPos<File>, // 日志压缩时，会修改这个
    // 当前正在写入的日志代数。
    current_gen: u64,
    // 内存索引：键 -> 命令在日志中的位置。
    index: BTreeMap<String, CommandPos>,
    // 未压缩的字节数，即可以通过压缩删除的“陈旧”命令所占用的字节数。
    uncompacted: u64,
}

impl KvStore {
    /// 在给定路径下打开一个 `KvStore`。
    ///
    /// 如果路径不存在，将创建一个新目录。
    ///
    /// # Errors
    ///
    /// 可能会传播日志重放过程中的 I/O 或反序列化错误。
    /// 这段代码体现了 Log-Structured Storage (LSM-Tree 家族/Bitcask) 的核心哲学：
    ///
    /// 持久化是第一位的：所有数据都在磁盘日志里。
    ///
    /// 内存索引是易失的：内存里的 index 只是磁盘日志的一个“缓存视图”。
    ///
    /// 重启即重放：每次启动，都要通过“重看一遍录像（日志）”来找回当前的状态
    pub fn open(path: impl Into<PathBuf>) -> Result<KvStore> {
        // 创建目录
        let path = path.into();
        fs::create_dir_all(&path)?;

        // readers 缓存所有打开的文件句柄，防止每次读数据都要重新 open 文件
        let mut readers = HashMap::new();

        // key -> (file_id, offset, length) 需要有序
        let mut index = BTreeMap::new();

        // 找出所有的 1.log 2.log 3.log 100.log这样文件，取出数字，并排序返回
        // 顺序极其重要，必须按照时间顺序重放日志，才能保证后面的覆盖前面的
        let gen_list = sorted_gen_list(&path)?;
        let mut uncompacted = 0;

        for &gen in &gen_list {
            // 遍历所有日志文件
            let mut reader = BufReaderWithPos::new(File::open(log_path(&path, gen))?)?;

            // 从头到尾读取文件中的每一条 command，如果是 set 在index 中更新k的位置，如果k 已经存在，说明旧位置的数据变成的垃圾
            uncompacted += load(gen, &mut reader, &mut index)?;
            readers.insert(gen, reader);
        }

        let current_gen = gen_list.last().unwrap_or(&0) + 1;
        // 旧的日志文件 readonly，最新的文件可写
        let writer = new_log_file(&path, current_gen, &mut readers)?;

        Ok(KvStore {
            path,
            readers,
            writer,
            current_gen,
            index,
            uncompacted,
        })
    }

    /// 将字符串键的值设置为字符串。
    ///
    /// 如果键已存在，旧值将被覆盖。
    ///
    /// # Errors
    ///
    /// 可能会传播写入日志过程中的 I/O 或序列化错误。
    /// 先写日志（disk），后更新索引 （memory），并附带了垃圾回收的触发机制
    ///
    /// 1. 顺序写 sequential write 只追加写入，极大提升写入性能，
    /// 2. 内存索引 hashmap indexing， 通过 index.insert 维护最新的 key 位置，保证读取速度是 O(1) 的
    /// 3. 空间换时间与惰性删除，更新数据时不原地修改，而是追加数据，旧数据变成垃圾
    /// 4. 后台压缩，通过 uncompacted 计数器监控垃圾量，适时清理，
    pub fn set(&mut self, key: String, value: String) -> Result<()> {
        // Log-Structured 的核心，所有的操作(包括删除)在磁盘上都表现为一条“日志记录”，这里创建了 一个 Set 类型的指令对象
        let cmd = Command::set(key, value);

        // 封装了 BufWriter pos 记录了当前文件写到了第几个字节 offset
        // 我们需要知道这条数据是从文件哪个位置开始写的
        let pos = self.writer.pos;

        // 将 cmd 对象序列化为 json 格式，并直接写入 write 缓冲区
        serde_json::to_writer(&mut self.writer, &cmd)?;

        // 缓冲区数据强制刷入 disk，保证了数据的持久性，如果此时掉电，数据不应该丢失
        // log 中只增加数据，不修改老数据
        self.writer.flush()?;
        if let Command::Set { key, .. } = cmd {
            if let Some(old_cmd) = self
                .index // index 记录数据在 disk 上的位置
                .insert(key, (self.current_gen, pos..self.writer.pos).into())
            // 如果key 存在，返回旧值，否则 返回 None
            {
                self.uncompacted += old_cmd.len;
            }
        }

        if self.uncompacted > COMPACTION_THRESHOLD {
            self.compact()?;
        }
        Ok(())
    }

    /// 获取给定字符串键的字符串值。
    ///
    /// 如果键不存在，返回 `None`。
    ///
    /// # Errors
    ///
    /// 如果给定的命令类型不符合预期，返回 `KvsError::UnexpectedCommandType`。
    /// 内存查索引 ，disk 读数据
    /// 拿着 key 去内存 hashMap 查这个 key 在磁盘文件 哪个位置offset ，多长，然后指哪打哪，直接去把那一段磁盘读出来
    pub fn get(&mut self, key: String) -> Result<Option<String>> {
        if let Some(cmd_pos) = self.index.get(&key) {
            // 内存索引查找 key -> CommandPos gen 文件号 pos 起始位置 len 多长
            let reader = self
                .readers // 缴存  所有文件 句柄
                .get_mut(&cmd_pos.gen) // 获取可变引用，因为要对reader seek
                .expect("Cannot find log reader");

            // 磁盘定位 Seeking 核心 IO 操作，告诉 OS，直接将磁头移动到 pos 这个字节的位置
            reader.seek(SeekFrom::Start(cmd_pos.pos))?;

            // 限制读取长度，读完 len，就会遇到 EOF
            let cmd_reader = reader.take(cmd_pos.len);

            if let Command::Set { value, .. } = serde_json::from_reader(cmd_reader)? {
                Ok(Some(value))
            } else {
                Err(KvsError::UnexpectedCommandType)
            }
        } else {
            // 索引里没这个 key，直接返回
            Ok(None)
        }
    }

    /// 移除给定的键。
    ///
    /// # Errors
    ///
    /// 如果找不到给定的键，返回 `KvsError::KeyNotFound`。
    /// 可能会传播写入日志过程中的 I/O 或序列化错误。
    pub fn remove(&mut self, key: String) -> Result<()> {
        if self.index.contains_key(&key) {
            let cmd = Command::remove(key);
            serde_json::to_writer(&mut self.writer, &cmd)?;
            self.writer.flush()?;
            if let Command::Remove { key } = cmd {
                let old_cmd = self.index.remove(&key).expect("key not found");
                self.uncompacted += old_cmd.len;
            }
            Ok(())
        } else {
            Err(KvsError::KeyNotFound)
        }
    }

    /// 清除日志中的过时条目。
    /// 日志压缩，也就是垃圾回收
    /// 把散落在多个旧日志文件中的有效数据找出来，合并到新的文件中，然后把旧文件全部删除掉，从而释放 disk space
    /// 搬家，需要的东西打包带到新家，剩下的垃圾，留在旧房子，然后把房子拆了
    pub fn compact(&mut self) -> Result<()> {
        // 将当前代数增加 2。current_gen + 1 用于压缩后的新文件。
        // 1。准备压缩专用文件的代号 id = N + 1
        let compaction_gen = self.current_gen + 1;

        // 2。 准备未来写入文件的代号 id = N + 2
        self.current_gen += 2;

        // 3. 将当前的 writer 立即指向 N + 2
        // 从这里起，所有的 set remove 操作会写入 N + 2.log
        // 不阻塞新的写入，如果有新的写入 set 请求进来，直接写到 N+2
        self.writer = self.new_log_file(self.current_gen)?;

        // 4。 创建一个新的 writer 专门用于写压缩后的数据 N + 1.log
        let mut compaction_writer = self.new_log_file(compaction_gen)?;

        // 记录 N+1.log 中写到哪个位置
        let mut new_pos = 0; // 新日志文件中的位置。

        // 遍历内存中的所有索引
        // 索引里存的一定是最新的，有效的数据
        // 已经被删除或者覆盖的数据根本不在index里面，自然不会搬运
        // 如果你有100GB的日志文件，由于反复的修改，只有1GB的有效数据，内存index中只有这1GB的key
        // loop 只会执行这1GB数据的IO操作，其他99GB的垃圾看都不看一眼，直接跳过
        for cmd_pos in &mut self.index.values_mut() {
            // 找到数据条目在哪个旧文件
            let reader = self
                .readers
                .get_mut(&cmd_pos.gen)
                .expect("Cannot find log reader");

            // 移动 磁头 到旧位置
            if reader.pos != cmd_pos.pos {
                reader.seek(SeekFrom::Start(cmd_pos.pos))?;
            }

            // 只读取这一段，读取一条数据
            let mut entry_reader = reader.take(cmd_pos.len);

            // 直接把数据从旧文件 copy 到 新文件 N+1.log
            // io::copy 非常快，使用流式传输
            let len = io::copy(&mut entry_reader, &mut compaction_writer)?;

            // 原地修改内存索引 ，把key 指向的位置，从旧文件的位置，更新为 压缩文件 N+1.log 的新搁置
            *cmd_pos = (compaction_gen, new_pos..new_pos + len).into();
            new_pos += len;
        }
        compaction_writer.flush()?;

        // 移除旧的日志文件。
        // 找出所有 id < N + 1 的文件
        let stale_gens: Vec<_> = self
            .readers
            .keys()
            .filter(|&&gen| gen < compaction_gen)
            .cloned()
            .collect();

        // 遍历删除
        for stale_gen in stale_gens {
            // 从内存的 readers 缓存中删除
            self.readers.remove(&stale_gen);

            // 从 disk 物理删除文件
            fs::remove_file(log_path(&self.path, stale_gen))?;
        }
        self.uncompacted = 0;

        Ok(())
    }

    /// 使用给定的代数创建一个新的日志文件，并将读取器添加到 readers 映射中。
    ///
    /// 返回该日志文件的写入器。
    fn new_log_file(&mut self, gen: u64) -> Result<BufWriterWithPos<File>> {
        new_log_file(&self.path, gen, &mut self.readers)
    }
}

/// 使用给定的代数创建一个新的日志文件，并将读取器添加到 readers 映射中。
///
/// 返回该日志文件的写入器。
fn new_log_file(
    path: &Path,
    gen: u64,
    readers: &mut HashMap<u64, BufReaderWithPos<File>>,
) -> Result<BufWriterWithPos<File>> {
    let path = log_path(&path, gen);
    let writer = BufWriterWithPos::new(
        OpenOptions::new()
            .create(true)
            .write(true)
            .append(true)
            .open(&path)?,
    )?;
    readers.insert(gen, BufReaderWithPos::new(File::open(&path)?)?);
    Ok(writer)
}

/// 返回给定目录下排序后的代数列表。
fn sorted_gen_list(path: &Path) -> Result<Vec<u64>> {
    let mut gen_list: Vec<u64> = fs::read_dir(&path)?
        // flat_map 将 DirEntry -> PathBuf，如果有错误，直接丢弃了
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

/// 加载整个日志文件并将值的位置存储在索引映射中。
///
/// 返回压缩后可以节省的字节数。
/// 存储引擎在启动时的 重放 逻辑
/// 扫描一个日志文件，将里的有效数据加载到内存索引  BTreeMap中，顺便计算出文件中有多少垃圾数据
fn load(
    gen: u64,                                 // 当前正在处理的日志文件，如 1.log
    reader: &mut BufReaderWithPos<File>,      // 文件读取器
    index: &mut BTreeMap<String, CommandPos>, // 全局内存索引， 要修改它
) -> Result<u64> {
    // 返回有多少字节是垃圾
    // 确保从文件开头开始读取。这个reader可能之前被用过，或者我们想从头开始扫描
    let mut pos = reader.seek(SeekFrom::Start(0))?;

    // 创建流式迭代器，不会一次性把几个GB的文件读到内存，而是每次只读一条JSON 命令
    let mut stream = Deserializer::from_reader(reader).into_iter::<Command>();

    let mut uncompacted = 0; // 压缩后可以节省的字节数。

    // serde_json 知道 json 的语法，所以能精确的读出一个 json
    while let Some(cmd) = stream.next() {
        // 一第一第解析 command
        // 获取当前解析完的位置
        let new_pos = stream.byte_offset() as u64;
        match cmd? {
            Command::Set { key, .. } => {
                if let Some(old_cmd) = index.insert(key, (gen, pos..new_pos).into()) {
                    // 返回 old_cmd 证明这个 key 之前已经存在了，被更新了，旧值就是垃圾
                    uncompacted += old_cmd.len;
                }
            }
            Command::Remove { key } => {
                if let Some(old_cmd) = index.remove(&key) {
                    uncompacted += old_cmd.len;
                }
                // “移除”命令本身也可以在下次压缩中删除，
                // 所以我们将其长度也计入 `uncompacted`。
                uncompacted += new_pos - pos;
            }
        }
        pos = new_pos; // 更新起始位置，为下一轮做准备 [pos..new_pos] 这条 json 数据在磁盘上的物理区间
    }
    Ok(uncompacted)
}

fn log_path(dir: &Path, gen: u64) -> PathBuf {
    dir.join(format!("{}.log", gen))
}

/// 表示一条命令的结构体。
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

/// 表示日志中 json 序列化后的命令的位置和长度。
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

/// 带有当前位置记录的 BufReader。
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

/// 带有当前位置记录的 BufWriter。
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
        // 每次写完后都会记录最新的位置
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
