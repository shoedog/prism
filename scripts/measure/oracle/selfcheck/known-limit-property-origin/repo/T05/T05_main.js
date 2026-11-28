const original = {get /*@def*/q(){ return {}; }};
const copy = { ...original };
const second = { ...copy };
console.log(original.q === second./*@read*/q, copy.q === second.q);
