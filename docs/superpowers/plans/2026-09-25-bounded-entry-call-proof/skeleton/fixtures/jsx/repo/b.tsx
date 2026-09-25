import React from "react";
import { Island, PlainRef } from "./a";
export function User(r: unknown) {
  const direct = PlainRef({ pad: 1 }, r);
  return <div><Island pad={1} ref={r} /><PlainRef pad={2} />{direct}</div>;
}
export class Holder extends React.Component {
  zoom(r: unknown) {
    this.setState((state: unknown) => ({ ...PlainRef({ pad: 3 }, state) }));
  }
}
const wrap = <T,>(f: T) => f;
export class Gesture {
  private onChange = wrap((event: unknown) => {
    return PlainRef({ pad: 4 }, event);
  });
}
