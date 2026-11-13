int addressof_builtin_cpp(int c) {
  int x = c;                       // @f7_aof:D x
  int *p = __builtin_addressof(x);
  int t = *p;
  x = 0;
  *p = t;
  return x;                        // @f7_aof:U:keep:builtin_addressof_takes_an_address_without_amp
}
