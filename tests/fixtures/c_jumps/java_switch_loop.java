class A {
int f(int x, int n) {
x = 0;
while (n > 0) {
switch (n) {
case 1:
x = 1;
break;
default:
x = 3;
}
if (x > 1) { continue; }
}
do {
x = 4;
} while (n > 0);
return x;
}
}
