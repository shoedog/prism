int source(void); int ready(void); void sink(int);
void test(int kind) {
 int x=0,y=0;
 if(kind==1)
 x=source();
else
 sink(x);
}
