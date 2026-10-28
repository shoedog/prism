void f() {
  int x = source();
  {
    int a = 1, x = 2;
    sink(a);
    sink(x);
  }
  sink(x);
}
