function go() {
 let s = "";
 for (let /*@decl*/i = 0, get = () => /*@read*/i; i < 3; i++) { /*@w*/i = 5; s += get(); }
 return s;
}
console.log(go());
