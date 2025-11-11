use std::path::{Path, PathBuf};
use std::{
    fmt, fs,
    io::{self, Read},
    mem,
};
use typst_library::foundations::{NativeRuleMap, StyleChain};

use comemo::{Track, Tracked};
use std::sync::{LazyLock, OnceLock};

use clap::builder::ValueParser;
use clap::{ArgAction, Args, Parser, ValueEnum};

use typst_library::World;
use typst_library::engine::{Engine, Route, Sink, Traced};
use typst_library::routines::Routines;
use typst_timing::timed;

use typst::LibraryExt;

use rustc_hash::FxHashMap;

use parking_lot::Mutex;
use typst_kit::fonts::{FontSlot, Fonts};
use typst_library::text::{Font, FontBook};

use chrono::offset::{FixedOffset, Local};

use typst::diag::{FileError, FileResult};

use typst::foundations::{Bytes, Dict, IntoValue, TargetElem};
use typst_library::introspection::Introspector;

use std::io::Write;

use chrono::{DateTime, Datelike, Timelike, Utc};
use ecow::eco_format;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use typst::diag::{At, HintedString, SourceResult, StrResult, Warned, bail};
use typst::foundations::{Datetime, Smart};
use typst::layout::{Frame, Page, PageRanges, PagedDocument};
use typst::syntax::{FileId, Lines, Source, Span, VirtualPath};
use typst_html::HtmlDocument;
use typst_library::{Library, model::DocumentInfo};
use typst_pdf::{PdfOptions, PdfStandards, Timestamp};

use typst_utils::{LazyHash, hash128};

mod gui;
pub fn main() -> StrResult<()> {
    fn help() {
        println!("Usage: typst (compile|render) <input_file>");
    }
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        help();
        return Ok(());
    }
    match args[1].as_str() {
        "compile" => {
            Renderer::new()
                .render_from_path_to_pdf(&"main.typ".into())
                .unwrap();
            Ok(())
        }
        "image" => {
            Renderer::new()
                .render_from_path_to_image(&"main.typ".into())
                .unwrap();
            Ok(())
        }
        "html" => {
            Renderer::new()
                .render_from_path_to_html(&"main.typ".into())
                .unwrap();
            Ok(())
        }
        "render" => render(),
        _ => {
            help();
            Ok(())
        }
    }
}

struct Renderer {
    world: SystemWorld,
}

impl Renderer {
    fn new() -> Self {
        let font_args: FontArgs = FontArgs {
            font_paths: Vec::new(),
            ignore_system_fonts: false,
        };
        let package = PackageArgs::default();
        let world_args = WorldArgs {
            root: None,
            inputs: Vec::new(),
            font: font_args,
            package: package,
            creation_timestamp: None,
        };
        let process_args = ProcessArgs {
            jobs: None,
            features: Vec::new(),
        };
        Self {
            world: SystemWorld::new(&Input::Stdin, &world_args, &process_args).unwrap(),
        }
    }
    fn render_from_path(&mut self, path: &PathBuf) -> Frame {
        println!("render_from_path");
        self.world.main = FileId::new(None, VirtualPath::new(path));
        let Warned { output, .. } = compile::<PagedDocument>(&mut self.world);
        let doc: PagedDocument = output.unwrap();
        doc.pages.into_iter().next().unwrap().frame
    }
    fn compile_config(output_path: PathBuf, output_format: OutputFormat) -> CompileConfig {
        let output = Output::Path(output_path);
        let pdf_standards = PdfStandards::default();
        let deps_format = DepsFormat::default();
        let config = CompileConfig {
            warnings: Vec::new(),
            watching: false,
            input: Input::Stdin,
            output,
            output_format,
            pages: None,
            creation_timestamp: None,
            open: None,
            pdf_standards,
            tagged: false,
            deps: None,
            deps_format,
            ppi: 120.0,
        };
        config
    }
    fn render_from_path_to_image(&mut self, path: &PathBuf) -> SourceResult<()> {
        self.world.main = FileId::new(None, VirtualPath::new(path));
        let Warned { output, .. } = compile::<PagedDocument>(&mut self.world);
        let doc: PagedDocument = output.unwrap();
        let config = Self::compile_config("main.png".into(), OutputFormat::Png);
        export_paged(&doc, &config)?;
        Ok(())
    }
    fn render_from_path_to_html(&mut self, path: &PathBuf) -> SourceResult<()> {
        self.world.main = FileId::new(None, VirtualPath::new(path));
        let Warned { output, .. } = compile::<HtmlDocument>(&mut self.world);
        let doc = output.unwrap();
        let config = Self::compile_config("main.html".into(), OutputFormat::Html);
        export_html(&doc, &config)
    }
    fn render_from_path_to_pdf(&mut self, path: &PathBuf) -> SourceResult<()> {
        self.world.main = FileId::new(None, VirtualPath::new(path));
        let Warned { output, .. } = compile::<PagedDocument>(&mut self.world);
        let doc: PagedDocument = output.unwrap();
        let config = Self::compile_config("main.pdf".into(), OutputFormat::Pdf);
        export_paged(&doc, &config)?;
        Ok(())
    }
    fn render_from_string(&mut self, data: String) -> Frame {
        let file = FileId::new(None, VirtualPath::new(&PathBuf::new()));
        let fingerprint = hash128(data.as_bytes());
        let source = Source::new(file, data);
        let slot = FileSlot {
            id: file,
            source: SlotCell {
                data: Some(Ok(source)),
                fingerprint,
                accessed: true,
            },
            file: SlotCell {
                data: None,
                fingerprint,
                accessed: true,
            },
        };
        self.world.slots.lock().insert(file, slot);
        self.world.main = file;
        // self.world.source = Some(data);
        let Warned { output, .. } = compile::<PagedDocument>(&mut self.world);
        let doc: PagedDocument = output.unwrap();
        doc.pages.into_iter().next().unwrap().frame
    }
}
pub fn compile<D>(world: &dyn World) -> Warned<SourceResult<D>>
where
    D: Document,
{
    let mut sink = Sink::new();
    let output = compile_impl::<D>(world.track(), Traced::default().track(), &mut sink);
    Warned {
        output,
        warnings: sink.warnings(),
    }
}

