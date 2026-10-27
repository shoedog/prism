void f() {
  int x = source();
  try { work(); }
  catch (const Error &x) {
    sink(x);
  }
  sink(x);
}
