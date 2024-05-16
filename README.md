# lisel

```
❯ lisel --help
Select lines from target by index

Usage: lisel [OPTIONS] [FILE]...

Arguments:
  [FILE]...
          Target filenames, accepts one (INDEX) or two filenames (INDEX and TARGET).
          
          2 files:
          The first file is INDEX, the second is TARGET.
          
          1 file:
          The file is INDEX, stdin is TARGET.

Options:
  -s, --swap-file-role
          Swap file role: INDEX and TARGET

  -e, --index-regex <INDEX_REGEX>
          Regular expression to determine whether the index of the row exists.
          
          When a certain line in INDEX matches, output the TARGET line corresponding to that line number.
          Default: .+

  -v, --index-invert-match
          Reverse lines to output and lines not to output

  -n, --index-line-number
          Use line number index.
          
          Instead of selecting rows from INDEX with regular expression, use a line in the following format as index.
          
            LINE_NUMBER
          
          selects line LINE_NUMBER of TARGET.
          
            LINE_START,LINE_END
          
          selects lines LINE_START to LINE_END (LINE_START <= LINE_END) of TARGET.
          
            LINE_START,
          
          selects lines LINE_START of TARGET to the end of TARGET.
          
            ,LINE_END
          
          selects lines the beginning of TARGET to LINE_END of TARGET.
          
          LINE_NUMBER and LINE_START are greater than the LINE_NUMBER and LINE_END of previous lines in the INDEX file.
          
          Debug logging can be enabled via RUST_LOG in env_logger https://crates.io/crates/env_logger.

  -h, --help
          Print help (see a summary with '-h')

  -V, --version
          Print version
```
