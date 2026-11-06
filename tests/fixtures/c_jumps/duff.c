int f(int x, int n) {
switch (n & 1) {
case 0: do {
x = 1;
case 1: x = 2;
} while (--n);
}
return x;
}
