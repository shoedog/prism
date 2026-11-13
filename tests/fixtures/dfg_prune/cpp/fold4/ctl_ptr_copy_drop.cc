int* pa();
int* pb();
int f() {
  int* q = pb();
  int* p = pa();  // @ctl_ptr_copy_drop:D p
  p = q;
  p = q;
  return *p + 1;  // @ctl_ptr_copy_drop:U:drop:operand_control
}
