int* src();
int g(int *q) {
  int *p = src();                  // @f6_refpc:D p
  int *(&rp) = p;
  int *t = rp;
  p = q;
  rp = t;
  return *p;                       // @f6_refpc:U:keep:control_pointer_reference_without_comment
}
