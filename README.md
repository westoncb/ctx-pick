# ctx-pick

[](https://www.google.com/search?q=https://crates.io/crates/ctx-pick)
[](https://opensource.org/licenses/MIT)

`ctx-pick` is a simple command-line utility that gathers file contents, formats them into a single Markdown string, and copies it to your clipboard. It's designed to make it effortless to provide code context to LLMs.

It can find files by direct path, directory, partial name, suffix, or even **glob patterns**. It can also extract abbreviated "source views" using the --depth param which controls how far the algorithm walks the parse tree for a given source file collecting tokens. Currently Rust, Python and Typescript are supported.

---

## Installation

You can install `ctx-pick` directly from Crates.io using Cargo:

```sh
cargo install ctx-pick
```

---

## Usage

The basic command structure is to provide a space-separated list of inputs, followed by any optional flags.

```sh
ctx-pick [INPUTS]... [OPTIONS]
```

### Options

- `--depth <LEVEL>`: Instead of full file content, this extracts a structural "skeleton" of the code (e.g., function signatures, struct definitions). This is for getting a high-level overview of a file's structure. A depth of `2-4` is usually effective. The depth indicates how far the algorithm walks a parse tree of the source file collecting tokens.

- `--to-stdout`: Print the final context to stdout instead of copying to the clipboard.

---

## Examples

**1. Pick specific files by path:**

```sh
ctx-pick src/main.rs src/types.rs
```

**2. Grab all files within a directory:**

```sh
# This will recursively find all files in the 'src' directory
ctx-pick src
```

**3. Use glob patterns to select files:**

> **Note:** It's good practice to quote your glob patterns to prevent your shell from expanding them.

```sh
# Grab all TypeScript files in the 'src' directory, recursively
ctx-pick 'src/**/*.ts'

# Grab all Rust and TypeScript files in the root
ctx-pick '*.rs' '*.ts'
```

**4. Use partial names or path suffixes:**

> `ctx-pick` will find files whose relative paths contain the input string.

```sh
# Finds 'src/display.rs' by its partial name
ctx-pick display

# Finds 'src/file_resolver.rs' by its suffix
ctx-pick file_resolver
```

**5. Extract Code Skeletons:**

```sh
# Get the skeletons of main.rs and the file_resolver at depth 4
ctx-pick main file_resolver --depth=4
```

---

## Output & Previews

`ctx-pick` provides a rich preview of its actions in your terminal (`stderr`) so you always know what's been copied.

### Example 1: Full Content Mode

```sh
$ ctx-pick src/main.rs src/error.rs
✅ Context copied to clipboard (2 files, 1000 lines)
========================================
Included files:

1. src/main.rs
    📄 600 lines

2. src/error.rs
    📄 400 lines
========================================
```

### Example 2: Skeleton Mode

The output indicates that a skeleton was generated, at what depth, and shows the character count of the resulting skeleton for each file.

```sh
$ ctx-pick src/main.rs src/display.rs --depth=4
✅ Context skeleton copied to clipboard (2 files, 1452 characters)
========================================
Included files:

1. src/main.rs (skeleton only; depth=4)
    🧬 850 characters

2. src/display.rs (skeleton only; depth=4)
    🧬 602 characters
========================================
```

### Clipboard Content

The content copied to your clipboard is formatted in clean Markdown.

**Full Content:**

````markdown
src/config.rs

```rust
use crate::error::AppError;
use std::env;
// ... (rest of file content) ...
```

src/error.rs

```rust
use thiserror::Error;
// ... (rest of file content) ...
```
````

**Skeleton Content (`--depth`):**

````markdown
src/symbol_extractor.rs

```
pub fn create_skeleton_by_depth ( source_code : & str , file_extension : & str , max_depth : usize ) -> Result < String , String > { ... } fn collect_tokens_at_depth ( node : Node , current_depth : usize , max_depth : usize , tokens : & mut Vec < String > , source_bytes : & [ u8 ] ) { ... }
```
````
