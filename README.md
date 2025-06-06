# ctx-pick

[](https://www.google.com/search?q=https://crates.io/crates/ctx-pick)
[](https://opensource.org/licenses/MIT)

`ctx-pick` is a simple command-line utility that gathers file contents, formats them into a single Markdown string, and copies it to your clipboard. It's designed to make it easy to provide code context to LLMs.

It intelligently finds files based on direct paths, directory names, or even partial file names, then reports on what it found before copying the final context.

---

## Installation

You can install `ctx-pick` directly from Crates.io using Cargo:

```sh
cargo install ctx-pick
```

---

## Usage

The basic command structure is to provide a space-separated list of files, directories, or partial names.

```sh
ctx-pick [INPUTS]...
```

### Examples

**1. Pick specific files by path:**

```sh
ctx-pick src/main.rs src/types.rs
```

**2. Grab all files within a directory:**

```sh
# This will recursively find all files in the 'src' directory
ctx-pick src
```

**3. Use partial file names:**

> `ctx-pick` will find files that contain the input string. It prioritizes exact filename matches over partial ones.

```sh
# Assuming 'main.rs' and 'display.rs' are the only files containing these strings
ctx-pick main display
```

**4. Combine all methods:**

```sh
# Gets lib.rs, all files in the 'tests' dir, and the file matching 'config'
ctx-pick src/lib.rs tests config
```

### Example Output

If successful, `ctx-pick` will report its actions to `stderr` and copy the context to your clipboard.

```sh
$ ctx-pick src/config.rs src/error.rs
✅ Context copied to clipboard (2 files, 23 lines)
========================================
Included files:

1. src/config.rs
   📄 20 lines
2. src/error.rs
   📄 3 lines

========================================
```

The content copied to the clipboard will be formatted in Markdown like this:

````markdown
src/config.rs

```rust
use crate::error::AppError; // We'll define this in the next step
use std::env;
// ... (rest of file content) ...
```

src/error.rs

```rust
use thiserror::Error;

#[derive(Error, Debug)]
// ... (rest of file content) ...
```
````
