export {};
namespace NSfc { export function fSfc(aSfc: number) { return aSfc; } }
namespace OuterSfc { export class NOtherSfc {} }
export function runSfc(xSfc: number) { return NSfc.fSfc(xSfc); }
