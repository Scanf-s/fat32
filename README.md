# fat32

FAT32 image explorer in Rust.

The first command reads the boot sector of a raw FAT32 volume and reports where the root directory starts.

```sh
cargo run -- info fat32.img
```

In the sample image, the root directory starts at cluster `2`, at byte offset
`661504`.

List the root directory using its short FAT names. A trailing `/` marks a directory.

```sh
cargo run -- ls fat32.img
```
