use std::collections::HashMap;
use std::ffi::OsStr;
use std::fs;
use std::ops::AddAssign;
use std::path::Path;
use std::sync::LazyLock;

use crate::content::is_binary;
use crate::error::Error;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Detail {
    language: &'static str,
    files: usize,
    bytes: u64,
    blank: usize,
    comment: usize,
    code: usize,
}

impl Detail {
    pub fn new(
        language: &'static str,
        bytes: u64,
        blank: usize,
        comment: usize,
        code: usize,
    ) -> Self {
        Self { language, files: 1, bytes, blank, comment, code }
    }

    pub fn language(self) -> &'static str {
        self.language
    }

    pub fn files(self) -> usize {
        self.files
    }

    pub fn bytes(self) -> u64 {
        self.bytes
    }

    pub fn blank(self) -> usize {
        self.blank
    }

    pub fn comment(self) -> usize {
        self.comment
    }

    pub fn code(self) -> usize {
        self.code
    }
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

impl Detail {
    pub fn read(path: &Path) -> Result<Self, Error> {
        let extension = path.extension().ok_or_else(|| Error::unsupported(path))?;
        let info = LANGUAGES.info(extension).ok_or_else(|| Error::unsupported(path))?;

        let bytes = fs::read(path).map_err(|source| Error::read_file(path, source))?;
        let size = bytes.len() as u64;

        if is_binary(&bytes) {
            return Err(Error::not_text(path));
        }

        let content = String::from_utf8(bytes).map_err(|_| Error::not_text(path))?;
        let stats = count_lines(&content, info);

        Ok(Detail::new(info.language, size, stats.blank, stats.comment, stats.code))
    }
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

        if line.is_empty() {
            stats.blank += 1;
            continue;
        }

        for single in info.single {
            if line.starts_with(single) {
                stats.comment += 1;
                continue 'lines;
            }
        }

        for &(start, end) in info.multi {
            if let Some(delimiters) = in_comment
                && delimiters != (start, end)
            {
                continue;
            }

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

struct Languages {
    languages: HashMap<&'static str, Info>,
    extensions: HashMap<&'static str, &'static str>,
}

impl Languages {
    fn new() -> Self {
        let mut languages = HashMap::<&'static str, Info>::new();
        let mut extensions = HashMap::<&'static str, &'static str>::new();

        macro_rules! language {
            ($language:expr, $extensions:expr, $single:expr, $multi:expr) => {{
                languages.insert(
                    $language,
                    Info { language: $language, single: $single, multi: $multi },
                );
                for &extension in $extensions {
                    extensions.insert(extension, $language);
                }
            }};
            ($language:expr, $extensions:expr, $single:expr) => {
                language!($language, $extensions, $single, &[])
            };
            ($language:expr, $extensions:expr) => {
                language!($language, $extensions, &[], &[])
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

        Self { languages, extensions }
    }

    fn info(&self, extension: &OsStr) -> Option<&Info> {
        extension
            .to_str()
            .and_then(|ext| self.extensions.get(ext))
            .and_then(|language| self.languages.get(language))
    }
}

static LANGUAGES: LazyLock<Languages> = LazyLock::new(Languages::new);
