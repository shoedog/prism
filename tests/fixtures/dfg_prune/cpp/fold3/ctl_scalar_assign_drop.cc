int src();
int f() {
  int x = src();  // @ctl_scalar_assign_drop:D x
  int y = src();
  x = y;
  x = y;
  return x + 1;  // @ctl_scalar_assign_drop:U:drop:scalar_assign_control
}