fn render() -> StrResult<()> {
    let path = Some(PathBuf::from("main.typ"));

    gui::run(path, Renderer::new());

    Ok(())
}

/// A world that provides access to the operating system.
pub struct SystemWorld {
    /// The working directory.
    workdir: Option<PathBuf>,
    /// The root relative to which absolute paths are resolved.
    root: PathBuf,
    /// The input path.
    main: FileId,
    /// Typst's standard library.
    library: LazyHash<Library>,
    /// Metadata about discovered fonts.
    book: LazyHash<FontBook>,
    /// Locations of and storage for lazily loaded fonts.
    fonts: Vec<FontSlot>,
    /// Maps file ids to source files and buffers.
    slots: Mutex<FxHashMap<FileId, FileSlot>>,
    /// The current datetime if requested. This is stored here to ensure it is
    /// always the same within one compilation.
    /// Reset between compilations if not [`Now::Fixed`].
    now: Now,
}

/// The current date and time.
enum Now {
    /// The date and time if the environment `SOURCE_DATE_EPOCH` is set.
    /// Used for reproducible builds.
    Fixed(DateTime<Utc>),
    /// The current date and time if the time is not externally fixed.
    System(OnceLock<DateTime<Utc>>),
}

/// An error that occurs during world construction.
#[derive(Debug)]
pub enum WorldCreationError {
    /// The input file does not appear to exist.
    InputNotFound(PathBuf),
    /// The input file is not contained within the root folder.
    InputOutsideRoot,
    /// The root directory does not appear to exist.
    RootNotFound(PathBuf),
    /// Another type of I/O error.
    Io(io::Error),
}

impl fmt::Display for WorldCreationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WorldCreationError::InputNotFound(path) => {
                write!(f, "input file not found (searched at {})", path.display())
            }
            WorldCreationError::InputOutsideRoot => {
                write!(f, "source file must be contained in project root")
            }
            WorldCreationError::RootNotFound(path) => {
                write!(
                    f,
                    "root directory not found (searched at {})",
                    path.display()
                )
            }
            WorldCreationError::Io(err) => write!(f, "{err}"),
        }
    }
}

/// Arguments related to where packages are stored in the system.
#[derive(Debug, Clone, Args, Default)]
pub struct PackageArgs {
    /// Custom path to local packages, defaults to system-dependent location.
    #[clap(long = "package-path", env = "TYPST_PACKAGE_PATH", value_name = "DIR")]
    pub package_path: Option<PathBuf>,

    /// Custom path to package cache, defaults to system-dependent location.
    #[clap(
        long = "package-cache-path",
        env = "TYPST_PACKAGE_CACHE_PATH",
        value_name = "DIR"
    )]
    pub package_cache_path: Option<PathBuf>,
}

const ENV_PATH_SEP: char = if cfg!(windows) { ';' } else { ':' };

