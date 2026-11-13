int f();
int g() {
  int x = f();  // @ctl_call_init_drop:D x
  x = 1;
  return x + 1;  // @ctl_call_init_drop:U:drop:operand_control
}
