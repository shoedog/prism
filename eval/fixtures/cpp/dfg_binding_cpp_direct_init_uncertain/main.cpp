struct T {
  T(int);
};

void f(int y) {
  T x(y);
  sink(x);
}
