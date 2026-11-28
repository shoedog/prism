function go() {
 let s = "";
 for (let /*@decl*/i = 0, set = (/** @type {number} */ v) => { /*@w*/i = v; }; /*@test*/i < 3; i++) { set(10); s += /*@read*/i; }
 return s;
}
console.log(go());
