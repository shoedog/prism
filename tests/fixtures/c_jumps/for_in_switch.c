int f(int x, int n) {
switch (n) {
case 1:
for (;;) {
x = 1;
break;
}
x = 2;
break;
default:
x = 3;
}
return x;
}
