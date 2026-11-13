int src();
int f() {
  char16_t x = src();  // @char16_t_scalar_keep:D x
  x = 0;
  return x + 1;  // @char16_t_scalar_keep:U:keep:fold3
}
