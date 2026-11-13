int src();
int f() {
  int n = src();  // @ctl_self_mul_drop:D n
  n = 1;
  n = n * 2;
  return n + 1;  // @ctl_self_mul_drop:U:drop:operand_control
}
