int source(void); int ready(void); void sink(int);
void test(int kind) {
 int x=0,y=0;
 if(kind==1) { if((x=source())==-1) return;
} else if(kind==2) {
 sink(x);
}
}
