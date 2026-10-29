struct T { T(int v) {} }; void sink(int);
void test() {
 int x=1,y=0;
 { T a(x); }
 sink(x);
}
