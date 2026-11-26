const /*@def*/key = 'slot';
declare class Ambient { [/*@erased*/key]: number }
class Evaluated { [/*@live*/key] = 1; }
console.log(typeof Ambient, new Evaluated().slot);
