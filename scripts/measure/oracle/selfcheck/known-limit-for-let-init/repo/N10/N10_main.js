function go() {
 let s = "";
 for (let /*@decl*/i = 0, get = () => /*@read*/i; /*@test*/i < 2; /*@upd*/i++) { s += get(); }
 return s;
}
console.log(go());
