function f(x, n) {
x = 0;
for (let i = 0; i < n; i++) {
switch (n) {
case 1:
x = 1;
break;
case 2:
x = 2;
continue;
default:
x = 3;
}
}
do {
x = 4;
} while (n);
return x;
}
