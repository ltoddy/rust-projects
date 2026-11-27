use std::collections::HashMap;
use std::ffi::OsStr;
use std::fs;
use std::io;
use std::ops::AddAssign;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;
use std::sync::mpsc::{Receiver, SyncSender, sync_channel};

use crate::executor::ThreadPoolExecutor;

pub struct Calculator {
    filename_receiver: Receiver<PathBuf>,
    detail_sender: SyncSender<Detail>,
    executor: ThreadPoolExecutor,
}

impl Calculator {
    pub fn new(filename_receiver: Receiver<PathBuf>) -> (Self, Receiver<Detail>) {
        let (detail_sender, detail_receiver) = sync_channel::<Detail>(32);

        let calculator =
            Self { filename_receiver, detail_sender, executor: ThreadPoolExecutor::new() };

        (calculator, detail_receiver)
    }

    pub fn calculate(self) {
        let Calculator { filename_receiver, detail_sender, executor } = self;

        for filename in filename_receiver {
            let sender = SyncSender::clone(&detail_sender);
            executor.submit(move || {
                if let Some(info) =
                    filename.extension().and_then(|ext| MANAGER.get_by_extension(ext))
                    && let Ok(detail) = statistic(&filename, info)
                {
                    let _ = sender.send(detail);
                }
            });
        }
    }
}

fn statistic(filename: &Path, info: &Info) -> io::Result<Detail> {
    let content = fs::read_to_string(filename)?;
    let bytes = filename.metadata()?.len();
    let stats = count_lines(&content, info);

    Ok(Detail {
        language: info.language,
        files: 1,
        bytes,
        blank: stats.blank,
        comment: stats.comment,
        code: stats.code,
    })
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
struct Stats {
    blank: usize,
    comment: usize,
    code: usize,
}

fn count_lines(content: &str, info: &Info) -> Stats {
    let mut stats = Stats::default();
    let mut in_comment: Option<(&'static str, &'static str)> = None;

    'lines: for line in content.lines() {
        let line = line.trim();

        // empty line
        if line.is_empty() {
            stats.blank += 1;
            continue;
        }

        // match single line comments
        for single in info.single {
            if line.starts_with(single) {
                stats.comment += 1;
                continue 'lines;
            }
        }

        // match multi line comments
        for &(start, end) in info.multi {
            if let Some(delimiters) = in_comment
                && delimiters != (start, end)
            {
                continue;
            }

            // multi line comments maybe in one line
            let mut same_line = false;
            if line.starts_with(start) {
                if in_comment.is_some() {
                    stats.comment += 1;
                    in_comment = None;
                    continue 'lines;
                }
                same_line = true;
                in_comment = Some((start, end));
            }

            // This line is in comments
            if in_comment.is_some() {
                stats.comment += 1;
                if line.ends_with(end) {
                    if same_line {
                        if line.len() >= start.len() + end.len() {
                            in_comment = None;
                        }
                    } else {
                        in_comment = None;
                    }
                }
                continue 'lines;
            }
        }

        stats.code += 1;
    }

    stats
}

#[derive(Debug)]
struct Info {
    language: &'static str,
    single: &'static [&'static str],
    multi: &'static [(&'static str, &'static str)],
}

struct Manager {
    languages: HashMap<&'static str, Info>,
    ext_to_language: HashMap<&'static str, &'static str>,
}

impl Manager {
    fn get_by_extension(&self, ext: &OsStr) -> Option<&Info> {
        ext.to_str()
            .and_then(|ext| self.ext_to_language.get(ext))
            .and_then(|language| self.languages.get(language))
    }
}

