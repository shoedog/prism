void f() {
  int x = source();
  auto call = [y = (x = clean())] { sink(y); };
  sink(x);
}
