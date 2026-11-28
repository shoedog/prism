function go(){
function /*@old*/f(/*@a*/a: number) { return "old:" + a; }
{
function /*@fresh*/f(/*@b*/b: number) { return "fresh:" + b; }
}
return /*@site*/ /*@read*/f(/*@actual*/7);
}
console.log(go());
