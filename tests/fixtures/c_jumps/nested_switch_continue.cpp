int f(int x, int n) {
while (n) {
switch (x) {
case 1:
x = 1; // def
continue;
default:
break;
}
x = 2;
}
return x;
}