/// Common arguments to customize available fonts.
#[derive(Debug, Clone, Parser)]
pub struct FontArgs {
    /// Adds additional directories that are recursively searched for fonts.
    ///
    /// If multiple paths are specified, they are separated by the system's path
    /// separator (`:` on Unix-like systems and `;` on Windows).
    #[clap(
        long = "font-path",
        env = "TYPST_FONT_PATHS",
        value_name = "DIR",
        value_delimiter = ENV_PATH_SEP,
    )]
    pub font_paths: Vec<PathBuf>,

    /// Ensures system fonts won't be searched, unless explicitly included via
    /// `--font-path`.
    #[arg(long, env = "TYPST_IGNORE_SYSTEM_FONTS")]
    pub ignore_system_fonts: bool,
}

/// Arguments for the construction of a world. Shared by compile, watch, and
/// query.
#[derive(Debug, Clone, Args)]
pub struct WorldArgs {
    /// Configures the project root (for absolute paths).
    #[clap(long = "root", env = "TYPST_ROOT", value_name = "DIR")]
    pub root: Option<PathBuf>,

    /// Add a string key-value pair visible through `sys.inputs`.
    #[clap(
        long = "input",
        value_name = "key=value",
        action = ArgAction::Append,
        value_parser = ValueParser::new(parse_sys_input_pair),
    )]
    pub inputs: Vec<(String, String)>,

    /// Common font arguments.
    #[clap(flatten)]
    pub font: FontArgs,

    /// Arguments related to storage of packages in the system.
    #[clap(flatten)]
    pub package: PackageArgs,

    /// The document's creation date formatted as a UNIX timestamp.
    ///
    /// For more information, see <https://reproducible-builds.org/specs/source-date-epoch/>.
    #[clap(
        long = "creation-timestamp",
        env = "SOURCE_DATE_EPOCH",
        value_name = "UNIX_TIMESTAMP",
        value_parser = parse_source_date_epoch,
    )]
    pub creation_timestamp: Option<DateTime<Utc>>,
}

fn parse_sys_input_pair(raw: &str) -> Result<(String, String), String> {
    let (key, val) = raw
        .split_once('=')
        .ok_or("input must be a key and a value separated by an equal sign")?;
    let key = key.trim().to_owned();
    if key.is_empty() {
        return Err("the key was missing or empty".to_owned());
    }
    let val = val.trim().to_owned();
    Ok((key, val))
}

/// Parses a UNIX timestamp according to <https://reproducible-builds.org/specs/source-date-epoch/>
fn parse_source_date_epoch(raw: &str) -> Result<DateTime<Utc>, String> {
    let timestamp: i64 = raw
        .parse()
        .map_err(|err| format!("timestamp must be decimal integer ({err})"))?;
    DateTime::from_timestamp(timestamp, 0).ok_or_else(|| "timestamp out of range".to_string())
}

/// An in-development feature that may be changed or removed at any time.
#[derive(Debug, Copy, Clone, Eq, PartialEq, ValueEnum)]
pub enum Feature {
    Html,
    A11yExtras,
}

/// Arguments for configuration the process of compilation itself.
#[derive(Debug, Clone, Args)]
pub struct ProcessArgs {
    /// Number of parallel jobs spawned during compilation. Defaults to number
    /// of CPUs. Setting it to 1 disables parallelism.
    #[clap(long, short)]
    pub jobs: Option<usize>,
    #[arg(long = "features", value_delimiter = ',', env = "TYPST_FEATURES")]
    pub features: Vec<Feature>,
}

