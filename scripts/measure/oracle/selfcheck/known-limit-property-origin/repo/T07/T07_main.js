const object = { /*@own*/__proto__(/** @type {number} */ /*@p*/p){ return "own:" + p; } };
console.log(/*@site*/object./*@read*/__proto__(/*@actual*/7), "__proto__" in object);
