function f() {
  let x = source();
  {
    let x = clean();
    sink(x);
  }
  x = clean();
  sink(x);
}
