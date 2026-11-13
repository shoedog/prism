int src();
int f() {
  int x = src();  // @ctl_int_drop:D x
  x = 0;
  return x + 1;  // @ctl_int_drop:U:drop:keyword_scalar_control
}