impl SystemWorld {
    /// Create a new system world.
    pub fn new(
        input: &Input,
        world_args: &WorldArgs,
        process_args: &ProcessArgs,
    ) -> Result<Self, WorldCreationError> {
        // Set up the thread pool.
        if let Some(jobs) = process_args.jobs {
            rayon::ThreadPoolBuilder::new()
                .num_threads(jobs)
                .use_current_thread()
                .build_global()
                .ok();
        }

        // Resolve the system-global input path.
        let input = match input {
            Input::Stdin => None,
            Input::Path(path) => Some(path.canonicalize().map_err(|err| match err.kind() {
                io::ErrorKind::NotFound => WorldCreationError::InputNotFound(path.clone()),
                _ => WorldCreationError::Io(err),
            })?),
        };

        // Resolve the system-global root directory.
        let root = {
            let path = world_args
                .root
                .as_deref()
                .or_else(|| input.as_deref().and_then(|i| i.parent()))
                .unwrap_or(Path::new("."));
            path.canonicalize().map_err(|err| match err.kind() {
                io::ErrorKind::NotFound => WorldCreationError::RootNotFound(path.to_path_buf()),
                _ => WorldCreationError::Io(err),
            })?
        };

        let main = if let Some(path) = &input {
            // Resolve the virtual path of the main file within the project root.
            let main_path = VirtualPath::within_root(path, &root)
                .ok_or(WorldCreationError::InputOutsideRoot)?;
            FileId::new(None, main_path)
        } else {
            // Return the special id of STDIN otherwise
            *STDIN_ID
        };

        let library = {
            // Convert the input pairs to a dictionary.
            let inputs: Dict = world_args
                .inputs
                .iter()
                .map(|(k, v)| (k.as_str().into(), v.as_str().into_value()))
                .collect();

            let features = process_args
                .features
                .iter()
                .map(|&feature| match feature {
                    Feature::Html => typst::Feature::Html,
                    Feature::A11yExtras => typst::Feature::A11yExtras,
                })
                .collect();

            Library::builder()
                .with_inputs(inputs)
                .with_features(features)
                .build()
        };

        let mut fonts = Fonts::searcher();
        fonts.include_system_fonts(!world_args.font.ignore_system_fonts);
        fonts.include_embedded_fonts(true);
        let fonts = fonts.search_with(&world_args.font.font_paths);

        let now = match world_args.creation_timestamp {
            Some(time) => Now::Fixed(time),
            None => Now::System(OnceLock::new()),
        };

        Ok(Self {
            workdir: std::env::current_dir().ok(),
            root,
            main,
            library: LazyHash::new(library),
            book: LazyHash::new(fonts.book),
            fonts: fonts.fonts,
            slots: Mutex::new(FxHashMap::default()),
            now,
        })
    }

    /// The id of the main source file.
    pub fn main(&self) -> FileId {
        self.main
    }

    /// The root relative to which absolute paths are resolved.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// The current working directory.
    pub fn workdir(&self) -> &Path {
        self.workdir.as_deref().unwrap_or(Path::new("."))
    }

    /// Return all paths the last compilation depended on.
    pub fn dependencies(&mut self) -> impl Iterator<Item = PathBuf> + '_ {
        self.slots
            .get_mut()
            .values()
            .filter(|slot| slot.accessed())
            .filter_map(|slot| system_path(&self.root, slot.id).ok())
    }

    /// Reset the compilation state in preparation of a new compilation.
    pub fn reset(&mut self) {
        #[allow(clippy::iter_over_hash_type, reason = "order does not matter")]
        for slot in self.slots.get_mut().values_mut() {
            slot.reset();
        }
        if let Now::System(time_lock) = &mut self.now {
            time_lock.take();
        }
    }

    /// Lookup line metadata for a file by id.
    #[track_caller]
    pub fn lookup(&self, id: FileId) -> Lines<String> {
        self.slot(id, |slot| {
            if let Some(source) = slot.source.get() {
                let source = source.as_ref().expect("file is not valid");
                source.lines().clone()
            } else if let Some(bytes) = slot.file.get() {
                let bytes = bytes.as_ref().expect("file is not valid");
                Lines::try_from(bytes).expect("file is not valid utf-8")
            } else {
                panic!("file id does not point to any source file");
            }
        })
    }
}

impl World for SystemWorld {
    fn library(&self) -> &LazyHash<Library> {
        &self.library
    }

    fn book(&self) -> &LazyHash<FontBook> {
        &self.book
    }

    fn main(&self) -> FileId {
        self.main
    }

    fn source(&self, id: FileId) -> FileResult<Source> {
        println!(".source");
        self.slot(id, |slot| slot.source(&self.root))
    }

    fn file(&self, id: FileId) -> FileResult<Bytes> {
        println!(".file");
        self.slot(id, |slot| slot.file(&self.root))
    }

    fn font(&self, index: usize) -> Option<Font> {
        // comemo's validation may invoke this function with an invalid index. This is
        // impossible in typst-cli but possible if a custom tool mutates the fonts.
        self.fonts.get(index)?.get()
    }

    fn today(&self, offset: Option<i64>) -> Option<Datetime> {
        let now = match &self.now {
            Now::Fixed(time) => time,
            Now::System(time) => time.get_or_init(Utc::now),
        };

        // The time with the specified UTC offset, or within the local time zone.
        let with_offset = match offset {
            None => now.with_timezone(&Local).fixed_offset(),
            Some(hours) => {
                let seconds = i32::try_from(hours).ok()?.checked_mul(3600)?;
                now.with_timezone(&FixedOffset::east_opt(seconds)?)
            }
        };

        Datetime::from_ymd(
            with_offset.year(),
            with_offset.month().try_into().ok()?,
            with_offset.day().try_into().ok()?,
        )
    }
}

