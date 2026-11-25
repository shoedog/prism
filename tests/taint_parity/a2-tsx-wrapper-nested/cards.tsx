import { memo, forwardRef } from 'react';
export const View = memo(forwardRef((p: {value: string}, ref: unknown) => {
  observe(p.value); // @SINK
  return null;
}));
