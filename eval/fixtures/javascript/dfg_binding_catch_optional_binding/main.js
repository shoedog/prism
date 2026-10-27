function f() {
  let x = source();
  try {
    g();
  } catch {
    x = clean();
    sink(x);
  }
  sink(x);
}
