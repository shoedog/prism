import { memo } from 'react';
export const View = memo((p: {value: string}) => {
  observe(p.value); // @SINK
  return null;
});
