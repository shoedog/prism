function f() {
  let x = source();
  while (ready()) {
    sink(x);
    x = nextValue();
  }
}
