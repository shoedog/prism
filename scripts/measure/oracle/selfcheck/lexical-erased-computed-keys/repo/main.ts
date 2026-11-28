const /*@def*/key = 'slot';
interface Getter { get [/*@interfaceGetter*/key](): number }
type Setter = { set [/*@literalSetter*/key](v: number) };
abstract class Abstract { abstract [/*@abstract*/key](): void; }
class Overload {
  [/*@signature*/key](): void;
  [/*@implementation*/key]() {}
}
const live = { get [/*@liveGetter*/key]() { return 1; }, set [/*@liveSetter*/key](v: number) {} };
class Concrete extends Abstract { [/*@concrete*/key]() {} }
console.log(live.slot, typeof new Overload().slot, typeof new Concrete().slot);
