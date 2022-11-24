use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

use clap::Parser;
use phf::phf_map;
use walkdir::WalkDir;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Directory to count the lines of
    #[arg(short, long, default_value_t = String::from("."))]
    directory: String,

    /// If this should take comments into account
    #[arg(long = "comments", default_value_t = false)]
    count_comments: bool,
    // If this should take empty lines into account
    #[arg(long = "empty", default_value_t = false)]
    count_empty: bool,
}

enum CommentSyntax<'a> {
    LineStart(&'a str),
    Range(&'a str, &'a str),
}

const C_STYLE: &[CommentSyntax] = &[
    CommentSyntax::LineStart("//"),
    CommentSyntax::Range("/*", "*/"),
];
const HASH: &[CommentSyntax] = &[CommentSyntax::LineStart("#")];
const MATLAB: &[CommentSyntax] = &[
    CommentSyntax::LineStart("%"),
    CommentSyntax::Range("%{", "}%"),
];
const SCHEME: &[CommentSyntax] = &[
    CommentSyntax::LineStart(";"),
    CommentSyntax::Range("#|", "|#"),
];

struct Language<'a> {
    name: &'a str,
    comments: &'a [CommentSyntax<'a>],
}

impl<'a> Language<'a> {
    const fn new(name: &'a str, comments: &'a [CommentSyntax<'a>]) -> Self {
        Self { name, comments }
    }
}

const IGNORE_DIRS: &[&str] = &["target", "build"];

static LANGUAGES: phf::Map<&'static str, Language<'static>> = phf_map! {
    "rs" => Language::new("Rust", C_STYLE),
    "go" => Language::new("Go", C_STYLE),
    "h" => Language::new("C", C_STYLE),
    "c" => Language::new("C", C_STYLE),
    "hpp" => Language::new("C++", C_STYLE),
    "cpp" => Language::new("C++", C_STYLE),
    "cs" => Language::new("C#", C_STYLE),
    "java" => Language::new("Java", C_STYLE),
    "js" => Language::new("javascript", C_STYLE),
    "carbon" => Language::new("Carbon", C_STYLE),
    "swift" => Language::new("Swift", C_STYLE),
    "dart" => Language::new("Dart", C_STYLE),
    "sc" => Language::new("Scala", C_STYLE),
    "kt" => Language::new("Kotlin", C_STYLE),
    "hla" => Language::new("HLA", C_STYLE),
    "lua" => Language::new("Lua", C_STYLE),
    "rhai" => Language::new("Rhai", C_STYLE),

    "ts" => Language::new("Scala", &[CommentSyntax::Range("/**", "*/")]),


    "wgsl" => Language::new("wglsl", C_STYLE),
    "glsl" => Language::new("glsl", C_STYLE),
    "hlsl" => Language::new("hlsl", C_STYLE),


    "php" => Language::new("Swift", &[CommentSyntax::LineStart("//"), CommentSyntax::LineStart("#"), CommentSyntax::Range("/*", "*/")]),
    "hs" => Language::new("Haskell", &[CommentSyntax::LineStart("--"), CommentSyntax::Range("{-", "-}")]),
    "rb" => Language::new("Ruby", &[CommentSyntax::LineStart("#"), CommentSyntax::Range("=begin", "=end")]),
    "asm" => Language::new("Assembly", &[CommentSyntax::LineStart(";")]),
    "tao" => Language::new("Tao", &[CommentSyntax::LineStart("##")]),

    "html" => Language::new("html", &[CommentSyntax::Range("<!--", "-->")]),
    "css" => Language::new("css", &[CommentSyntax::Range("/*", "*/")]),
    "zig" => Language::new("Zig", &[CommentSyntax::LineStart("//")]),

    "py" => Language::new("Python", HASH),
    "r" => Language::new("R", HASH),
    "pl" => Language::new("Perl", HASH),
    "emojic" => Language::new("emojicode", HASH),

    "toml" => Language::new("TOML", HASH),
    "gitignore" => Language::new("git ignore", HASH),
    "makefile" => Language::new("make file", HASH),
    "bash" => Language::new("bash script", HASH),

    "bat" => Language::new("batch script", &[CommentSyntax::LineStart("Rem"), CommentSyntax::LineStart("::")]),

    "m" => Language::new("Matlab", MATLAB),
    "mat" => Language::new("Matlab", MATLAB),

    "ss" => Language::new("Scheme", SCHEME),
    "sls" => Language::new("Scheme", SCHEME),
    "scm" => Language::new("Scheme", SCHEME),
};

#[derive(Default)]
struct CountResult {
    languages: HashMap<String, usize>,
    total: usize,
}

fn count_dir(dir: &Path, count_empty: bool, count_comments: bool) -> CountResult {
    let mut res = CountResult::default();
    for entry in WalkDir::new(dir)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| {
            e.file_type().is_file()
                && !IGNORE_DIRS.iter().any(|d| {
                    e.path()
                        .ancestors()
                        .find(|anc| {
                            anc != &e.path()
                                && (anc.to_str().map_or(false, |s| s.contains("/."))
                                    || anc.ends_with(d))
                        })
                        .is_some()
                })
        })
    {
        let Ok(src) = fs::read_to_string(entry.path()) else {
            continue;
        };

        let name = entry.file_name().to_string_lossy();
        let lang = name.split('.').last().unwrap();

        let mut lines = src.lines().filter_map(|line| {
            let line = line.trim();
            (count_empty || !line.is_empty()).then_some(line)
        });

        let lines = if let Some(language) = LANGUAGES.get(lang).filter(|_| !count_comments) {
            let mut count = 0;
            while let Some(line) = lines.next() {
                let mut skip = false;
                'comments: for sntx in language.comments {
                    match sntx {
                        CommentSyntax::LineStart(start) => {
                            if line.starts_with(start) {
                                skip = true;
                                break;
                            }
                        }
                        CommentSyntax::Range(start, end) => {
                            if let Some(i) = line
                                .find(start)
                                .filter(|i| line.find(end).map_or(true, |j| j < *i))
                            {
                                if i > 0 {
                                    count += 1;
                                }
                                while let Some(line) = lines.next() {
                                    if let Some(i) = line.find(end) {
                                        skip = i + end.len() == line.len();

                                        break 'comments;
                                    }
                                }
                            }
                        }
                    }
                }
                if !skip {
                    count += 1;
                }
            }
            count
        } else {
            lines.count()
        };

        *res.languages.entry(lang.to_string()).or_insert(0) += lines;
        res.total += lines;
    }
    res
}

fn main() {
    let args = Args::parse();
    let res = count_dir(
        &PathBuf::from(args.directory),
        args.count_empty,
        args.count_comments,
    );
    let mut languages: Vec<_> = res.languages.into_iter().collect();

    languages.sort_by_key(|e| e.1);

    for (lang, count) in languages {
        let name = if let Some(lang) = LANGUAGES.get(&lang) {
            lang.name
        } else {
            &lang
        };
        println!("{}: {}", name, count);
    }

    println!("Total: {}", res.total);
}