int b;
void save(void);
void restore(void);
int (*f(int a))(int b) {
  b = 1;                           // @f6_fpc:D b
  save();
  b = 2;
  restore();
  if (b) return 0;                 // @f6_fpc:U:keep:control_outer_declarator_without_comment
  return 0;
}
