int f(int x, int n) {
switch (n) {
case 1:
x = 1; // def
break;
case 2:
x = 2;
break;
default:
x = 3;
}
return x; // use
}
