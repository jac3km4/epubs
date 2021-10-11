# epubs
Small and fast zero-copy EPUB library

## usage
```rust
let mut book = Epub::new(File::open("book.epub")?)?;

for point in book.read(Href::TOC)?.toc()?.points() {
  println!("{}", point.label.text);
}
```
