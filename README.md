# fat32

FAT32 image explorer in Rust.

The first command reads the boot sector of a raw FAT32 volume and reports where the root directory starts.

```sh
cargo run -- info fat32.img
```

In the sample image, the root directory starts at cluster `2`, at byte offset
`661504`.

List directories using their short FAT names. A trailing `/` in the output marks a directory.
Omit the directory path to list the root, or pass a path such as `/DOCS`.

```sh
cargo run -- ls fat32.img
cargo run -- ls fat32.img /DOCS
```