impl SystemWorld {
    /// Access the canonical slot for the given file id.
    fn slot<F, T>(&self, id: FileId, f: F) -> T
    where
        F: FnOnce(&mut FileSlot) -> T,
    {
        let mut map = self.slots.lock();
        f(map.entry(id).or_insert_with(|| FileSlot::new(id)))
    }
}

/// Holds the processed data for a file ID.
///
/// Both fields can be populated if the file is both imported and read().
struct FileSlot {
    /// The slot's file id.
    id: FileId,
    /// The lazily loaded and incrementally updated source file.
    source: SlotCell<Source>,
    /// The lazily loaded raw byte buffer.
    file: SlotCell<Bytes>,
}

impl FileSlot {
    /// Create a new file slot.
    fn new(id: FileId) -> Self {
        Self {
            id,
            file: SlotCell::new(),
            source: SlotCell::new(),
        }
    }

    /// Whether the file was accessed in the ongoing compilation.
    fn accessed(&self) -> bool {
        self.source.accessed() || self.file.accessed()
    }

    /// Marks the file as not yet accessed in preparation of the next
    /// compilation.
    fn reset(&mut self) {
        self.source.reset();
        self.file.reset();
    }

    /// Retrieve the source for this file.
    fn source(&mut self, project_root: &Path) -> FileResult<Source> {
        println!("sourcing {project_root:?}");
        self.source.get_or_init(
            || read(self.id, project_root),
            |data, prev| {
                let text = decode_utf8(&data)?;
                if let Some(mut prev) = prev {
                    prev.replace(text);
                    Ok(prev)
                } else {
                    Ok(Source::new(self.id, text.into()))
                }
            },
        )
    }

    /// Retrieve the file's bytes.
    fn file(&mut self, project_root: &Path) -> FileResult<Bytes> {
        self.file.get_or_init(
            || read(self.id, project_root),
            |data, _| Ok(Bytes::new(data)),
        )
    }
}

/// Decode UTF-8 with an optional BOM.
fn decode_utf8(buf: &[u8]) -> FileResult<&str> {
    // Remove UTF-8 BOM.
    Ok(std::str::from_utf8(
        buf.strip_prefix(b"\xef\xbb\xbf").unwrap_or(buf),
    )?)
}

/// Reads a file from a `FileId`.
///
/// If the ID represents stdin it will read from standard input,
/// otherwise it gets the file path of the ID and reads the file from disk.
fn read(id: FileId, project_root: &Path) -> FileResult<Vec<u8>> {
    if id == *STDIN_ID {
        read_from_stdin()
    } else {
        read_from_disk(&system_path(project_root, id)?)
    }
}

/// Resolves the path of a file id on the system, downloading a package if
/// necessary.
fn system_path(project_root: &Path, id: FileId) -> FileResult<PathBuf> {
    // Join the path to the root. If it tries to escape, deny
    // access. Note: It can still escape via symlinks.
    id.vpath()
        .resolve(project_root)
        .ok_or(FileError::AccessDenied)
}

/// Read a file from disk.
fn read_from_disk(path: &Path) -> FileResult<Vec<u8>> {
    let f = |e| FileError::from_io(e, path);
    if fs::metadata(path).map_err(f)?.is_dir() {
        Err(FileError::IsDirectory)
    } else {
        fs::read(path).map_err(f)
    }
}

/// Read from stdin.
fn read_from_stdin() -> FileResult<Vec<u8>> {
    let mut buf = Vec::new();
    let result = io::stdin().read_to_end(&mut buf);
    match result {
        Ok(_) => (),
        Err(err) if err.kind() == io::ErrorKind::BrokenPipe => (),
        Err(err) => return Err(FileError::from_io(err, Path::new("<stdin>"))),
    }
    Ok(buf)
}

static STDIN_ID: LazyLock<FileId> = LazyLock::new(|| FileId::new_fake(VirtualPath::new("<stdin>")));

/// Lazily processes data for a file.
struct SlotCell<T> {
    /// The processed data.
    data: Option<FileResult<T>>,
    /// A hash of the raw file contents / access error.
    fingerprint: u128,
    /// Whether the slot has been accessed in the current compilation.
    accessed: bool,
}

impl<T: Clone> SlotCell<T> {
    /// Creates a new, empty cell.
    fn new() -> Self {
        Self {
            data: None,
            fingerprint: 0,
            accessed: false,
        }
    }

