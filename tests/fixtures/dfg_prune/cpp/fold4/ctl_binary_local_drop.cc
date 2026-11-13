int src();
int f() {
  int y = src();
  int x = src();  // @ctl_binary_local_drop:D x
  x = y + 1;
  x = y + 2;
  return x + 1;  // @ctl_binary_local_drop:U:drop:operand_control
}
