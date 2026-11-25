import * as React from 'react';
export const View = React.forwardRef(function Inner(p: {value: string}, ref: unknown) {
  observe(p.value); // @SINK
  observe(ref);
  return null;
});
