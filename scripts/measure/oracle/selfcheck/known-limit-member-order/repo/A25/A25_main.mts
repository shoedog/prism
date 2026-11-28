const key: string = "f";
const o = {
 /*@old*/f: "data",
 get [key]() { return "getter"; }
};
console.log(o./*@read*/f);
