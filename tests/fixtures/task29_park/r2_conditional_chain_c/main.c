int source(void); int ready(void); void sink(int);
void test(int kind) {
 int x=0,y=0;
 kind ? (x=source()) :
 (sink(x), 0);
sink(x);
}
