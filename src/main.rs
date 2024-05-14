use clap::{error::ErrorKind, CommandFactory, Parser};
use lisel::index::Type;
use lisel::select::{Select, SelectError};
use regex::Regex;
use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::mem;

/// Select lines from target by index.
#[derive(Parser, Debug)]
#[command(version, about)]
struct Cli {
    /// Target filenames, accepts one or two filenames.
    ///
    /// 2 files:
    /// The first file is INDEX, the second is TARGET.
    ///
    /// 1 file:
    /// The file is INDEX, stdin is TARGET.
    #[arg(value_name = "FILE", num_args = 1..=2, verbatim_doc_comment)]
    files: Vec<String>,
    /// Swap file role: INDEX and TARGET.
    #[arg(short, long)]
    swap_file_role: bool,
    /// Regular expression to determine whether the index of the row exists.
    ///
    /// When a certain line in INDEX matches, output the TARGET line corresponding to that line number.
    /// Default: .+
    #[arg(short = 'e', long, value_parser = Regex::new, verbatim_doc_comment)]
    index_regex: Option<Regex>,
    /// Reverse lines to output and lines not to output.
    #[arg(short = 'v', long)]
    index_invert_match: bool,
    /// Use line number index.
    ///
    /// Instead of selecting rows from INDEX with regular expression, use a line in the following format as index.
    ///
    ///   LINE_NUMBER
    ///
    /// selects line LINE_NUMBER of TARGET.
    ///
    ///   LINE_START,LINE_END
    ///
    /// selects lines LINE_START to LINE_END (LINE_START <= LINE_END) of TARGET.
    ///
    ///   LINE_START,
    ///
    /// selects lines LINE_START of TARGET to the end of TARGET.
    ///
    ///   ,LINE_END
    ///
    /// selects lines the beginning of TARGET to LINE_END of TARGET.
    ///
    /// LINE_NUMBER and LINE_START are greater than the LINE_NUMBER and LINE_END of previous lines in the INDEX file.
    ///
    /// Debug logging can be enabled via RUST_LOG in env_logger https://crates.io/crates/env_logger.
    #[arg(short = 'n', long, conflicts_with_all = ["index_regex"], verbatim_doc_comment)]
    index_line_number: bool,
}

fn main() {
    env_logger::init();
    let cli = Cli::parse();
    if let Err(r) = run(&cli) {
        let mut cmd = Cli::command();
        cmd.error(r.0, r.1).exit();
    }
}

#[derive(Debug)]
struct RunError(ErrorKind, String);

fn run(cli: &Cli) -> Result<(), RunError> {
    let index_type = new_index_type(cli.index_regex.clone(), cli.index_line_number);

    match cli.files.as_slice() {
        [f1, f2] => {
            let mut index_file = f1;
            let mut target_file = f2;

            if cli.swap_file_role {
                mem::swap(&mut target_file, &mut index_file);
            }

            let target = File::open(target_file)
                .map(BufReader::new)
                .map_err(|x| RunError(ErrorKind::InvalidValue, x.to_string()))?;
            let index = File::open(index_file)
                .map(BufReader::new)
                .map_err(|x| RunError(ErrorKind::InvalidValue, x.to_string()))?;

            let selector = Select::new(target, index, index_type, cli.index_invert_match);

            for line in selector {
                let r = line.map_err(|x| {
                    RunError(
                        match x {
                            SelectError::Io(_) => ErrorKind::Io,
                            SelectError::Parse(_) => ErrorKind::InvalidValue,
                        },
                        x.to_string(),
                    )
                })?;
                print!("{}", r);
            }
            Ok(())
        }
        [f1] => {
            let stdin = io::stdin();
            let target_stdin = stdin.lock();
            let mut target: Box<dyn BufRead> = Box::new(target_stdin);
            let index_file = File::open(f1)
                .map(BufReader::new)
                .map_err(|x| RunError(ErrorKind::InvalidValue, x.to_string()))?;
            let mut index: Box<dyn BufRead> = Box::new(index_file);

            if cli.swap_file_role {
                mem::swap(&mut target, &mut index);
            }

            let selector = Select::new(target, index, index_type, cli.index_invert_match);

            for line in selector {
                let r = line.map_err(|x| {
                    RunError(
                        match x {
                            SelectError::Io(_) => ErrorKind::Io,
                            SelectError::Parse(_) => ErrorKind::InvalidValue,
                        },
                        x.to_string(),
                    )
                })?;
                print!("{}", r);
            }
            Ok(())
        }
        _ => Err(RunError(
            ErrorKind::WrongNumberOfValues,
            "files".to_string(),
        )),
    }
}

fn new_index_type(r: Option<Regex>, index_line_number: bool) -> Option<Type> {
    if index_line_number {
        None
    } else {
        r.or_else(|| Some(Regex::new(".+").unwrap())).map(Type::Re)
    }
}

#[cfg(test)]
mod tests {
    use std::fs::File;
    use std::io::Write;
    use std::process::{Command, Stdio};
    use tempfile::TempDir;

