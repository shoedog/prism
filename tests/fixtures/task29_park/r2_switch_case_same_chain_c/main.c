int source(void); void sink(int);
void test(int kind) {
 int x=0,y=0;
 switch(kind) {
 case 1: x=source(); break;
 default: x=2;
 }
 sink(x);
}
