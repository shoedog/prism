void f(int x) {
  for (int a = 1, x = 2; ready(); step()) {
    sink(x);
  }
  sink(x);
}
