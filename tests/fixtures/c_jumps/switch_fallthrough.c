int f(int x, int n) {
switch (n) {
case 1:
x = 1; // def
case 2:
use(x); // use
break;
}
return x;
}