static MANAGER: LazyLock<Manager> = LazyLock::new(|| {
    let mut languages = HashMap::<&'static str, Info>::new();
    let mut ext_to_language = HashMap::new();

    macro_rules! language {
        ($language: expr, $ext: expr, $single: expr, $multi: expr) => {{
            languages
                .insert($language, Info { language: $language, single: $single, multi: $multi });
            for &e in $ext {
                ext_to_language.insert(e, $language);
            }
        }};
        ($language: expr, $ext: expr, $single: expr) => {
            language!($language, $ext, $single, &[])
        };
        ($language: expr, $ext: expr) => {
            language!($language, $ext, &[], &[])
        };
    }

    language!("ABAP", &["abap"], &["*", "\\\""]);
    language!("ABNF", &["abnf"], &[";"]);
    language!("ActionScript", &["as"], &["//"], &[("/*", "*/")]);
    language!("Ada", &["ada", "adb", "ads", "pad"], &["--"]);
    language!("Agda", &["agda"], &["--"], &[("{-", "-}")]);
    language!("Alloy", &["als"], &["--", "//"], &[("/*", "*/")]);
    language!("Arduino C++", &["ino"], &["//"], &[("/*", "*/")]);
    language!("Assembly", &["asm"], &[";"]);
    language!("GNU Style Assembly", &["s"], &["//"], &[("/*", "*/")]);
    language!("ASP", &["asa", "asp"], &["'", "REM"]);
    language!(
        "ASP.NET",
        &["asax", "ascx", "asmx", "aspx", "master", "sitemap", "webinfo"],
        &[],
        &[("<!--", "-->"), ("<%--", "-->")]
    );
    language!("Autoconf", &["in"], &["#", "dnl"]);
    language!("Automake", &["am"], &["#"]);
    language!("Bash", &["bash"], &["#"]);
    language!("Batch", &["bat", "btm", "cmd"], &["REM", "::"]);
    language!("Cabal", &["cabal"], &["--"], &[("{-", "-}")]);
    language!("C", &["c"], &["//"], &[("/*", "*/")]);
    language!("Ceylon", &["ceylon"], &["//"], &[("/*", "*/")]);
    language!("C Header", &["h"], &["//"], &[("/*", "*/")]);
    language!("Clojure", &["clj"], &[";"]);
    language!("ClojureScript", &["cljs"], &[";"]);
    language!("ClojureC", &["cljc"], &[";"]);
    language!("CMake", &["cmake"], &["#"]);
    language!("Cobol", &["cob", "cbl", "ccp", "cobol", "cpy"], &["*"]);
    language!("CoffeeScript", &["coffee", "cjsx"], &["#"], &[("###", "###")]);
    language!("Coq", &["v"], &[], &[("(*", "*)")]);
    language!("C++", &["cc", "cpp", "cxx", "c++", "pcc", "tpp"], &["//"], &[("/*", "*/")]);
    language!("C++ Header", &["hh", "hpp", "hxx", "inl", "ipp"], &["//"], &[("/*", "*/")]);
    language!("Crystal", &["crystal"], &["#"]);
    language!("C#", &["cs", "csx"], &["//"], &[("/*", "*/")]);
    language!("CSS", &["css"], &["//"], &[("/*", "*/")]);
    language!("D", &["d"], &["//"], &[("/*", "*/")]);
    language!("DAML", &["daml"], &["--"], &[("{-", "-}")]);
    language!("dart", &["dart"], &["//"], &[("/*", "*/")]);
    language!("Emacs Lisp", &["el"], &[";"]);
    language!("Elixir", &["ex", "exs"], &["#"]);
    language!("Elm", &["elm"], &["--"], &[("{-", "-}")]);
    language!("Erlang", &["erl", "hrl"], &["%"]);
    language!(
        "Fortran",
        &["F", "f90", "f95", "f03", "f08", "f15", "f77", "f", "for", "ftn", "fpp"],
        &["!"]
    );
    language!("FreeMarker", &["ftl", "ftlh", "ftlx"], &[], &[("<#--", "-->")]);
    language!("F#", &["fs", "fsi", "fsx", "fsscript"], &["//"], &[("(*", "*)")]);
    language!("Go", &["go"], &["//"], &[("/*", "*/"), ("/**", "*/")]);
    language!("Go HTML", &["gohtml"], &[], &[("<!--", "-->"), ("{{/*", "*/}}")]);
    language!("GraphQL", &["gql", "graphql"], &["#"]);
    language!("Groovy", &["groovy", "grt", "gtpl", "gvy"], &["//"], &[("/*", "*/")]);
    language!("Gradle", &["gradle"], &["//"], &[("/*", "*/"), ("/**", "*/")]);
    language!("Haskell", &["hs"], &["--"], &[("{-", "-}")]);
    language!("Haxe", &["hx"], &["//"], &[("/*", "*/")]);
    language!("Html", &["html", "xhtml", "hml"], &[], &[("<!--", "-->")]);
    language!("Idris", &["idr", "lidr"], &["--"], &[("{-", "-}")]);
    language!("Ini", &["ini"], &[";", "#"]);
    language!("Java", &["java"], &["//"], &[("/*", "*/")]);
    language!("JavaScript", &["js", "mjs"], &["//"], &[("/*", "*/")]);
    language!("JSON", &["json"]);
    language!("JSX", &["jsx"], &["//"], &[("/*", "*/")]);
    language!("Julia", &["jl"], &["#"], &[("#=", "=#")]);
    language!("Jupyter Notebooks", &["ipynb"]);
    language!("Kotlin", &["kt", "kts"], &["//"], &[("/*", "*/")]);
    language!("Less", &["less"], &["//"], &[("/*", "*/")]);
    language!("LLVM", &["ll"], &[";"]);
    language!("Lua", &["lua"], &["--"], &[("--[[", "]]")]);
    language!("Lucius", &["lucius"], &["//"], &[("/*", "*/")]);
    language!("Markdown", &["md", "markdown"]);
    language!("Mint", &["mint"]);
    language!("Nim", &["nim"], &["#"]);
    language!("Nix", &["nix"], &[], &[("/*", "*/")]);
    language!("Objective-C", &["m"], &["//"], &[("/*", "*/")]);
    language!("Objective-C++", &["mm"], &["//"], &[("/*", "*/")]);
    language!("OCaml", &["ml", "mli", "re", "rei"], &[], &[("/*", "*/")]);
    language!("Org", &["org"], &["#"]);
    language!("Pascal", &["pas", "pp"], &["//"], &[("{", "}"), ("(*", "*)")]);
    language!("Perl", &["pl", "pm"], &["#"], &[("=pod", "=cut")]);
    language!("Pest", &["pest"], &["//"]);
    language!("Plain Text", &["text", "txt"]);
    language!(
        "Php",
        &["php4", "php5", "php", "phtml"],
        &["#", "//"],
        &[("/*", "*/"), ("/**", "*/")]
    );
    language!("PostCSS", &["pcss", "sss"], &["//"], &[("/*", "*/")]);
    language!("Prolog", &["p", "pro"], &["%"]);
    language!("Protocol Buffer", &["proto"], &["//"]);
    language!(
        "PowerShell",
        &["ps1", "psm1", "psd1", "ps1xml", "cdxml", "pssc", "psc1"],
        &["#"],
        &[("<#", "#>")]
    );
    language!("PureScript", &["purs"], &["--"], &[("{-", "-}")]);
    language!("Python", &["py"], &["#"], &[("'''", "'''"), (r#"""#, r#"""#)]);
    language!("QCL", &["qcl"], &["//"], &[("/*", "*/")]);
    language!("R", &["r"], &["#"]);
    language!("Racket", &["rkt"], &[";"], &[("#|", "|#")]);
    language!("Rakefile", &["rake"], &["#"], &[("=begin", "=end")]);
    language!("Rakudo", &["pl6", "pm6"], &["#"], &[("=begin", "=end")]);
    language!("Rust", &["rs"], &["//", "///", "///!"], &[("/*", "*/")]);
    language!("Ruby", &["rb"], &["#"], &[("=begin", "=end")]);
    language!("Ruby HTML", &["erb", "rhtml"], &[], &[("<!--", "-->")]);
    language!("ReStructuredText", &["rst"]);
    language!("Sass", &["sass", "scss"], &["//"], &[("/*", "*/")]);
    language!("Scala", &["scala", "sc"], &["//"], &[("/*", "*/")]);
    language!("Scheme", &["scm", "ss"], &[";"], &[("#|", "|#")]);
    language!("Shell", &["sh"], &["#"]);
    language!("Solidity", &["sol"], &["//"], &[("/*", "*/")]);
    language!("SQL", &["sql"], &["#", "--"], &[("/*", "*/")]);
    language!("Stylus", &["styl"], &["//"], &[("/*", "*/")]);
    language!("SVG", &["svg"], &[], &[("<!--", "-->")]);
    language!("Swift", &["swift"], &["//"], &[("/*", "*/")]);
    language!("TCL", &["tcl"], &["#"]);
    language!("Terraform", &["tf", "tfvars"], &["#", "//"], &[("/*", "*/")]);
    language!("TeX", &["tex", "sty"], &["%"]);
    language!("Thrift", &["thrift"], &["#", "//"], &[("/*", "*/")]);
    language!("Toml", &["toml"], &["#"]);
    language!("TSX", &["tsx"], &["//"], &[("/*", "*/")]);
    language!("TypeScript", &["ts"], &["//"], &[("/*", "*/")]);
    language!("VBScript", &["vbs"], &["'", "REM"]);
    language!("Visual Basic", &["vb"], &["'"]);
    language!("Visual Studio Solution", &["sln"]);
    language!("Visual Studio Project", &["vcproj", "vcxproj"], &[], &[("<!--", "-->")]);
    language!("Vim script", &["vim"], &["\\\""], &[("\\\"", "\\\""), ("'", "'")]);
    language!("Vue", &["vue"], &["//"], &[("<!--", "-->"), ("/*", "*/")]);
    language!("WebAssembly", &["wat", "wast"], &[";;"]);
    language!("XML", &["xml"], &[], &[("<!--", "-->"), ("<![CDATA[", "]]>")]);
    language!("Yaml", &["yml", "yaml"], &["#"]);
    language!("Zig", &["zig"], &["//"]);
    language!("Zsh", &["zsh"], &["#"]);

    Manager { languages, ext_to_language }
});

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Detail {
    pub language: &'static str,
    pub files: usize,
    pub bytes: u64,
    pub blank: usize,
    pub comment: usize,
    pub code: usize,
}

impl AddAssign<&Detail> for Detail {
    fn add_assign(&mut self, rhs: &Detail) {
        self.files += rhs.files;
        self.bytes += rhs.bytes;
        self.blank += rhs.blank;
        self.comment += rhs.comment;
        self.code += rhs.code;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn info(
        single: &'static [&'static str],
        multi: &'static [(&'static str, &'static str)],
    ) -> Info {
        Info { language: "Test", single, multi }
    }

    #[test]
    fn counts_blank_and_code_lines() {
        let info = info(&["//"], &[("/*", "*/")]);
        let content = "fn main() {}\n\n   \nlet a = 1;\n";

        assert_eq!(count_lines(content, &info), Stats { blank: 2, comment: 0, code: 2 });
    }

    #[test]
    fn counts_single_line_comments() {
        let info = info(&["//"], &[]);
        let content = "// comment\nlet a = 1; // trailing\n//!  inner doc\n";

        assert_eq!(count_lines(content, &info), Stats { blank: 0, comment: 2, code: 1 });
    }

    #[test]
    fn counts_multi_line_comments() {
        let info = info(&[], &[("/*", "*/")]);
        let content = "/* one line */\n/*\n * two\n */\nlet a = 1;\n";

        assert_eq!(count_lines(content, &info), Stats { blank: 0, comment: 4, code: 1 });
    }

    #[test]
    fn counts_unterminated_multi_line_comment_to_the_end() {
        let info = info(&[], &[("/*", "*/")]);
        let content = "/*\nstill comment\n";

        assert_eq!(count_lines(content, &info), Stats { blank: 0, comment: 2, code: 0 });
    }

    #[test]
    fn looks_up_language_by_extension() {
        let rust = MANAGER.get_by_extension(OsStr::new("rs")).expect("Rust is registered");

        assert_eq!(rust.language, "Rust");
        assert!(MANAGER.get_by_extension(OsStr::new("unknown_ext")).is_none());
    }

    #[test]
    fn accumulates_details() {
        let mut sum = Detail { language: "Sum", files: 0, bytes: 0, blank: 0, comment: 0, code: 0 };
        let detail =
            Detail { language: "Rust", files: 1, bytes: 42, blank: 2, comment: 3, code: 5 };

        sum += &detail;

        assert_eq!(sum.files, 1);
        assert_eq!(sum.bytes, 42);
        assert_eq!(sum.blank, 2);
        assert_eq!(sum.comment, 3);
        assert_eq!(sum.code, 5);
    }
}
