int f(int x, int n) {
x = 0;
switch (n) {
case 1:
x = 1; // def
break; // jump
x = 2; // bypassed kill

default:
x = 3;
}
return x; // use
}
