struct Manual { bool done; };
void f(Manual* first, Manual* second) {
  int x = source();
  {
    int x = clean();
    sink(x);
  }
  sink(x);
  {
    Manual* m = first;
    m->done = false;
  }
  {
    Manual* m = second;
    if (!m->done) work();
    sink(m->done);
  }
}
