int src();
int f() {
  int a[10];
  int i = src();  // @ctl_array_subscript_drop_i:D i
  int x = src();  // @ctl_array_subscript_drop_x:D x
  i = 3;
  x = 0;
  a[i] = 0;  // @ctl_array_subscript_drop_i:U:drop:operand_control
  x = a[i];
  x = a[i];
  return x + 1;  // @ctl_array_subscript_drop_x:U:drop:operand_control
}
