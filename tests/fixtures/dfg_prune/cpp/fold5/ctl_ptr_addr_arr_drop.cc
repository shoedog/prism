int* pb();
int f() {
  int arr[3] = {0, 0, 0};
  int* q = pb();
  int* p = &arr[0];  // @ctl_ptr_addr_arr_drop:D p
  p = q;
  p = q;
  return *p + 1;  // @ctl_ptr_addr_arr_drop:U:drop:operand_control_fold5
}