    /// Whether the cell was accessed in the ongoing compilation.
    fn accessed(&self) -> bool {
        self.accessed
    }

    /// Marks the cell as not yet accessed in preparation of the next
    /// compilation.
    fn reset(&mut self) {
        self.accessed = false;
    }

    /// Gets the contents of the cell.
    fn get(&self) -> Option<&FileResult<T>> {
        self.data.as_ref()
    }

    /// Gets the contents of the cell or initialize them.
    fn get_or_init(
        &mut self,
        load: impl FnOnce() -> FileResult<Vec<u8>>,
        f: impl FnOnce(Vec<u8>, Option<T>) -> FileResult<T>,
    ) -> FileResult<T> {
        // If we accessed the file already in this compilation, retrieve it.
        if mem::replace(&mut self.accessed, true)
            && let Some(data) = &self.data
        {
            println!("return data");
            return data.clone();
        }

        // Read and hash the file.
        let result = timed!("loading file", load());
        println!("slot taken result {result:?}");
        let fingerprint = timed!("hashing file", typst::utils::hash128(&result));

        // If the file contents didn't change, yield the old processed data.
        if mem::replace(&mut self.fingerprint, fingerprint) == fingerprint
            && let Some(data) = &self.data
        {
            return data.clone();
        }

        let prev = self.data.take().and_then(Result::ok);
        let value = result.and_then(|data| f(data, prev));
        self.data = Some(value.clone());

        value
    }
}

mod sealed {
    use typst_library::foundations::{Content, Target};

    use super::*;

    pub trait Sealed: Sized {
        const TARGET: Target;

        fn create(engine: &mut Engine, content: &Content, styles: StyleChain)
        -> SourceResult<Self>;
    }

    impl Sealed for PagedDocument {
        const TARGET: Target = Target::Paged;

        fn create(
            engine: &mut Engine,
            content: &Content,
            styles: StyleChain,
        ) -> SourceResult<Self> {
            typst_layout::layout_document(engine, content, styles)
        }
    }

    impl Sealed for HtmlDocument {
        const TARGET: Target = Target::Html;

        fn create(
            engine: &mut Engine,
            content: &Content,
            styles: StyleChain,
        ) -> SourceResult<Self> {
            typst_html::html_document(engine, content, styles)
        }
    }
}

/// A document is what results from compilation.
pub trait Document: sealed::Sealed {
    /// Get the document's metadata.
    fn info(&self) -> &DocumentInfo;

    /// Get the document's introspector.
    fn introspector(&self) -> &Introspector;
}

impl Document for PagedDocument {
    fn info(&self) -> &DocumentInfo {
        &self.info
    }

    fn introspector(&self) -> &Introspector {
        &self.introspector
    }
}

impl Document for HtmlDocument {
    fn info(&self) -> &DocumentInfo {
        &self.info
    }

    fn introspector(&self) -> &Introspector {
        &self.introspector
    }
}

fn compile_impl<D: Document>(
    world: Tracked<dyn World + '_>,
    traced: Tracked<Traced>,
    sink: &mut Sink,
) -> SourceResult<D> {
    let library = world.library();
    let base = StyleChain::new(&library.styles);
    let target = TargetElem::target.set(D::TARGET).wrap();
    let styles = base.chain(&target);
    let empty_introspector = Introspector::default();

    // Fetch the main source file once.
    let main = world.main();
    let main = world.source(main).unwrap();

    // First evaluate the main source file into a module.
    let content = typst_eval::eval(
        &ROUTINES,
        world,
        traced,
        sink.track_mut(),
        Route::default().track(),
        &main,
    )?
    .content();

    let mut subsink;
    let introspector = &empty_introspector;

    subsink = Sink::new();

    let constraint = comemo::Constraint::new();
    let mut engine = Engine {
        world,
        introspector: introspector.track_with(&constraint),
        traced,
        sink: subsink.track_mut(),
        route: Route::default(),
        routines: &ROUTINES,
    };

    // Layout!
    let document = D::create(&mut engine, &content, styles)?;

    sink.extend_from_sink(subsink);

    // Promote delayed errors.
    let delayed = sink.delayed();
    if !delayed.is_empty() {
        return Err(delayed);
    }

    Ok(document)
}

