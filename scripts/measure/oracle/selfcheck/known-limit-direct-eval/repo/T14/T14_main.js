/** @param {number} a */
function /*@old*/f(/*@a*/a){return "old:" + a;}
function go(raw = /*@defaultSite*/ /*@defaultRead*/f(/*@defaultActual*/9)) {
/** @param {number} b */
function /*@fresh*/alt(/*@b*/b){return "fresh:" + b;}
eval("var f = alt;");
return raw + "|" + /*@site*/ /*@read*/f(/*@actual*/7);
}
console.log(go());
