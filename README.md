# XTRMV - CLI Text Editor.

**xtrmv** is a minimalist CLI Text Editor Written In Rust inspired by Vi.

## How to install.

- Via Cargo. (If you have Cargo Installed)

```bash
cargo install xtrmv
```

- Via GitHub release. [here](https://github.com/NickyHariniaina/xtrmv/releases/tag/cli)

🔗 Features and Usage.

- Multiple Modes: Normal - Insert - Select.
- Command Support:
  - Vim j - k - l - h key to move.
  - 0: Go to start of line
  - $: Go to end of line
  - gg - Go to start of file.
  - G - Go to end of file.
  - w, b, e - Move by space.
  - H, L, M - Go to top, bottom, middle of file.
  - Vi O and o command to insert line.
  - i: Mode Insert.
  - a: Mode Insert (after the cursor).
  - v: Mode Select.
  - x: Delete a character under cursor.
  - <C-w>: Delete by space.
  - :w : Save
  - :q : Quit
  - :q! : Quit without saving.
  - :wq | :x : Save and quit.
- Auto-closed brackets support.

🔗 Contributing:

- You can open a PR to contribute to this project:

Feel free to use, copy, fork this project.
