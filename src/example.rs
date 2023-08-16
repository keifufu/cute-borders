struct Root {
  examples: Vec<Example>,
}

impl Root {
  // creates examples sometimes
}

struct Example {
  variable_that_keeps_getting_modified: u32,
}
impl Example {
  pub fn wnd_proc() {}
}

unsafe extern "system" fn wnd_proc(
  hwnd: HWND,
  msg: u32,
  wparam: WPARAM,
  lparam: LPARAM,
) -> LRESULT {
  // need to call back to self.wnd_proc();
}
