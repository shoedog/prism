import React from "react";
export const Island = React.forwardRef<HTMLDivElement, { pad: number }>(({ pad }, ref) => (
  <div data-pad={pad} ref={ref} />
));
export const PlainRef = ({ pad }: { pad: number }, ref: unknown) => <div data-pad={pad} data-ref={ref} />;
