struct S { S& operator=(int& r); };
int* kept;
int src();
void sink(int);
namespace N { S y; }
void f(bool c) {
  int x = src();  // @cw_using_decl:D x
  int y = 0;
  {
    using N::y;
    y = c ? x : x;
  }
  int saved = *kept;
  x = 0;
  *kept = saved;
  int z = x + 1;  // @cw_using_decl:U:keep:fold3
  sink(z + y);
}
