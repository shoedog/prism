int src();
int f() {
  char32_t x = src();  // @char32_t_scalar_keep:D x
  x = 0;
  return x + 1;  // @char32_t_scalar_keep:U:keep:fold3
}
