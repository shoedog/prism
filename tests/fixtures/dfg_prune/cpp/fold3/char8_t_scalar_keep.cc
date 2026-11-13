int src();
int f() {
  char8_t x = src();  // @char8_t_scalar_keep:D x
  x = 0;
  return x + 1;  // @char8_t_scalar_keep:U:keep:fold3
}
