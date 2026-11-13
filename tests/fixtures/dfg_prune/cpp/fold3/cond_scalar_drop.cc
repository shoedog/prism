int src();
void sink(int);
void f(bool c) {
  int x = src();  // @cond_scalar_drop:D x
  int y = src();
  int t = c ? x : y + 1;
  x = 0;
  int z = x + 1;  // @cond_scalar_drop:U:drop:both_branches_builtin_scalar
  sink(z + t);
}
