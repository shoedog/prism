int* keep; int gv;
void f() {
  int x = 1; //D  // @f5s_cmt_addr_decl:D x
  int sv = x;
  int* p = &(/*c*/ x);
  x = 2;
  *p = sv;
  int z = x; //U  // @f5s_cmt_addr_decl:U:keep:enum3_sonnet_area3
  (void)z;
}
