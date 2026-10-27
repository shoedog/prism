void f() {
  int x = source();
  {
    int x = clean();
    sink(x);
  }
  sink(x);
}