    macro_rules! test_e2e {
        ($name:expr, $dir:expr, $bin:expr, $args:expr, $data:expr, $stdin:expr, $want:expr) => {{
            eprint!("test {} ... ", $name);

            let f1_path = $dir.path().join(format!("{}_f1", $name));
            {
                let mut f1 = File::create(&f1_path).expect("failed to create 1st file");
                f1.write_all($data.as_bytes())
                    .expect("failed to write data to 1st file");
            }

            let mut args = vec![f1_path.to_str().unwrap()];
            args.extend_from_slice(&$args);
            let mut process = Command::new($bin)
                .args(args.clone())
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .expect("failed to spawn process");
            if let Some(ref mut stdin) = process.stdin {
                stdin
                    .write_all($stdin.as_bytes())
                    .expect("failed to write data to stdin");
            }

            let output = process.wait_with_output().expect("failed to wait process");
            assert!(output.status.success());

            let got = String::from_utf8(output.stdout).expect("failed to read stdout");
            let err = String::from_utf8(output.stderr).expect("failed to read stderr");

            assert_eq!(
                $want, got,
                "{} stdout, args: {:?} err: {}",
                $name, &args, err
            );

            eprintln!("ok");
        }};
    }

    macro_rules! test_e2e_files {
        ($name:expr, $dir:expr, $bin:expr, $args:expr, $index:expr, $target:expr, $want:expr) => {{
            eprint!("test {} ... ", $name);

            let f1_path = $dir.path().join(format!("{}_f1", $name));
            let f2_path = $dir.path().join(format!("{}_f2", $name));
            {
                let mut f1 = File::create(&f1_path).expect("failed to create 1st file");
                let mut f2 = File::create(&f2_path).expect("failed to create 2nd file");
                f1.write_all($index.as_bytes())
                    .expect("failed to write index to 1st file");
                f2.write_all($target.as_bytes())
                    .expect("failed to write target to 2nd file");
            }

            let mut args = vec![f1_path.to_str().unwrap(), f2_path.to_str().unwrap()];
            args.extend_from_slice(&$args);
            let output = Command::new($bin)
                .args(args.clone())
                .output()
                .expect("failed to run process");
            assert!(
                output.status.success(),
                "{} status, args: {:?}",
                $name,
                &args
            );
            let got = String::from_utf8(output.stdout).expect("failed to read stdout");
            let err = String::from_utf8(output.stderr).expect("failed to read stderr");
            assert_eq!(
                $want, got,
                "{} stdout, args: {:?} err: {}",
                $name, &args, err
            );

            eprintln!("ok");
        }};
    }

    #[test]
    fn main() {
        let status = Command::new("cargo")
            .arg("build")
            .status()
            .expect("failed to execute build");
        assert!(status.success(), "{}", "cargo build");

        let bin = "./target/debug/lisel";
        let output = Command::new(bin)
            .arg("--help")
            .output()
            .expect("failed to execute help");
        assert!(output.status.success(), "{}", "help status");
        assert!(output.stdout.len() > 0, "{}", "help stdout");

        let tmp_dir = TempDir::new_in(".").unwrap();

        test_e2e!(
            "e2e_re_default",
            tmp_dir,
            bin,
            vec![],
            "1\n\n1\n",
            "l1\nl2\nl3\nl4\nl5\n",
            "l1\nl3\n"
        );
        test_e2e!(
            "e2e_re_default_invert",
            tmp_dir,
            bin,
            vec!["--index-invert-match"],
            "1\n\n1\n",
            "l1\nl2\nl3\nl4\nl5\n",
            "l2\nl4\nl5\n"
        );
        test_e2e!(
            "e2e_re_default_swap",
            tmp_dir,
            bin,
            vec!["--swap-file-role"],
            "l1\nl2\nl3\nl4\nl5\n",
            "1\n\n1\n",
            "l1\nl3\n"
        );

        test_e2e_files!(
            "e2e_files_re_default",
            tmp_dir,
            bin,
            vec![],
            "1\n\n1\n",
            "l1\nl2\nl3\nl4\nl5\n",
            "l1\nl3\n"
        );
        test_e2e_files!(
            "e2e_files_re",
            tmp_dir,
            bin,
            vec!["--index-regex", "^$"],
            "1\n\n1\n",
            "l1\nl2\nl3\nl4\nl5\n",
            "l2\n"
        );
        test_e2e_files!(
            "e2e_files_re_invert",
            tmp_dir,
            bin,
            vec!["--index-regex", "^$", "--index-invert-match"],
            "1\n\n1\n",
            "l1\nl2\nl3\nl4\nl5\n",
            "l1\nl3\nl4\nl5\n"
        );
        test_e2e_files!(
            "e2e_files_re_default_swap",
            tmp_dir,
            bin,
            vec!["--swap-file-role"],
            "l1\nl2\nl3\nl4\nl5\n",
            "1\n\n1\n",
            "l1\nl3\n"
        );
        test_e2e_files!(
            "e2e_files_number",
            tmp_dir,
            bin,
            vec!["--index-line-number"],
            "1\n3,4\n",
            "l1\nl2\nl3\nl4\nl5\n",
            "l1\nl3\nl4\n"
        );
        test_e2e_files!(
            "e2e_files_number",
            tmp_dir,
            bin,
            vec!["--index-line-number", "--index-invert-match"],
            "1\n3,4\n",
            "l1\nl2\nl3\nl4\nl5\n",
            "l2\nl5\n"
        );

        tmp_dir.close().unwrap();
    }
}
