int *held;
int saved;
struct Box { Box(int& v) { saved = v; held = &v; } };
int f() {
  int x = 7; // DEF  // @cstyle_class_conversion:D x
  Box b = (Box)x;
  x = 0;
  *held = saved;
  return x; // USE  // @cstyle_class_conversion:U:keep:fold2_enum
}