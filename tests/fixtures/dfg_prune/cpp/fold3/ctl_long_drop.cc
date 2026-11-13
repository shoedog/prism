int src();
int f() {
  long x = src();  // @ctl_long_drop:D x
  x = 0;
  return x + 1;  // @ctl_long_drop:U:drop:keyword_scalar_control
}
