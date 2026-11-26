const /*@def*/key = 'slot';
class Base { [/*@live*/key] = 1; }
class Derived extends Base { declare [/*@erased*/key]: number; }
console.log(new Derived().slot);
