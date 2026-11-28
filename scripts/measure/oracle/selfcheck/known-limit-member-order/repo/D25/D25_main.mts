const key: string = "slot";
const o = { /*@orig*/slot: "data", set [key](v: string) { console.log(v); } };
console.log(typeof o./*@read*/slot);
