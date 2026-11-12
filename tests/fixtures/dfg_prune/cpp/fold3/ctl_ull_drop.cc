int src();
int f() {
  unsigned long long x = src();  // @ctl_ull_drop:D x
  x = 0;
  return x + 1;  // @ctl_ull_drop:U:drop:keyword_scalar_control
}