pub static ROUTINES: LazyLock<Routines> = LazyLock::new(|| Routines {
    rules: {
        let mut rules = NativeRuleMap::new();
        typst_layout::register(&mut rules);
        typst_html::register(&mut rules);
        rules
    },
    eval_string: typst_eval::eval_string,
    eval_closure: typst_eval::eval_closure,
    realize: typst_realize::realize,
    layout_frame: typst_layout::layout_frame,
    html_module: typst_html::module,
    html_span_filled: typst_html::html_span_filled,
});

#[derive(Debug, Clone)]
pub enum Input {
    /// Stdin, represented by `-`.
    Stdin,
    /// A non-empty path.
    Path(PathBuf),
}

#[derive(Debug, Clone)]
pub enum Output {
    /// Stdout, represented by `-`.
    Stdout,
    /// A non-empty path.
    Path(PathBuf),
}

/// A step-by-step writable version of [`Output`].
#[derive(Debug)]
pub enum OpenOutput<'a> {
    Stdout(std::io::StdoutLock<'a>),
    File(std::fs::File),
}

impl Output {
    /// Write data to the output.
    pub fn write(&self, buffer: &[u8]) -> std::io::Result<()> {
        match self {
            Output::Stdout => std::io::stdout().write_all(buffer),
            Output::Path(path) => std::fs::write(path, buffer),
        }
    }

    /// Open the output for writing.
    pub fn open(&self) -> std::io::Result<OpenOutput<'_>> {
        match self {
            Self::Stdout => Ok(OpenOutput::Stdout(std::io::stdout().lock())),
            Self::Path(path) => std::fs::File::create(path).map(OpenOutput::File),
        }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub enum OutputFormat {
    Pdf,
    Png,
    Svg,
    Html,
}

#[derive(Debug, Default, Copy, Clone, Eq, PartialEq)]
pub enum DepsFormat {
    /// Encodes as JSON, failing for non-Unicode paths.
    #[default]
    Json,
    /// Separates paths with NULL bytes and can express all paths.
    Zero,
    /// Emits in Make format, omitting inexpressible paths.
    Make,
}

/// A preprocessed `CompileCommand`.
pub struct CompileConfig {
    /// Static warnings to emit after compilation.
    pub warnings: Vec<HintedString>,
    /// Whether we are watching.
    pub watching: bool,
    /// Path to input Typst file or stdin.
    pub input: Input,
    /// Path to output file (PDF, PNG, SVG, or HTML).
    pub output: Output,
    /// The format of the output file.
    pub output_format: OutputFormat,
    /// Which pages to export.
    pub pages: Option<PageRanges>,
    /// The document's creation date formatted as a UNIX timestamp, with UTC suffix.
    pub creation_timestamp: Option<DateTime<Utc>>,
    /// Opens the output file with the default viewer or a specific program after
    /// compilation.
    pub open: Option<Option<String>>,
    /// A list of standards the PDF should conform to.
    pub pdf_standards: PdfStandards,
    /// Whether to write PDF (accessibility) tags.
    pub tagged: bool,
    /// A destination to write a list of dependencies to.
    pub deps: Option<Output>,
    /// The format to use for dependencies.
    pub deps_format: DepsFormat,
    /// The PPI (pixels per inch) to use for PNG export.
    pub ppi: f32,
}

/// Export to HTML.
fn export_html(document: &HtmlDocument, config: &CompileConfig) -> SourceResult<()> {
    let html = typst_html::html(document)?;
    let result = config.output.write(html.as_bytes());

    result
        .map_err(|err| eco_format!("failed to write HTML file ({err})"))
        .at(Span::detached())
}

/// Export to one or multiple images.
fn export_image(
    document: &PagedDocument,
    config: &CompileConfig,
    fmt: ImageExportFormat,
) -> StrResult<Vec<Output>> {
    // Determine whether we have indexable templates in output
    let can_handle_multiple = match config.output {
        Output::Stdout => false,
        Output::Path(ref output) => {
            output_template::has_indexable_template(output.to_str().unwrap_or_default())
        }
    };

    let exported_pages = document
        .pages
        .iter()
        .enumerate()
        .filter(|(i, _)| {
            config
                .pages
                .as_ref()
                .is_none_or(|exported_page_ranges| exported_page_ranges.includes_page_index(*i))
        })
        .collect::<Vec<_>>();

    if !can_handle_multiple && exported_pages.len() > 1 {
        let err = match config.output {
            Output::Stdout => "to stdout",
            Output::Path(_) => "without a page number template ({p}, {0p}) in the output path",
        };
        bail!("cannot export multiple images {err}");
    }

    // The results are collected in a `Vec<()>` which does not allocate.
    exported_pages
        .par_iter()
        .map(|(i, page)| {
            // Use output with converted path.
            let output = match &config.output {
                Output::Path(path) => {
                    let storage;
                    let path = if can_handle_multiple {
                        storage = output_template::format(
                            path.to_str().unwrap_or_default(),
                            i + 1,
                            document.pages.len(),
                        );
                        Path::new(&storage)
                    } else {
                        path
                    };

                    Output::Path(path.to_owned())
                }
                Output::Stdout => Output::Stdout,
            };

            export_image_page(config, page, &output, fmt)?;
            Ok(output)
        })
        .collect::<StrResult<Vec<Output>>>()
}

mod output_template {
    const INDEXABLE: [&str; 3] = ["{p}", "{0p}", "{n}"];

