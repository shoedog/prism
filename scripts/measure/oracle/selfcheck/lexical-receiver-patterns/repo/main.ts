class A { m = 0; }
const a = new A();
const b = new A();
a./*@write*/m = 1;
const { /*@bindingOther*/m: other } = b;
const { /*@bindingSame*/m: same } = a;
let assignedOther = -1;
let assignedSame = -1;
({ /*@assignmentOther*/m: assignedOther } = b);
({ /*@assignmentSame*/m: assignedSame } = a);
console.log(other, same, assignedOther, assignedSame);
