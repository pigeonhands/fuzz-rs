# fuzz-rs

[![](https://img.shields.io/crates/v/fuzz?style=for-the-badge)](https://crates.io/crates/fuzz)

```
cargo install fuzz
```


fuzz is a bruteforce/fuzzing tool written in rust to be working right away.

__Requires rust `nightly` to build__

Currently supported

| Feature | Description|
|----------|-------|
| [httpdir](#HttpDir) | http directory fuzzer/buster |


### **HttpDir** mode

Used to find http paths that return success http status codes `200`-`299`


### Basic usage:

| Command | Description |
| --------|------------|
|``` fuzz httpdir http://example.com ```| Use [default word list](https://github.com/digination/dirbuster-ng/blob/master/wordlists/common.txt) and no extentions |
| ```fuzz httpdir http://example.com -f --ignore-code 403 404``` | Log all requests except `403` and `404` status codes|
| ```fuzz httpdir http://example.com -x txt php``` | Append `.txt` and `.php` to each request (as well as the base word)|





Using `--default-ext` will add [extensions_common.txt](https://github.com/digination/dirbuster-ng/blob/master/wordlists/extensions_common.txt) to the extentions.

### Full usage:

```
USAGE:
    fuzz.exe httpdir [FLAGS] [OPTIONS] <TARGET>

ARGS:
    <TARGET>

FLAGS:
        --default-ext    Use default extention list (adds to -x if any)
    -e, --expand-url     Show full url (rather than /<word>)
    -g, --gzip           Compresss requests qith gzip
    -h, --help           Prints help information
    -f, --print-fails    Print/output non-success requests
        --silent         Disable console output
    -V, --version        Prints version information

OPTIONS:
    -d, --delay <delay>                    Minimum delay between word processing [default: 0]
    -x, --extentions <extentions>...       List of file extentions to append to word
        --ignore-code <ignore-codes>...    List of status codes to ignore
    -o, --out-file <out-file>              Save output to specified file
    -P, --password <password>              Basic auth password
    -t, --threads <threads>                Number of threads to use for fuzzing [default: 10]
        --timeout <timeout>                Http timeout in ms [default: 0]
        --agent <user-agent>               Request user agent
    -u, --username <username>              Basic auth username
    -w, --word-list <word-list>            Input work list used to fuzz

```