    pub fn has_indexable_template(output: &str) -> bool {
        INDEXABLE.iter().any(|template| output.contains(template))
    }

    pub fn format(output: &str, this_page: usize, total_pages: usize) -> String {
        // Find the base 10 width of number `i`
        fn width(i: usize) -> usize {
            1 + i.checked_ilog10().unwrap_or(0) as usize
        }

        let other_templates = ["{t}"];
        INDEXABLE
            .iter()
            .chain(other_templates.iter())
            .fold(output.to_string(), |out, template| {
                let replacement = match *template {
                    "{p}" => format!("{this_page}"),
                    "{0p}" | "{n}" => format!("{:01$}", this_page, width(total_pages)),
                    "{t}" => format!("{total_pages}"),
                    _ => unreachable!("unhandled template placeholder {template}"),
                };
                out.replace(template, replacement.as_str())
            })
    }
}
fn export_image_page(
    config: &CompileConfig,
    page: &Page,
    output: &Output,
    fmt: ImageExportFormat,
) -> StrResult<()> {
    match fmt {
        ImageExportFormat::Png => {
            let pixmap = typst_render::render(page, config.ppi / 72.0);
            let buf = pixmap
                .encode_png()
                .map_err(|err| eco_format!("failed to encode PNG file ({err})"))?;
            output
                .write(&buf)
                .map_err(|err| eco_format!("failed to write PNG file ({err})"))?;
        }
        ImageExportFormat::Svg => {
            let svg = typst_svg::svg(page);
            output
                .write(svg.as_bytes())
                .map_err(|err| eco_format!("failed to write SVG file ({err})"))?;
        }
    }
    Ok(())
}

/// An image format to export in.
#[derive(Copy, Clone)]
enum ImageExportFormat {
    Png,
    Svg,
}

/// Export to a paged target format.
fn export_paged(document: &PagedDocument, config: &CompileConfig) -> SourceResult<Vec<Output>> {
    match config.output_format {
        OutputFormat::Pdf => export_pdf(document, config).map(|()| vec![config.output.clone()]),
        OutputFormat::Png => {
            export_image(document, config, ImageExportFormat::Png).at(Span::detached())
        }
        OutputFormat::Svg => {
            export_image(document, config, ImageExportFormat::Svg).at(Span::detached())
        }
        OutputFormat::Html => unreachable!(),
    }
}

/// Convert [`chrono::DateTime`] to [`Datetime`]
fn convert_datetime<Tz: chrono::TimeZone>(date_time: chrono::DateTime<Tz>) -> Option<Datetime> {
    Datetime::from_ymd_hms(
        date_time.year(),
        date_time.month().try_into().ok()?,
        date_time.day().try_into().ok()?,
        date_time.hour().try_into().ok()?,
        date_time.minute().try_into().ok()?,
        date_time.second().try_into().ok()?,
    )
}

/// Export to a PDF.
fn export_pdf(document: &PagedDocument, config: &CompileConfig) -> SourceResult<()> {
    // If the timestamp is provided through the CLI, use UTC suffix,
    // else, use the current local time and timezone.
    let timestamp = match config.creation_timestamp {
        Some(timestamp) => convert_datetime(timestamp).map(Timestamp::new_utc),
        None => {
            let local_datetime = chrono::Local::now();
            convert_datetime(local_datetime).and_then(|datetime| {
                Timestamp::new_local(datetime, local_datetime.offset().local_minus_utc() / 60)
            })
        }
    };

    let options = PdfOptions {
        ident: Smart::Auto,
        timestamp,
        page_ranges: config.pages.clone(),
        standards: config.pdf_standards.clone(),
        tagged: config.tagged,
    };
    let buffer = typst_pdf::pdf(document, &options)?;
    config
        .output
        .write(&buffer)
        .map_err(|err| eco_format!("failed to write PDF file ({err})"))
        .at(Span::detached())?;
    Ok(())
}
