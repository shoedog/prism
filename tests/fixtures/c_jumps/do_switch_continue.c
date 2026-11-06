int f(int x, int n) {
x = 0;
do {
switch (n) {
case 1:
x = 1;
continue;
default:
x = 2;
break;
}
x = 3;
} while (n);
return x;
}
