fn main() {
  let mut res = winres::WindowsResource::new();
  res.set_icon("src/data/icon.ico");
  res.compile().unwrap();
}
