function go() {
 let s = "";
 let get = () => 0;
 for (let /*@decl*/i = 0; get = () => /*@read*/i, i < 2; /*@upd*/i++) { }
 return get();
}
console.log(go());
