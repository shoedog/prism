int f(int x, int n) {
x = 0;
do {
x = 1; // def
break; // jump
x = 2; // bypassed kill
} while (n);
return x; // use
}
