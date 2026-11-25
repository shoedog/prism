import { memo, forwardRef } from 'react';
export const View = forwardRef((p: {value: string}, ref: unknown) => {
  observe(p.value); // @SINK
  return null;
}, ...([] as []));
