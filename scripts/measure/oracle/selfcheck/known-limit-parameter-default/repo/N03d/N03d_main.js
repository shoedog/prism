class /*@old*/K { who() { return "old"; } }
class Host { go(/** @type {() => string} */ g = () => new /*@read*/K().who()) {
 class /*@fresh*/K { who() { return "fresh"; } }
 return g();
} }
console.log(new Host().go());
