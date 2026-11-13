int src();
int* pa();
int* pb();
int f() {
  int* p = pa();  // @ctl_ptr_assign_drop:D p
  int* q = pb();
  p = q;
  p = q;
  return *p + 1;  // @ctl_ptr_assign_drop:U:drop:pointer_assign_control
}
