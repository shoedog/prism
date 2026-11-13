int *held;
int saved;
void hold(int& v) { saved = v; held = &v; }
template<class... T>
int folded(int x, T... ts) {  // @fold_lvalue_ref:D x
  hold((ts, ..., x));
  x = 0;
  *held = saved;
  return x; // USE  // @fold_lvalue_ref:U:keep:fold2_enum
}
int main() { return folded<>(7); }